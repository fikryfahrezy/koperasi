//! Application use cases for loans.

use sqlx::{SqliteConnection, SqlitePool};

use super::posting::{append, audit, Component, Entry, APP_ACTOR};
use super::read_model::get_app_snapshot;
use super::savings::member_name;
use crate::{
    contracts::{AppSnapshot, CreateLoanInput, LoanPreview, PreviewLoanInput},
    domain::{
        add_months, calculate_loan, calculate_rate_amount, timestamp_id, validate_business_date,
        AuditAction, AuditEntityType, Channel, ComponentType, InterestType, LoanGroup, LoanType,
        ParameterKey, TransactionDirection,
    },
};

/// 1.0 -> "1", 1.5 -> "1,5".
fn format_rate(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.replace('.', ",")
}

/// 2500000 -> "2.500.000", as in provision fee descriptions in the daily cash ledger.
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
    input: PreviewLoanInput,
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

/// A loan is recorded when it is realised, as in PINJAMAN BULANAN: the principal outflow
/// and provision fee inflow are written as two rows, as in the daily cash ledger.
pub(crate) async fn create_loan(
    input: CreateLoanInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Data pinjaman tidak valid.".into());
    }
    let loan_group = LoanGroup::try_from(input.loan_group.as_str())?;
    let loan_type = LoanType::try_from(input.loan_type.as_str())?;
    let interest_type = InterestType::try_from(input.interest_type.as_str())?;
    let disbursement = input.disbursement;
    let channel = Channel::try_from(disbursement.channel.as_str())?;
    validate_business_date(&disbursement.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let (member_id, borrower_name) = match loan_group {
        LoanGroup::Member => (
            Some(input.member_id.as_str()),
            member_name(&mut db, company_id, &input.member_id).await?,
        ),
        LoanGroup::NonMember => {
            let name = input.borrower_name.trim();
            if name.is_empty() {
                return Err("Nama peminjam wajib diisi.".into());
            }
            (None, name.to_string())
        }
    };
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
    let provision_rate = parameter(
        &mut db,
        company_id,
        ParameterKey::LoanProvisionRate,
        &disbursement.business_date,
    )
    .await?;
    let provision = calculate_rate_amount(input.plafond, provision_rate)?;
    let due_date = add_months(&disbursement.business_date, input.tenor)?;

    let id = timestamp_id("LOAN");
    sqlx::query("INSERT INTO loans (id, company_id, loan_group, member_id, member_name, plafond, rate_annual, tenor, interest_type, loan_type, realization_date, due_date) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(company_id).bind(loan_group.as_str()).bind(member_id).bind(&borrower_name).bind(input.plafond).bind(rate_percent).bind(input.tenor).bind(interest_type.as_str()).bind(loan_type.as_str()).bind(&disbursement.business_date).bind(&due_date).execute(&mut *db).await.map_err(|error| error.to_string())?;

    let disbursement_id = timestamp_id("DISB");
    append(
        &mut db,
        company_id,
        Entry {
            id: disbursement_id.clone(),
            business_date: &disbursement.business_date,
            display_date: &disbursement.display_date,
            display_time: &disbursement.display_time,
            member_id,
            member_name: &borrower_name,
            channel,
            description: &format!("Realisasi Pinjaman {borrower_name}"),
            reference: &format!("KBS/DISB/{id}/{disbursement_id}"),
            direction: TransactionDirection::Out,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components: vec![Component::new(
                ComponentType::LoanDisbursement,
                "Pencairan pokok pinjaman",
                input.plafond,
            )
            .for_loan(&id)],
        },
    )
    .await?;
    if provision > 0 {
        let provision_id = timestamp_id("PROV");
        append(
            &mut db,
            company_id,
            Entry {
                id: provision_id.clone(),
                business_date: &disbursement.business_date,
                display_date: &disbursement.display_date,
                display_time: &disbursement.display_time,
                member_id,
                member_name: &borrower_name,
                channel,
                description: &format!(
                    "Provisi {} % x Rp. {}",
                    format_rate(provision_rate * 100.0),
                    format_thousands(input.plafond)
                ),
                reference: &format!("KBS/PROV/{id}/{provision_id}"),
                direction: TransactionDirection::In,
                actor: APP_ACTOR,
                reversed_transaction_id: None,
                components: vec![Component::new(
                    ComponentType::LoanProvision,
                    "Pendapatan provisi",
                    provision,
                )
                .for_loan(&id)],
            },
        )
        .await?;
    }
    audit(
        &mut db,
        company_id,
        AuditEntityType::Loan,
        &id,
        AuditAction::Disbursed,
        serde_json::json!({"loanGroup": loan_group.as_str(), "memberId": member_id, "plafond": input.plafond, "tenor": input.tenor, "interestType": interest_type.as_str(), "loanType": loan_type.as_str(), "provision": provision, "dueDate": due_date, "channel": channel.as_str()}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
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
