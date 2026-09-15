//! Application use cases for loans.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, CreateLoanInput, DisburseLoanInput, LoanPreview},
    domain::{add_months, calculate_loan, timestamp_id},
};

pub(crate) async fn preview_loan(
    input: CreateLoanInput,
    pool: &SqlitePool,
) -> Result<LoanPreview, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Plafond dan tenor harus lebih dari nol.".into());
    }
    let annual_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(pool).await.map_err(|error| error.to_string())?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(pool).await.map_err(|error| error.to_string())?;
    let calculation = calculate_loan(input.plafond, input.tenor, annual_rate, provision_rate);
    Ok(LoanPreview {
        principal_installment: calculation.principal_installment,
        first_interest: calculation.first_interest,
        provision: calculation.provision,
        first_total: calculation.first_total,
        annual_rate: calculation.annual_rate,
    })
}

pub(crate) async fn create_loan(
    input: CreateLoanInput,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.plafond <= 0
        || input.tenor <= 0
        || !matches!(input.interest_type.as_str(), "Menurun" | "Flat")
    {
        return Err("Data pinjaman tidak valid.".into());
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Anggota aktif tidak ditemukan.")?;
    let rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(&mut *db).await.map_err(|error| error.to_string())?;
    let id = timestamp_id("LOAN");
    sqlx::query("INSERT INTO loans (id, member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES (?, ?, ?, ?, 0, ?, ?, ?, '-', '-', 'Draf')")
        .bind(&id).bind(&input.member_id).bind(&member_name).bind(input.plafond).bind(rate * 100.0).bind(input.tenor).bind(&input.interest_type).execute(&mut *db).await.map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('LOAN', ?, 'DRAFT_CREATED', ?, 'Aplikasi lokal')")
        .bind(&id).bind(serde_json::to_string(&serde_json::json!({"memberId": input.member_id, "plafond": input.plafond, "tenor": input.tenor, "interestType": input.interest_type})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}

pub(crate) async fn disburse_loan(
    input: DisburseLoanInput,
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

    let loan = sqlx::query(
        "SELECT member_id, member_name, plafond, tenor, status, balance FROM loans WHERE id = ?",
    )
    .bind(&input.loan_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Pinjaman tidak ditemukan.")?;
    let status: String = loan.get("status");
    let balance: i64 = loan.get("balance");
    if status != "Draf" || balance != 0 {
        return Err("Hanya pinjaman draf dengan saldo nol yang dapat dicairkan.".into());
    }
    let member_id: Option<String> = loan.get("member_id");
    let member_name: String = loan.get("member_name");
    let plafond: i64 = loan.get("plafond");
    let tenor: i64 = loan.get("tenor");
    let due_date = add_months(&input.business_date, tenor)?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= ? ORDER BY effective_date DESC LIMIT 1")
        .bind(&input.business_date)
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let provision = (plafond as f64 * provision_rate).round() as i64;

    let disbursement_id = timestamp_id("DISB");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'LOAN_DISBURSEMENT', 'Pencairan pinjaman', ?, 'Keluar', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&disbursement_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&member_id)
        .bind(&member_name)
        .bind(format!("KBS/DISB/{}/{}", input.loan_id, disbursement_id))
        .bind(plafond)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_DISBURSEMENT', 'Pencairan pokok pinjaman', ?, ?)")
        .bind(&disbursement_id)
        .bind(plafond)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Keluar', ?)",
    )
    .bind(&disbursement_id)
    .bind(plafond)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    if provision > 0 {
        let provision_id = timestamp_id("PROV");
        sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'LOAN_PROVISION', 'Provisi pencairan pinjaman', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
            .bind(&provision_id)
            .bind(&input.business_date)
            .bind(&input.display_date)
            .bind(&input.display_time)
            .bind(&member_id)
            .bind(&member_name)
            .bind(format!("KBS/PROV/{}/{}", input.loan_id, provision_id))
            .bind(provision)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_PROVISION', 'Pendapatan provisi', ?, ?)")
            .bind(&provision_id)
            .bind(provision)
            .bind(&input.loan_id)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
        )
        .bind(&provision_id)
        .bind(provision)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    sqlx::query("UPDATE loans SET balance = plafond, realization_date = ?, due_date = ?, status = 'Berjalan' WHERE id = ?")
        .bind(&input.business_date)
        .bind(&due_date)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('LOAN', ?, 'DISBURSED', ?, 'Aplikasi lokal')")
        .bind(&input.loan_id)
        .bind(serde_json::to_string(&serde_json::json!({"plafond": plafond, "provision": provision, "dueDate": due_date})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(pool).await
}
