//! Financial parameters needed by operational forms.

use crate::{
    contracts::{FinancialParametersDto, SetIncentiveRatesInput},
    domain::ParameterKey,
};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub(crate) async fn get_financial_parameters(
    company_id: &str,
    pool: &SqlitePool,
) -> Result<FinancialParametersDto, String> {
    // The yearly incentive rates have their own dates; see `incentive_rates`.
    let parameter_rows = sqlx::query("SELECT parameter_key, value, effective_date FROM parameters WHERE company_id = ? AND parameter_key NOT IN (?, ?) AND effective_date = (SELECT MAX(effective_date) FROM parameters WHERE company_id = ? AND parameter_key NOT IN (?, ?) AND effective_date <= '2026-09-13')")
        .bind(company_id)
        .bind(ParameterKey::LoanIncentiveRate.as_str())
        .bind(ParameterKey::SavingsIncentiveRate.as_str())
        .bind(company_id)
        .bind(ParameterKey::LoanIncentiveRate.as_str())
        .bind(ParameterKey::SavingsIncentiveRate.as_str())
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut values = HashMap::new();
    let mut effective_date = String::new();
    for row in parameter_rows {
        values.insert(
            ParameterKey::try_from(
                row.try_get::<String, _>("parameter_key")
                    .map_err(|error| error.to_string())?
                    .as_str(),
            )?,
            row.try_get::<f64, _>("value")
                .map_err(|error| error.to_string())?,
        );
        effective_date = row
            .try_get("effective_date")
            .map_err(|error| error.to_string())?;
    }
    Ok(FinancialParametersDto {
        principal_savings: values
            .get(&ParameterKey::PrincipalSavings)
            .copied()
            .unwrap_or(0.0) as i64,
        mandatory_savings: values
            .get(&ParameterKey::MandatorySavings)
            .copied()
            .unwrap_or(0.0) as i64,
        provision_rate: values
            .get(&ParameterKey::LoanProvisionRate)
            .copied()
            .unwrap_or(0.0)
            * 100.0,
        annual_rate: values
            .get(&ParameterKey::LoanAnnualRate)
            .copied()
            .unwrap_or(0.0)
            * 100.0,
        effective_date,
    })
}

/// Insentif Pinjaman and Insentif Simpanan rates for a book year: the latest
/// values effective on or before 31 December of that year (0 when never set).
pub(crate) async fn incentive_rates(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<(f64, f64), String> {
    let year_end = format!("{year:04}-12-31");
    let mut rates = [0.0; 2];
    for (slot, key) in [
        ParameterKey::LoanIncentiveRate,
        ParameterKey::SavingsIncentiveRate,
    ]
    .into_iter()
    .enumerate()
    {
        let value: Option<f64> = sqlx::query_scalar(
            "SELECT value FROM parameters WHERE company_id = ? AND parameter_key = ? AND effective_date <= ? ORDER BY effective_date DESC LIMIT 1",
        )
        .bind(company_id)
        .bind(key.as_str())
        .bind(&year_end)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
        rates[slot] = value.unwrap_or(0.0);
    }
    Ok((rates[0], rates[1]))
}

/// Records the incentive rates decided for a book year (effective 1 January).
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
    let effective_date = format!("{:04}-01-01", input.year);
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, value) in rates {
        let Some(value) = value else { continue };
        sqlx::query(
            "INSERT INTO parameters (company_id, parameter_key, value, effective_date, created_by) VALUES (?, ?, ?, ?, 'Aplikasi') ON CONFLICT(company_id, parameter_key, effective_date) DO UPDATE SET value = excluded.value",
        )
        .bind(company_id)
        .bind(key.as_str())
        .bind(value)
        .bind(&effective_date)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }
    db.commit().await.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incentive_rates_follow_the_book_year() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
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
            // The operational parameters are unaffected.
            let parameters = get_financial_parameters("default", &pool).await.unwrap();
            assert_eq!(parameters.mandatory_savings, 50_000);
        });
    }
}
