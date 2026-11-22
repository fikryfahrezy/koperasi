//! Financial parameters needed by operational forms.

use super::posting::audit_change;
use crate::{
    contracts::{FinancialParametersDto, SetFinancialParametersInput, SetIncentiveRatesInput},
    domain::{validate_business_date, AuditAction, AuditEntityType, ParameterKey},
};

use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub(crate) async fn get_financial_parameters(
    company_id: &str,
    pool: &SqlitePool,
) -> Result<FinancialParametersDto, String> {
    // Each key is versioned on its own: the value in force today is its latest
    // row dated on or before today. The yearly incentive rates are read per
    // book year instead; see `incentive_rates`.
    let mut values = HashMap::new();
    let mut effective_date = String::new();

    for key in [
        ParameterKey::PrincipalSavings,
        ParameterKey::MandatorySavings,
        ParameterKey::LoanProvisionRate,
        ParameterKey::LoanAnnualRate,
        ParameterKey::LoanMaxOutstanding,
    ] {
        let row = sqlx::query(
            "SELECT value, effective_date
             FROM parameters
             WHERE company_id = ?
               AND parameter_key = ?
               AND effective_date <= date('now', 'localtime')
             ORDER BY effective_date DESC
             LIMIT 1",
        )
        .bind(company_id)
        .bind(key.as_str())
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;

        let Some(row) = row else { continue };
        let date: String = row
            .try_get("effective_date")
            .map_err(|error| error.to_string())?;

        // The newest change among the keys is when the current set took effect.
        if date > effective_date {
            effective_date = date;
        }

        values.insert(
            key,
            row.try_get::<f64, _>("value")
                .map_err(|error| error.to_string())?,
        );
    }

    let value = |key| values.get(&key).copied().unwrap_or(0.0);
    // Fractions to percent without floating-point noise (0.07 -> 7, not
    // 7.000000000000001).
    let percent = |key| (value(key) * 100.0 * 1e6).round() / 1e6;

    Ok(FinancialParametersDto {
        principal_savings: value(ParameterKey::PrincipalSavings) as i64,
        mandatory_savings: value(ParameterKey::MandatorySavings) as i64,
        provision_rate: percent(ParameterKey::LoanProvisionRate),
        annual_rate: percent(ParameterKey::LoanAnnualRate),
        max_loan_outstanding: value(ParameterKey::LoanMaxOutstanding) as i64,
        effective_date,
    })
}

/// Records new values of the form parameters (Pengaturan), effective from
/// `effective_date`; earlier dates keep their values. Saving again on the same
/// date overwrites that date's row.
pub(crate) async fn set_financial_parameters(
    input: SetFinancialParametersInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<(), String> {
    validate_business_date(&input.effective_date)
        .map_err(|_| "Tanggal berlaku tidak valid.".to_string())?;

    let rate = |percent: Option<f64>, label: &str| match percent {
        Some(percent) if !(0.0..=100.0).contains(&percent) => {
            Err(format!("{label} harus antara 0% dan 100%."))
        }
        percent => Ok(percent.map(|percent| percent / 100.0)),
    };
    let amount = |value: Option<i64>, label: &str| match value {
        Some(value) if value < 0 => Err(format!("{label} tidak boleh negatif.")),
        value => Ok(value.map(|value| value as f64)),
    };

    let values = [
        (
            ParameterKey::PrincipalSavings,
            amount(input.principal_savings, "Simpanan pokok")?,
        ),
        (
            ParameterKey::MandatorySavings,
            amount(input.mandatory_savings, "Simpanan wajib")?,
        ),
        (
            ParameterKey::LoanProvisionRate,
            rate(input.provision_rate, "Provisi")?,
        ),
        (
            ParameterKey::LoanAnnualRate,
            rate(input.annual_rate, "Bunga")?,
        ),
        (
            ParameterKey::LoanMaxOutstanding,
            amount(input.max_loan_outstanding, "Batas sisa pinjaman")?,
        ),
    ];

    save_parameters(company_id, &input.effective_date, values, pool).await
}

/// Writes each value given at `effective_date`, with one audit event holding
/// the values it replaces.
async fn save_parameters(
    company_id: &str,
    effective_date: &str,
    values: impl IntoIterator<Item = (ParameterKey, Option<f64>)>,
    pool: &SqlitePool,
) -> Result<(), String> {
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let mut before = serde_json::Map::new();
    let mut after = serde_json::Map::new();

    for (key, value) in values {
        let Some(value) = value else { continue };

        // A value saved again for the same date overwrites the row; keep the old one.
        let previous: Option<f64> = sqlx::query_scalar(
            "SELECT value
             FROM parameters
             WHERE company_id = ? AND parameter_key = ? AND effective_date = ?",
        )
        .bind(company_id)
        .bind(key.as_str())
        .bind(effective_date)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?;

        before.insert(key.as_str().into(), previous.into());
        after.insert(key.as_str().into(), value.into());

        sqlx::query(
            "INSERT INTO parameters (company_id, parameter_key, value, effective_date, created_by)
             VALUES (?, ?, ?, ?, 'Aplikasi')
             ON CONFLICT (company_id, parameter_key, effective_date)
             DO UPDATE SET value = excluded.value",
        )
        .bind(company_id)
        .bind(key.as_str())
        .bind(value)
        .bind(effective_date)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    if !after.is_empty() {
        audit_change(
            &mut db,
            company_id,
            AuditEntityType::Parameter,
            effective_date,
            AuditAction::Updated,
            Some(before.into()),
            after.into(),
        )
        .await?;
    }

    db.commit().await.map_err(|error| error.to_string())
}

/// The latest value of a yearly parameter effective on or before 31 December
/// of `year` (0 when never set).
async fn yearly_parameter(
    company_id: &str,
    key: ParameterKey,
    year: i32,
    pool: &SqlitePool,
) -> Result<f64, String> {
    let value: Option<f64> = sqlx::query_scalar(
        "SELECT value
         FROM parameters
         WHERE company_id = ? AND parameter_key = ? AND effective_date <= ?
         ORDER BY effective_date DESC
         LIMIT 1",
    )
    .bind(company_id)
    .bind(key.as_str())
    .bind(format!("{year:04}-12-31"))
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(value.unwrap_or(0.0))
}

/// Insentif Pinjaman and Insentif Simpanan rates for a book year: the latest
/// values effective on or before 31 December of that year (0 when never set).
pub(crate) async fn incentive_rates(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<(f64, f64), String> {
    Ok((
        yearly_parameter(company_id, ParameterKey::LoanIncentiveRate, year, pool).await?,
        yearly_parameter(company_id, ParameterKey::SavingsIncentiveRate, year, pool).await?,
    ))
}

/// Uang duduk + uang makan per anggota in the Daftar SHU of a book year, in
/// rupiah; a year without its own amount keeps the previous one.
pub(crate) async fn sitting_allowance(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<i64, String> {
    Ok(
        yearly_parameter(company_id, ParameterKey::ShuSittingAllowance, year, pool)
            .await?
            .round() as i64,
    )
}

/// Records the incentive rates and the uang duduk decided for a book year
/// (effective 1 January).
pub(crate) async fn set_incentive_rates(
    input: SetIncentiveRatesInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<(), String> {
    let rates = [
        (ParameterKey::LoanIncentiveRate, input.loan_incentive_rate),
        (
            ParameterKey::SavingsIncentiveRate,
            input.savings_incentive_rate,
        ),
    ];

    if rates
        .iter()
        .any(|(_, rate)| rate.is_some_and(|rate| !(0.0..=1.0).contains(&rate)))
    {
        return Err("Persentase insentif harus antara 0% dan 100%.".into());
    }

    if input.sitting_allowance.is_some_and(|amount| amount < 0) {
        return Err("Uang duduk tidak boleh negatif.".into());
    }

    let rates = rates.into_iter().chain([(
        ParameterKey::ShuSittingAllowance,
        input.sitting_allowance.map(|amount| amount as f64),
    )]);

    let effective_date = format!("{:04}-01-01", input.year);

    save_parameters(company_id, &effective_date, rates, pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_parameters_change_from_their_date() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let input = |provision_rate, max_loan_outstanding| SetFinancialParametersInput {
                effective_date: "2026-02-01".into(),
                principal_savings: None,
                mandatory_savings: Some(60_000),
                provision_rate,
                annual_rate: None,
                max_loan_outstanding,
            };

            set_financial_parameters(input(Some(2.0), Some(30_000_000)), "default", &pool)
                .await
                .unwrap();

            let parameters = get_financial_parameters("default", &pool).await.unwrap();
            assert_eq!(parameters.mandatory_savings, 60_000);
            assert_eq!(parameters.provision_rate, 2.0);
            assert_eq!(parameters.max_loan_outstanding, 30_000_000);
            // Left out: unchanged.
            assert_eq!(parameters.annual_rate, 24.0);
            assert_eq!(parameters.principal_savings, 50_000);
            assert_eq!(parameters.effective_date, "2026-02-01");

            assert!(
                set_financial_parameters(input(Some(101.0), None), "default", &pool)
                    .await
                    .is_err()
            );
            assert!(
                set_financial_parameters(input(None, Some(-1)), "default", &pool)
                    .await
                    .is_err()
            );

            let mut bad_date = input(None, None);
            bad_date.effective_date = "2026-02-30".into();
            assert!(set_financial_parameters(bad_date, "default", &pool)
                .await
                .is_err());
        });
    }

    #[test]
    fn incentive_rates_follow_the_book_year() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let before = get_financial_parameters("default", &pool).await.unwrap();

            set_incentive_rates(
                SetIncentiveRatesInput {
                    year: 2025,
                    loan_incentive_rate: Some(0.101),
                    savings_incentive_rate: Some(0.0365),
                    sitting_allowance: Some(100_000),
                },
                "default",
                &pool,
            )
            .await
            .unwrap();

            // RAT 2025 rates apply until a later year sets its own.
            assert_eq!(
                incentive_rates("default", 2026, &pool).await.unwrap(),
                (0.101, 0.0365)
            );

            set_incentive_rates(
                SetIncentiveRatesInput {
                    year: 2027,
                    loan_incentive_rate: Some(0.12),
                    savings_incentive_rate: None,
                    sitting_allowance: None,
                },
                "default",
                &pool,
            )
            .await
            .unwrap();

            assert_eq!(
                incentive_rates("default", 2026, &pool).await.unwrap(),
                (0.101, 0.0365)
            );

            assert_eq!(
                incentive_rates("default", 2027, &pool).await.unwrap(),
                (0.12, 0.0365)
            );

            // Uang duduk is kept per year the same way.
            assert_eq!(sitting_allowance("default", 2024, &pool).await.unwrap(), 0);
            assert_eq!(
                sitting_allowance("default", 2027, &pool).await.unwrap(),
                100_000
            );

            // Saving a year again keeps the replaced rate in the audit.
            set_incentive_rates(
                SetIncentiveRatesInput {
                    year: 2027,
                    loan_incentive_rate: Some(0.13),
                    savings_incentive_rate: None,
                    sitting_allowance: None,
                },
                "default",
                &pool,
            )
            .await
            .unwrap();

            let (replaced, saved): (String, String) = sqlx::query_as(
                "SELECT before_json, after_json
                 FROM audit_events
                 WHERE entity_type = 'PARAMETER' AND entity_id = '2027-01-01'
                 ORDER BY id DESC
                 LIMIT 1",
            )
            .fetch_one(&pool)
            .await
            .unwrap();

            assert_eq!(replaced, r#"{"LOAN_INCENTIVE_RATE":0.12}"#);
            assert_eq!(saved, r#"{"LOAN_INCENTIVE_RATE":0.13}"#);

            // The operational parameters are unaffected. (Batas pinjaman came from
            // a data script, not from the migrations, so it is compared with its
            // value before the rates were set instead of a fixed amount.)
            let parameters = get_financial_parameters("default", &pool).await.unwrap();
            assert_eq!(parameters.mandatory_savings, 50_000);
            assert_eq!(parameters.max_loan_outstanding, before.max_loan_outstanding);
        });
    }

    #[test]
    fn each_parameter_uses_its_own_latest_date() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;

            for (key, value, date) in [
                // A new rate for one key keeps the other keys' values.
                (ParameterKey::LoanAnnualRate, 0.18, "2026-06-01"),
                // A rate that has not started yet is not used.
                (ParameterKey::MandatorySavings, 75_000.0, "2999-01-01"),
            ] {
                sqlx::query(
                    "INSERT INTO parameters (
                         company_id, parameter_key, value, effective_date, created_by
                     )
                     VALUES ('default', ?, ?, ?, 'Test')",
                )
                .bind(key.as_str())
                .bind(value)
                .bind(date)
                .execute(&pool)
                .await
                .unwrap();
            }

            let parameters = get_financial_parameters("default", &pool).await.unwrap();
            assert_eq!(parameters.annual_rate, 18.0);
            assert_eq!(parameters.provision_rate, 1.0);
            assert_eq!(parameters.principal_savings, 50_000);
            assert_eq!(parameters.mandatory_savings, 50_000);
            assert_eq!(parameters.effective_date, "2026-06-01");
        });
    }
}
