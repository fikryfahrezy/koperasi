//! Application use cases for members.

use sqlx::SqlitePool;

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AddMemberInput, AppSnapshot},
    domain::{
        business_period, timestamp_id, MemberStatus, PeriodStatus, SavingsAccountStatus,
        SavingsAccountType, TransactionDirection, TransactionStatus,
    },
};

pub(crate) async fn add_member(
    input: AddMemberInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.name.trim().is_empty() || input.member_number.trim().is_empty() {
        return Err("Nama dan nomor anggota wajib diisi.".into());
    }
    if input.principal_savings < 50_000 {
        return Err("Simpanan pokok minimal Rp50.000.".into());
    }
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
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM members WHERE company_id = ? AND UPPER(member_number) = UPPER(?)",
    )
    .bind(company_id)
    .bind(input.member_number.trim())
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor anggota sudah digunakan.".into());
    }
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(id, INSTR(id, '-M') + 2) AS INTEGER)), 0) + 1 FROM members WHERE company_id = ?",
    )
        .bind(company_id)
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let id = format!("{company_id}-M{next:03}");
    sqlx::query("INSERT INTO members (id, company_id, member_number, name, joined_at, status) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(company_id).bind(input.member_number.trim()).bind(input.name.trim()).bind(input.joined_at.trim()).bind(MemberStatus::Active.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for kind in [
        SavingsAccountType::Principal,
        SavingsAccountType::Mandatory,
        SavingsAccountType::Voluntary,
    ] {
        sqlx::query("INSERT INTO savings_accounts (id, company_id, member_id, account_type, balance, status) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(format!("SA-{id}-{}", kind.as_str())).bind(company_id).bind(&id).bind(kind.as_str()).bind(0_i64).bind(SavingsAccountStatus::Active.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    let transaction_id = timestamp_id("SAV-IN");
    let account_id = format!("SA-{id}-{}", SavingsAccountType::Principal.as_str());
    let reference = format!("KBS/SAV/{transaction_id}");
    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, ?, ?, ?, 'Aplikasi lokal')")
        .bind(&transaction_id)
        .bind(company_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&id)
        .bind(input.name.trim())
        .bind(&reference)
        .bind(TransactionDirection::In.as_str())
        .bind(input.principal_savings)
        .bind(TransactionStatus::Posted.as_str())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, ?)")
        .bind(company_id)
        .bind(&transaction_id)
        .bind(input.principal_savings)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE savings_accounts SET balance = ? WHERE company_id = ? AND id = ?")
        .bind(input.principal_savings)
        .bind(company_id)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)",
    )
    .bind(company_id)
    .bind(&transaction_id)
    .bind(TransactionDirection::In.as_str())
    .bind(input.principal_savings)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(company_id)
        .bind(&transaction_id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": crate::domain::SavingsMovement::Deposit.as_str(), "accountType": SavingsAccountType::Principal.as_str(), "amount": input.principal_savings, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'MEMBER', ?, 'CREATED', ?, 'Aplikasi lokal')")
        .bind(company_id).bind(&id).bind(serde_json::to_string(&serde_json::json!({"name": input.name, "memberNumber": input.member_number, "principalSavings": input.principal_savings})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
