//! Financial parameters needed by operational forms.

use crate::contracts::FinancialParametersDto;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub(crate) async fn get_financial_parameters(
    company_id: &str,
    pool: &SqlitePool,
) -> Result<FinancialParametersDto, String> {
    let parameter_rows = sqlx::query("SELECT parameter_key, value, effective_date FROM parameters WHERE company_id = ? AND effective_date = (SELECT MAX(effective_date) FROM parameters WHERE company_id = ? AND effective_date <= '2026-09-13')")
        .bind(company_id).bind(company_id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut values = HashMap::new();
    let mut effective_date = String::new();
    for row in parameter_rows {
        values.insert(
            row.try_get::<String, _>("parameter_key")
                .map_err(|error| error.to_string())?,
            row.try_get::<f64, _>("value")
                .map_err(|error| error.to_string())?,
        );
        effective_date = row
            .try_get("effective_date")
            .map_err(|error| error.to_string())?;
    }
    Ok(FinancialParametersDto {
        principal_savings: values.get("SAVINGS_PRINCIPAL").copied().unwrap_or(0.0) as i64,
        mandatory_savings: values.get("SAVINGS_MONTHLY").copied().unwrap_or(0.0) as i64,
        provision_rate: values.get("LOAN_PROVISION_RATE").copied().unwrap_or(0.0) * 100.0,
        annual_rate: values.get("LOAN_ANNUAL_RATE").copied().unwrap_or(0.0) * 100.0,
        effective_date,
    })
}
