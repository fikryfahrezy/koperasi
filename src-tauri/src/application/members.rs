//! Application use cases for members.

use sqlx::SqlitePool;

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AddMemberInput, AppSnapshot},
    domain::timestamp_id,
};

pub(crate) async fn add_member(
    input: AddMemberInput,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.name.trim().is_empty() || input.member_number.trim().is_empty() {
        return Err("Nama dan nomor anggota wajib diisi.".into());
    }
    if input.principal_savings < 50_000 {
        return Err("Simpanan pokok minimal Rp50.000.".into());
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE UPPER(member_number) = UPPER(?)")
            .bind(input.member_number.trim())
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor anggota sudah digunakan.".into());
    }
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(id, 2) AS INTEGER)), 0) + 1 FROM members WHERE id GLOB 'M[0-9]*'",
    )
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let id = format!("M{next:03}");
    sqlx::query("INSERT INTO members (id, member_number, name, joined_at, status) VALUES (?, ?, ?, ?, 'Aktif')")
        .bind(&id).bind(input.member_number.trim()).bind(input.name.trim()).bind(input.joined_at.trim()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for kind in ["POKOK", "WAJIB", "MANASUKA"] {
        sqlx::query("INSERT INTO savings_accounts (id, member_id, account_type, balance) VALUES (?, ?, ?, ?)")
            .bind(format!("SA-{id}-{kind}")).bind(&id).bind(kind).bind(0_i64).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    let transaction_id = timestamp_id("SAV-IN");
    let account_id = format!("SA-{id}-POKOK");
    let reference = format!("KBS/SAV/{transaction_id}");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&transaction_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&id)
        .bind(input.name.trim())
        .bind(&reference)
        .bind(input.principal_savings)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, ?)")
        .bind(&transaction_id)
        .bind(input.principal_savings)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE savings_accounts SET balance = ? WHERE id = ?")
        .bind(input.principal_savings)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
    )
    .bind(&transaction_id)
    .bind(input.principal_savings)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(&transaction_id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": "Setoran", "accountType": "POKOK", "amount": input.principal_savings, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('MEMBER', ?, 'CREATED', ?, 'Aplikasi lokal')")
        .bind(&id).bind(serde_json::to_string(&serde_json::json!({"name": input.name, "memberNumber": input.member_number, "principalSavings": input.principal_savings})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}
