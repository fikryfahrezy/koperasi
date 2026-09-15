//! Application use cases for savings.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, PaymentInput, SavingsTransactionInput},
    domain::timestamp_id,
};

pub(crate) async fn post_savings_transaction(
    input: SavingsTransactionInput,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.amount <= 0 {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }
    if !matches!(input.account_type.as_str(), "POKOK" | "WAJIB" | "MANASUKA") {
        return Err("Jenis simpanan tidak valid.".into());
    }
    if !matches!(input.movement.as_str(), "Setoran" | "Penarikan") {
        return Err("Jenis mutasi tidak valid.".into());
    }
    if input.movement == "Penarikan" && input.account_type != "MANASUKA" {
        return Err("Hanya simpanan manasuka yang dapat ditarik pada versi ini.".into());
    }
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
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Anggota aktif tidak ditemukan.")?;
    let account = sqlx::query(
        "SELECT id, balance FROM savings_accounts WHERE member_id = ? AND account_type = ?",
    )
    .bind(&input.member_id)
    .bind(&input.account_type)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    let account_id: String = account.get("id");
    let balance: i64 = account.get("balance");
    if input.movement == "Penarikan" && balance < input.amount {
        return Err("Saldo manasuka tidak mencukupi.".into());
    }

    let id = timestamp_id(if input.movement == "Setoran" {
        "SAV-IN"
    } else {
        "SAV-OUT"
    });
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/SAV/{id}")
    } else {
        input.reference.trim().to_string()
    };
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE reference = ?")
            .bind(&reference)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }
    let is_deposit = input.movement == "Setoran";
    let direction = if is_deposit { "Masuk" } else { "Keluar" };
    let transaction_type = if is_deposit {
        "SAVINGS_DEPOSIT"
    } else {
        "SAVINGS_WITHDRAWAL"
    };
    let component_type = transaction_type;
    let description = format!(
        "{} simpanan {}",
        input.movement,
        input.account_type.to_lowercase()
    );
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&input.member_id)
        .bind(&member_name)
        .bind(transaction_type)
        .bind(&description)
        .bind(&reference)
        .bind(direction)
        .bind(input.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(component_type)
        .bind(&description)
        .bind(input.amount)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let delta = if is_deposit {
        input.amount
    } else {
        -input.amount
    };
    sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
        .bind(delta)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(direction)
        .bind(input.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(&id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": input.movement, "accountType": input.account_type, "amount": input.amount, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}

pub(crate) async fn post_payment(
    input: PaymentInput,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if [
        input.principal,
        input.interest,
        input.wajib,
        input.voluntary,
    ]
    .iter()
    .any(|value| *value < 0)
    {
        return Err("Komponen pembayaran tidak boleh negatif.".into());
    }
    let amount = input.principal + input.interest + input.wajib + input.voluntary;
    if amount <= 0 {
        return Err("Total pembayaran harus lebih dari nol.".into());
    }
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
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Anggota aktif tidak ditemukan.")?;
    let id = timestamp_id("TRX");
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/RCPT/{id}")
    } else {
        input.reference.trim().to_string()
    };
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE reference = ?")
            .bind(&reference)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }

    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'MEMBER_PAYMENT', 'Pembayaran anggota', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(&input.member_id).bind(&member_name).bind(&reference).bind(amount).execute(&mut *db).await.map_err(|error| error.to_string())?;

    let mut remaining_principal = input.principal;
    let loan_rows = sqlx::query("SELECT id, balance FROM loans WHERE member_id = ? AND status = 'Berjalan' AND balance > 0 ORDER BY realization_date, id")
        .bind(&input.member_id).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for loan in loan_rows {
        if remaining_principal == 0 {
            break;
        }
        let loan_id: String = loan.get("id");
        let balance: i64 = loan.get("balance");
        let allocated = remaining_principal.min(balance);
        sqlx::query("UPDATE loans SET balance = balance - ?, status = CASE WHEN balance - ? = 0 THEN 'Lunas' ELSE status END WHERE id = ?")
            .bind(allocated).bind(allocated).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_PRINCIPAL', 'Pokok pinjaman', ?, ?)")
            .bind(&id).bind(allocated).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        remaining_principal -= allocated;
    }
    if remaining_principal > 0 {
        return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
    }

    if input.interest > 0 {
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount) VALUES (?, 'LOAN_INTEREST', 'Bunga pinjaman', ?)").bind(&id).bind(input.interest).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    for (kind, component_type, label, value) in [
        ("WAJIB", "SAVINGS_WAJIB", "Simpanan wajib", input.wajib),
        (
            "MANASUKA",
            "SAVINGS_VOLUNTARY",
            "Simpanan manasuka",
            input.voluntary,
        ),
    ] {
        if value > 0 {
            let account_id: String = sqlx::query_scalar(
                "SELECT id FROM savings_accounts WHERE member_id = ? AND account_type = ?",
            )
            .bind(&input.member_id)
            .bind(kind)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
                .bind(value)
                .bind(&account_id)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?)").bind(&id).bind(component_type).bind(label).bind(value).bind(account_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        }
    }
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
    )
    .bind(&id)
    .bind(amount)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')").bind(&id).bind(serde_json::to_string(&serde_json::json!({"amount": amount, "reference": reference})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}
