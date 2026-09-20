//! Application use cases for loans.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, CreateLoanInput, DisburseLoanInput, LoanPreview},
    domain::{
        add_months, business_period, calculate_loan, calculate_rate_amount, timestamp_id,
        InterestType, LoanStatus, MemberStatus, PeriodStatus, TransactionDirection,
        TransactionStatus,
    },
};

pub(crate) async fn preview_loan(
    input: CreateLoanInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<LoanPreview, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Plafond dan tenor harus lebih dari nol.".into());
    }
    InterestType::try_from(input.interest_type.as_str())?;
    let annual_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE company_id = ? AND parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .bind(company_id).fetch_one(pool).await.map_err(|error| error.to_string())?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE company_id = ? AND parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .bind(company_id).fetch_one(pool).await.map_err(|error| error.to_string())?;
    let calculation = calculate_loan(input.plafond, input.tenor, annual_rate, provision_rate)?;
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
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Data pinjaman tidak valid.".into());
    }
    let interest_type = InterestType::try_from(input.interest_type.as_str())?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name: String = sqlx::query_scalar(
        "SELECT name FROM members WHERE company_id = ? AND id = ? AND status = ?",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .bind(MemberStatus::Active.as_str())
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Anggota aktif tidak ditemukan.")?;
    let rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE company_id = ? AND parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .bind(company_id).fetch_one(&mut *db).await.map_err(|error| error.to_string())?;
    let rate_percent = rate * 100.0;
    if rate < 0.0 || !rate_percent.is_finite() {
        return Err("Suku bunga pinjaman tidak valid.".into());
    }
    let id = timestamp_id("LOAN");
    sqlx::query("INSERT INTO loans (id, company_id, member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, '-', '-', ?)")
        .bind(&id).bind(company_id).bind(&input.member_id).bind(&member_name).bind(input.plafond).bind(rate_percent).bind(input.tenor).bind(interest_type.as_str()).bind(LoanStatus::Draft.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'LOAN', ?, 'DRAFT_CREATED', ?, 'Aplikasi lokal')")
        .bind(company_id).bind(&id).bind(serde_json::to_string(&serde_json::json!({"memberId": input.member_id, "plafond": input.plafond, "tenor": input.tenor, "interestType": input.interest_type})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

pub(crate) async fn disburse_loan(
    input: DisburseLoanInput,
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

    let loan = sqlx::query(
        "SELECT member_id, member_name, plafond, tenor, status, balance FROM loans WHERE company_id = ? AND id = ?",
    )
    .bind(company_id)
    .bind(&input.loan_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Pinjaman tidak ditemukan.")?;
    let status: String = loan.try_get("status").map_err(|error| error.to_string())?;
    let balance: i64 = loan.try_get("balance").map_err(|error| error.to_string())?;
    if LoanStatus::try_from(status.as_str())? != LoanStatus::Draft || balance != 0 {
        return Err("Hanya pinjaman draf dengan saldo nol yang dapat dicairkan.".into());
    }
    let member_id: Option<String> = loan
        .try_get("member_id")
        .map_err(|error| error.to_string())?;
    let member_name: String = loan
        .try_get("member_name")
        .map_err(|error| error.to_string())?;
    let plafond: i64 = loan.try_get("plafond").map_err(|error| error.to_string())?;
    let tenor: i64 = loan.try_get("tenor").map_err(|error| error.to_string())?;
    let due_date = add_months(&input.business_date, tenor)?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE company_id = ? AND parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= ? ORDER BY effective_date DESC LIMIT 1")
        .bind(company_id).bind(&input.business_date)
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let provision = calculate_rate_amount(plafond, provision_rate)?;

    let disbursement_id = timestamp_id("DISB");
    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, 'LOAN_DISBURSEMENT', 'Pencairan pinjaman', ?, ?, ?, ?, 'Aplikasi lokal')")
        .bind(&disbursement_id)
        .bind(company_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&member_id)
        .bind(&member_name)
        .bind(format!("KBS/DISB/{}/{}", input.loan_id, disbursement_id))
        .bind(TransactionDirection::Out.as_str())
        .bind(plafond)
        .bind(TransactionStatus::Posted.as_str())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES (?, ?, 'LOAN_DISBURSEMENT', 'Pencairan pokok pinjaman', ?, ?)")
        .bind(company_id)
        .bind(&disbursement_id)
        .bind(plafond)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)",
    )
    .bind(company_id)
    .bind(&disbursement_id)
    .bind(TransactionDirection::Out.as_str())
    .bind(plafond)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    if provision > 0 {
        let provision_id = timestamp_id("PROV");
        sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, 'LOAN_PROVISION', 'Provisi pencairan pinjaman', ?, ?, ?, ?, 'Aplikasi lokal')")
            .bind(&provision_id)
            .bind(company_id)
            .bind(&input.business_date)
            .bind(&input.display_date)
            .bind(&input.display_time)
            .bind(&member_id)
            .bind(&member_name)
            .bind(format!("KBS/PROV/{}/{}", input.loan_id, provision_id))
            .bind(TransactionDirection::In.as_str())
            .bind(provision)
            .bind(TransactionStatus::Posted.as_str())
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES (?, ?, 'LOAN_PROVISION', 'Pendapatan provisi', ?, ?)")
            .bind(company_id)
            .bind(&provision_id)
            .bind(provision)
            .bind(&input.loan_id)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)",
        )
        .bind(company_id)
        .bind(&provision_id)
        .bind(TransactionDirection::In.as_str())
        .bind(provision)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    sqlx::query("UPDATE loans SET balance = plafond, realization_date = ?, due_date = ?, status = ? WHERE company_id = ? AND id = ?")
        .bind(&input.business_date)
        .bind(&due_date)
        .bind(LoanStatus::Active.as_str())
        .bind(company_id)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'LOAN', ?, 'DISBURSED', ?, 'Aplikasi lokal')")
        .bind(company_id).bind(&input.loan_id)
        .bind(serde_json::to_string(&serde_json::json!({"plafond": plafond, "provision": provision, "dueDate": due_date})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
