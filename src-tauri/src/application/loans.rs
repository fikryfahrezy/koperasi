//! Application use cases for loans.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::posting::{
    append, audit, ensure_period_open, loan_balance, Component, Entry, APP_ACTOR,
};
use super::read_model::get_app_snapshot;
use super::savings::{active_member_name, money_channel};
use crate::{
    contracts::{AppSnapshot, CreateLoanInput, DisburseLoanInput, LoanPreview},
    domain::{
        add_months, calculate_loan, calculate_rate_amount, timestamp_id, AuditAction,
        AuditEntityType, ComponentType, InterestType, LoanStatus, ParameterKey,
        TransactionDirection, TransactionType,
    },
};

/// 1.0 -> "1", 1.5 -> "1,5".
fn format_rate(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.replace('.', ",")
}

/// 2500000 -> "2.500.000", seperti uraian provisi di Buku Kas Harian.
fn format_thousands(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if value < 0 {
        format!("-{out}")
    } else {
        out
    }
}

async fn parameter(
    db: &mut SqliteConnection,
    company_id: &str,
    key: ParameterKey,
    on_date: &str,
) -> Result<f64, String> {
    sqlx::query_scalar("SELECT value FROM parameters WHERE company_id = ? AND parameter_key = ? AND effective_date <= ? ORDER BY effective_date DESC LIMIT 1")
        .bind(company_id)
        .bind(key.as_str())
        .bind(on_date)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Parameter {} belum diatur.", key.as_str()))
}

pub(crate) async fn preview_loan(
    input: CreateLoanInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<LoanPreview, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Plafond dan tenor harus lebih dari nol.".into());
    }
    InterestType::try_from(input.interest_type.as_str())?;
    let mut db = pool.acquire().await.map_err(|error| error.to_string())?;
    let annual_rate = parameter(
        &mut db,
        company_id,
        ParameterKey::LoanAnnualRate,
        "9999-12-31",
    )
    .await?;
    let provision_rate = parameter(
        &mut db,
        company_id,
        ParameterKey::LoanProvisionRate,
        "9999-12-31",
    )
    .await?;
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
    let member_name = active_member_name(&mut db, company_id, &input.member_id).await?;
    let rate = parameter(
        &mut db,
        company_id,
        ParameterKey::LoanAnnualRate,
        "9999-12-31",
    )
    .await?;
    let rate_percent = rate * 100.0;
    if rate < 0.0 || !rate_percent.is_finite() {
        return Err("Suku bunga pinjaman tidak valid.".into());
    }
    let id = timestamp_id("LOAN");
    sqlx::query("INSERT INTO loans (id, company_id, member_id, member_name, plafond, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '-', '-', ?)")
        .bind(&id).bind(company_id).bind(&input.member_id).bind(&member_name).bind(input.plafond).bind(rate_percent).bind(input.tenor).bind(interest_type.as_str()).bind(LoanStatus::Draft.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Loan,
        &id,
        AuditAction::DraftCreated,
        serde_json::json!({"memberId": input.member_id, "plafond": input.plafond, "tenor": input.tenor, "interestType": input.interest_type}),
    )
    .await?;
    if let Some(disbursement) = input.disbursement {
        disburse(
            &mut db,
            company_id,
            DisburseLoanInput {
                loan_id: id,
                channel: disbursement.channel,
                business_date: disbursement.business_date,
                display_date: disbursement.display_date,
                display_time: disbursement.display_time,
            },
        )
        .await?;
    }
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// Pencairan: uang pokok keluar dan provisi masuk dicatat sebagai dua baris,
/// sama seperti di Buku Kas Harian.
async fn disburse(
    db: &mut SqliteConnection,
    company_id: &str,
    input: DisburseLoanInput,
) -> Result<(), String> {
    let channel = money_channel(&input.channel)?;
    ensure_period_open(db, company_id, &input.business_date).await?;
    let loan = sqlx::query(
        "SELECT member_id, member_name, plafond, tenor, status FROM loans WHERE company_id = ? AND id = ?",
    )
    .bind(company_id)
    .bind(&input.loan_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Pinjaman tidak ditemukan.")?;
    let status: String = loan.try_get("status").map_err(|error| error.to_string())?;
    if LoanStatus::try_from(status.as_str())? != LoanStatus::Draft
        || loan_balance(db, company_id, &input.loan_id).await? != 0
    {
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
    let provision_rate = parameter(
        db,
        company_id,
        ParameterKey::LoanProvisionRate,
        &input.business_date,
    )
    .await?;
    let provision = calculate_rate_amount(plafond, provision_rate)?;

    let disbursement_id = timestamp_id("DISB");
    append(
        db,
        company_id,
        Entry {
            id: disbursement_id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: member_id.as_deref(),
            member_name: &member_name,
            transaction_type: TransactionType::LoanDisbursement,
            channel,
            description: &format!("Realisasi Pinjaman {member_name}"),
            reference: &format!("KBS/DISB/{}/{disbursement_id}", input.loan_id),
            direction: TransactionDirection::Out,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components: vec![Component::new(
                ComponentType::LoanDisbursement,
                "Pencairan pokok pinjaman",
                plafond,
            )
            .for_loan(&input.loan_id)],
        },
    )
    .await?;
    if provision > 0 {
        let provision_id = timestamp_id("PROV");
        append(
            db,
            company_id,
            Entry {
                id: provision_id.clone(),
                business_date: &input.business_date,
                display_date: &input.display_date,
                display_time: &input.display_time,
                member_id: member_id.as_deref(),
                member_name: &member_name,
                transaction_type: TransactionType::LoanProvision,
                channel,
                description: &format!(
                    "Provisi {} % x Rp. {}",
                    format_rate(provision_rate * 100.0),
                    format_thousands(plafond)
                ),
                reference: &format!("KBS/PROV/{}/{provision_id}", input.loan_id),
                direction: TransactionDirection::In,
                actor: APP_ACTOR,
                reversed_transaction_id: None,
                components: vec![Component::new(
                    ComponentType::LoanProvision,
                    "Pendapatan provisi",
                    provision,
                )
                .for_loan(&input.loan_id)],
            },
        )
        .await?;
    }

    // Syarat pinjaman (bukan saldo) ikut ditetapkan saat pencairan.
    sqlx::query("UPDATE loans SET realization_date = ?, due_date = ?, status = ? WHERE company_id = ? AND id = ?")
        .bind(&input.business_date)
        .bind(&due_date)
        .bind(LoanStatus::Active.as_str())
        .bind(company_id)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    audit(
        db,
        company_id,
        AuditEntityType::Loan,
        &input.loan_id,
        AuditAction::Disbursed,
        serde_json::json!({"plafond": plafond, "provision": provision, "dueDate": due_date, "channel": channel.as_str()}),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_provision_descriptions_like_the_cash_book() {
        assert_eq!(format_thousands(2_500_000), "2.500.000");
        assert_eq!(format_thousands(500), "500");
        assert_eq!(format_rate(1.0), "1");
        assert_eq!(format_rate(1.5), "1,5");
    }
}
