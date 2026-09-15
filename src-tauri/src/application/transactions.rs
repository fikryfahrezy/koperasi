//! Application use cases for transactions.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, ReverseTransactionInput},
    domain::timestamp_id,
};

pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let period = input
        .business_date
        .get(0..7)
        .ok_or("Tanggal bisnis tidak valid.")?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM periods WHERE period = ? AND status = 'LOCKED'")
            .bind(period)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let id = &input.id;
    let original = sqlx::query("SELECT business_date, display_date, display_time, member_id, member_name, description, reference, direction, amount, status FROM transactions WHERE id = ?")
        .bind(id).fetch_optional(&mut *db).await.map_err(|error| error.to_string())?.ok_or("Transaksi tidak ditemukan.")?;
    let status: String = original.get("status");
    if status != "Terposting" {
        return Err("Hanya transaksi terposting yang dapat dibalik.".into());
    }
    let components = sqlx::query("SELECT component_type, label, amount, loan_id, savings_account_id FROM transaction_components WHERE transaction_id = ?")
        .bind(id).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for component in &components {
        let component_type: String = component.get("component_type");
        let value: i64 = component.get("amount");
        if component_type == "LOAN_PRINCIPAL" {
            let loan_id: Option<String> = component.try_get("loan_id").ok();
            if let Some(loan_id) = loan_id {
                sqlx::query(
                    "UPDATE loans SET balance = balance + ?, status = 'Berjalan' WHERE id = ?",
                )
                .bind(value)
                .bind(loan_id)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            }
        } else if component_type == "LOAN_DISBURSEMENT" {
            let loan_id: Option<String> = component.try_get("loan_id").ok();
            if let Some(loan_id) = loan_id {
                let result = sqlx::query("UPDATE loans SET balance = balance - ?, status = 'Draf', realization_date = '-', due_date = '-' WHERE id = ? AND balance = ?")
                    .bind(value)
                    .bind(loan_id)
                    .bind(value)
                    .execute(&mut *db)
                    .await
                    .map_err(|error| error.to_string())?;
                if result.rows_affected() == 0 {
                    return Err(
                        "Pencairan tidak dapat dibalik setelah pinjaman memiliki pembayaran."
                            .into(),
                    );
                }
            }
        } else if component_type.starts_with("SAVINGS_") {
            let account_id: Option<String> = component.try_get("savings_account_id").ok();
            if let Some(account_id) = account_id {
                let result = if component_type == "SAVINGS_WITHDRAWAL" {
                    sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
                        .bind(value)
                        .bind(account_id)
                        .execute(&mut *db)
                        .await
                        .map_err(|error| error.to_string())?
                } else {
                    sqlx::query("UPDATE savings_accounts SET balance = balance - ? WHERE id = ? AND balance >= ?")
                        .bind(value)
                        .bind(account_id)
                        .bind(value)
                        .execute(&mut *db)
                        .await
                        .map_err(|error| error.to_string())?
                };
                if result.rows_affected() == 0 {
                    return Err("Reversal ditolak karena saldo sub-ledger tidak mencukupi.".into());
                }
            }
        }
    }
    sqlx::query("UPDATE transactions SET status = 'Dibalik' WHERE id = ?")
        .bind(id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let reversal_id = timestamp_id("REV");
    let direction: String = original.get("direction");
    let reversal_direction = if direction == "Masuk" {
        "Keluar"
    } else {
        "Masuk"
    };
    let reference: String = original.get("reference");
    let amount: i64 = original.get("amount");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, reversed_transaction_id, actor) VALUES (?, ?, ?, ?, ?, ?, 'REVERSAL', ?, ?, ?, ?, 'Terposting', ?, 'Aplikasi lokal')")
        .bind(&reversal_id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(original.get::<Option<String>, _>("member_id")).bind(original.get::<String, _>("member_name")).bind(format!("Reversal · {}", original.get::<String, _>("description"))).bind(format!("{reference}/REV/{}", timestamp_id("R"))).bind(reversal_direction).bind(amount).bind(id).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for component in components {
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id, savings_account_id) VALUES (?, 'REVERSAL', ?, ?, ?, ?)")
            .bind(&reversal_id).bind(format!("Reversal · {}", component.get::<String, _>("label"))).bind(component.get::<i64, _>("amount")).bind(component.get::<Option<String>, _>("loan_id")).bind(component.get::<Option<String>, _>("savings_account_id")).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, ?, ?)")
        .bind(&reversal_id)
        .bind(reversal_direction)
        .bind(amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, before_json, after_json, actor) VALUES ('TRANSACTION', ?, 'REVERSED', ?, ?, 'Aplikasi lokal')")
        .bind(id).bind(serde_json::to_string(&serde_json::json!({"status": "Terposting"})).unwrap_or_default()).bind(serde_json::to_string(&serde_json::json!({"status": "Dibalik", "reversalId": reversal_id})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}
