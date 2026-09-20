//! Application use cases for transactions.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, ReverseTransactionInput},
    domain::{
        business_period, timestamp_id, LoanStatus, PeriodStatus, TransactionDirection,
        TransactionStatus,
    },
};

pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let period = business_period(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM periods WHERE company_id = ? AND period = ? AND status = ?",
    )
    .bind(company_id)
    .bind(period)
    .bind(PeriodStatus::Locked.as_str())
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let id = &input.id;
    let original = sqlx::query("SELECT business_date, display_date, display_time, member_id, member_name, description, reference, direction, amount, status FROM transactions WHERE company_id = ? AND id = ?")
        .bind(company_id).bind(id).fetch_optional(&mut *db).await.map_err(|error| error.to_string())?.ok_or("Transaksi tidak ditemukan.")?;
    let status: String = original
        .try_get("status")
        .map_err(|error| error.to_string())?;
    if TransactionStatus::try_from(status.as_str())? != TransactionStatus::Posted {
        return Err("Hanya transaksi terposting yang dapat dibalik.".into());
    }
    let direction: String = original
        .try_get("direction")
        .map_err(|error| error.to_string())?;
    let reversal_direction = TransactionDirection::try_from(direction.as_str())?.reversed();
    let reference: String = original
        .try_get("reference")
        .map_err(|error| error.to_string())?;
    let amount: i64 = original
        .try_get("amount")
        .map_err(|error| error.to_string())?;
    let member_id: Option<String> = original
        .try_get("member_id")
        .map_err(|error| error.to_string())?;
    let member_name: String = original
        .try_get("member_name")
        .map_err(|error| error.to_string())?;
    let description: String = original
        .try_get("description")
        .map_err(|error| error.to_string())?;
    let components = sqlx::query("SELECT component_type, label, amount, loan_id, savings_account_id FROM transaction_components WHERE company_id = ? AND transaction_id = ?")
        .bind(company_id).bind(id).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for component in &components {
        let component_type: String = component
            .try_get("component_type")
            .map_err(|error| error.to_string())?;
        let value: i64 = component
            .try_get("amount")
            .map_err(|error| error.to_string())?;
        if component_type == "LOAN_PRINCIPAL" {
            let loan_id: Option<String> = component
                .try_get("loan_id")
                .map_err(|error| error.to_string())?;
            let loan_id = loan_id.ok_or("Komponen pembayaran tidak memiliki pinjaman terkait.")?;
            let result = sqlx::query(
                "UPDATE loans SET balance = balance + ?, status = ? WHERE company_id = ? AND id = ? AND balance <= ?",
            )
                .bind(value)
                .bind(LoanStatus::Active.as_str())
                .bind(company_id)
                .bind(loan_id)
                .bind(i64::MAX - value)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            if result.rows_affected() != 1 {
                return Err("Pinjaman terkait komponen pembayaran tidak ditemukan.".into());
            }
        } else if component_type == "LOAN_DISBURSEMENT" {
            let loan_id: Option<String> = component
                .try_get("loan_id")
                .map_err(|error| error.to_string())?;
            let loan_id = loan_id.ok_or("Komponen pencairan tidak memiliki pinjaman terkait.")?;
            let result = sqlx::query("UPDATE loans SET balance = balance - ?, status = ?, realization_date = '-', due_date = '-' WHERE company_id = ? AND id = ? AND balance = ?")
                .bind(value)
                .bind(LoanStatus::Draft.as_str())
                .bind(company_id)
                .bind(loan_id)
                .bind(value)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            if result.rows_affected() != 1 {
                return Err(
                    "Pencairan tidak dapat dibalik setelah pinjaman memiliki pembayaran.".into(),
                );
            }
        } else if component_type.starts_with("SAVINGS_") {
            let account_id: Option<String> = component
                .try_get("savings_account_id")
                .map_err(|error| error.to_string())?;
            let account_id =
                account_id.ok_or("Komponen simpanan tidak memiliki rekening terkait.")?;
            let result = if component_type == "SAVINGS_WITHDRAWAL" {
                sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE company_id = ? AND id = ? AND balance <= ?")
                    .bind(value)
                    .bind(company_id)
                    .bind(account_id)
                    .bind(i64::MAX - value)
                    .execute(&mut *db)
                    .await
                    .map_err(|error| error.to_string())?
            } else {
                sqlx::query("UPDATE savings_accounts SET balance = balance - ? WHERE company_id = ? AND id = ? AND balance >= ?")
                    .bind(value)
                    .bind(company_id)
                    .bind(account_id)
                    .bind(value)
                    .execute(&mut *db)
                    .await
                    .map_err(|error| error.to_string())?
            };
            if result.rows_affected() != 1 {
                return Err("Reversal ditolak karena saldo sub-ledger tidak mencukupi.".into());
            }
        }
    }
    sqlx::query("UPDATE transactions SET status = ? WHERE company_id = ? AND id = ?")
        .bind(TransactionStatus::Reversed.as_str())
        .bind(company_id)
        .bind(id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let reversal_id = timestamp_id("REV");
    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, reversed_transaction_id, actor) VALUES (?, ?, ?, ?, ?, ?, ?, 'REVERSAL', ?, ?, ?, ?, ?, ?, 'Aplikasi lokal')")
        .bind(&reversal_id).bind(company_id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(member_id).bind(member_name).bind(format!("Reversal · {description}")).bind(format!("{reference}/REV/{}", timestamp_id("R"))).bind(reversal_direction.as_str()).bind(amount).bind(TransactionStatus::Posted.as_str()).bind(id).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for component in components {
        let label: String = component
            .try_get("label")
            .map_err(|error| error.to_string())?;
        let component_amount: i64 = component
            .try_get("amount")
            .map_err(|error| error.to_string())?;
        let loan_id: Option<String> = component
            .try_get("loan_id")
            .map_err(|error| error.to_string())?;
        let savings_account_id: Option<String> = component
            .try_get("savings_account_id")
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id) VALUES (?, ?, 'REVERSAL', ?, ?, ?, ?)")
            .bind(company_id).bind(&reversal_id).bind(format!("Reversal · {label}")).bind(component_amount).bind(loan_id).bind(savings_account_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)")
        .bind(company_id)
        .bind(&reversal_id)
        .bind(reversal_direction.as_str())
        .bind(amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor) VALUES (?, 'TRANSACTION', ?, 'REVERSED', ?, ?, 'Aplikasi lokal')")
        .bind(company_id).bind(id).bind(serde_json::to_string(&serde_json::json!({"status": TransactionStatus::Posted.as_str()})).unwrap_or_default()).bind(serde_json::to_string(&serde_json::json!({"status": TransactionStatus::Reversed.as_str(), "reversalId": reversal_id})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
