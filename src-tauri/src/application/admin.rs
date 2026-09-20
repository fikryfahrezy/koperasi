//! Application use cases for admin.

use std::collections::HashMap;

use sqlx::{Row, SqlitePool};

use crate::{
    contracts::{AdminStateDto, AuditEventDto, FinancialParametersDto, SaveParametersInput},
    domain::business_period,
};

async fn admin_state(company_id: &str, pool: &SqlitePool) -> Result<AdminStateDto, String> {
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
    let parameters = FinancialParametersDto {
        principal_savings: values.get("SAVINGS_PRINCIPAL").copied().unwrap_or(0.0) as i64,
        mandatory_savings: values.get("SAVINGS_MONTHLY").copied().unwrap_or(0.0) as i64,
        provision_rate: values.get("LOAN_PROVISION_RATE").copied().unwrap_or(0.0) * 100.0,
        annual_rate: values.get("LOAN_ANNUAL_RATE").copied().unwrap_or(0.0) * 100.0,
        effective_date,
    };
    let audit_rows = sqlx::query("SELECT id, entity_type, entity_id, action, actor, created_at FROM audit_events WHERE company_id = ? ORDER BY id DESC LIMIT 50")
        .bind(company_id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut audit_events = Vec::with_capacity(audit_rows.len());
    for row in audit_rows {
        audit_events.push(AuditEventDto {
            id: row.try_get("id").map_err(|error| error.to_string())?,
            entity_type: row
                .try_get("entity_type")
                .map_err(|error| error.to_string())?,
            entity_id: row
                .try_get("entity_id")
                .map_err(|error| error.to_string())?,
            action: row.try_get("action").map_err(|error| error.to_string())?,
            actor: row.try_get("actor").map_err(|error| error.to_string())?,
            created_at: row
                .try_get("created_at")
                .map_err(|error| error.to_string())?,
        });
    }
    Ok(AdminStateDto {
        parameters,
        audit_events,
    })
}

pub(crate) async fn get_admin_state(
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AdminStateDto, String> {
    admin_state(company_id, pool).await
}

pub(crate) async fn save_financial_parameters(
    input: SaveParametersInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AdminStateDto, String> {
    if input.principal_savings < 0
        || input.mandatory_savings < 0
        || input.principal_savings > (1_i64 << 53)
        || input.mandatory_savings > (1_i64 << 53)
        || input.provision_rate < 0.0
        || input.annual_rate < 0.0
        || !input.provision_rate.is_finite()
        || !input.annual_rate.is_finite()
    {
        return Err("Nilai atau tanggal berlaku parameter tidak valid.".into());
    }
    business_period(&input.effective_date)
        .map_err(|_| "Nilai atau tanggal berlaku parameter tidak valid.".to_string())?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM parameters WHERE company_id = ? AND effective_date = ?",
    )
    .bind(company_id)
    .bind(&input.effective_date)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if existing > 0 {
        return Err("Versi parameter untuk tanggal berlaku tersebut sudah ada.".into());
    }
    for (key, value) in [
        ("SAVINGS_PRINCIPAL", input.principal_savings as f64),
        ("SAVINGS_MONTHLY", input.mandatory_savings as f64),
        ("LOAN_PROVISION_RATE", input.provision_rate / 100.0),
        ("LOAN_ANNUAL_RATE", input.annual_rate / 100.0),
    ] {
        sqlx::query("INSERT INTO parameters (company_id, parameter_key, value, effective_date, created_by) VALUES (?, ?, ?, ?, 'Aplikasi lokal')")
            .bind(company_id).bind(key)
            .bind(value)
            .bind(&input.effective_date)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'PARAMETERS', ?, 'VERSION_CREATED', ?, 'Aplikasi lokal')")
        .bind(company_id).bind(&input.effective_date)
        .bind(serde_json::to_string(&serde_json::json!({"principalSavings": input.principal_savings, "mandatorySavings": input.mandatory_savings, "provisionRate": input.provision_rate, "annualRate": input.annual_rate})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    admin_state(company_id, pool).await
}
