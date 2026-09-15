//! Application use cases for admin.

use std::collections::HashMap;

use sqlx::{Row, SqlitePool};

use crate::contracts::{AdminStateDto, AuditEventDto, FinancialParametersDto, SaveParametersInput};

async fn admin_state(pool: &SqlitePool) -> Result<AdminStateDto, String> {
    let parameter_rows = sqlx::query("SELECT parameter_key, value, effective_date FROM parameters WHERE effective_date = (SELECT MAX(effective_date) FROM parameters WHERE effective_date <= '2026-09-13')")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut values = HashMap::new();
    let mut effective_date = String::new();
    for row in parameter_rows {
        values.insert(
            row.get::<String, _>("parameter_key"),
            row.get::<f64, _>("value"),
        );
        effective_date = row.get("effective_date");
    }
    let parameters = FinancialParametersDto {
        principal_savings: values.get("SAVINGS_PRINCIPAL").copied().unwrap_or(0.0) as i64,
        mandatory_savings: values.get("SAVINGS_MONTHLY").copied().unwrap_or(0.0) as i64,
        provision_rate: values.get("LOAN_PROVISION_RATE").copied().unwrap_or(0.0) * 100.0,
        annual_rate: values.get("LOAN_ANNUAL_RATE").copied().unwrap_or(0.0) * 100.0,
        effective_date,
    };
    let audit_rows = sqlx::query("SELECT id, entity_type, entity_id, action, actor, created_at FROM audit_events ORDER BY id DESC LIMIT 50")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let audit_events = audit_rows
        .into_iter()
        .map(|row| AuditEventDto {
            id: row.get("id"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            action: row.get("action"),
            actor: row.get("actor"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(AdminStateDto {
        parameters,
        audit_events,
    })
}

pub(crate) async fn get_admin_state(pool: &SqlitePool) -> Result<AdminStateDto, String> {
    admin_state(pool).await
}

pub(crate) async fn save_financial_parameters(
    input: SaveParametersInput,
    pool: &SqlitePool,
) -> Result<AdminStateDto, String> {
    if input.principal_savings < 0
        || input.mandatory_savings < 0
        || input.provision_rate < 0.0
        || input.annual_rate < 0.0
        || input.effective_date.len() != 10
    {
        return Err("Nilai atau tanggal berlaku parameter tidak valid.".into());
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM parameters WHERE effective_date = ?")
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
        sqlx::query("INSERT INTO parameters (parameter_key, value, effective_date, created_by) VALUES (?, ?, ?, 'Aplikasi lokal')")
            .bind(key)
            .bind(value)
            .bind(&input.effective_date)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('PARAMETERS', ?, 'VERSION_CREATED', ?, 'Aplikasi lokal')")
        .bind(&input.effective_date)
        .bind(serde_json::to_string(&serde_json::json!({"principalSavings": input.principal_savings, "mandatorySavings": input.mandatory_savings, "provisionRate": input.provision_rate, "annualRate": input.annual_rate})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    admin_state(pool).await
}
