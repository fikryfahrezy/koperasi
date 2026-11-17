//! Ledger writes. All money movements pass through here as
//! new transactions with their components; balances are never updated in place.

use sqlx::SqliteConnection;

use crate::domain::{AuditAction, AuditEntityType, Channel, ComponentType, TransactionDirection};

pub(crate) const APP_ACTOR: &str = "Aplikasi lokal";

pub(crate) struct Component {
    pub(crate) component_type: ComponentType,
    pub(crate) label: String,
    pub(crate) amount: i64,
    pub(crate) loan_id: Option<String>,
    pub(crate) savings_account_id: Option<String>,
}

impl Component {
    pub(crate) fn new(
        component_type: ComponentType,
        label: impl Into<String>,
        amount: i64,
    ) -> Self {
        Self {
            component_type,
            label: label.into(),
            amount,
            loan_id: None,
            savings_account_id: None,
        }
    }

    pub(crate) fn for_loan(mut self, loan_id: impl Into<String>) -> Self {
        self.loan_id = Some(loan_id.into());
        self
    }

    pub(crate) fn for_savings(mut self, savings_account_id: impl Into<String>) -> Self {
        self.savings_account_id = Some(savings_account_id.into());
        self
    }
}

pub(crate) struct Entry<'a> {
    pub(crate) id: String,
    pub(crate) business_date: &'a str,
    pub(crate) member_id: Option<&'a str>,
    pub(crate) member_name: &'a str,
    pub(crate) channel: Channel,
    pub(crate) description: &'a str,
    pub(crate) direction: TransactionDirection,
    pub(crate) reversed_transaction_id: Option<&'a str>,
    pub(crate) components: Vec<Component>,
}

pub(crate) async fn savings_balance(
    db: &mut SqliteConnection,
    company_id: &str,
    savings_account_id: &str,
) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT balance
         FROM savings_balances
         WHERE company_id = ? AND savings_account_id = ?",
    )
    .bind(company_id)
    .bind(savings_account_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Rekening simpanan tidak ditemukan.".to_string())
}

pub(crate) async fn loan_balance(
    db: &mut SqliteConnection,
    company_id: &str,
    loan_id: &str,
) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT balance
         FROM loan_balances
         WHERE company_id = ? AND loan_id = ?",
    )
    .bind(company_id)
    .bind(loan_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Pinjaman tidak ditemukan.".to_string())
}

/// Writes a single transaction. Zero-value components are skipped; the transaction amount
/// is the sum of its components.
pub(crate) async fn append(
    db: &mut SqliteConnection,
    company_id: &str,
    entry: Entry<'_>,
) -> Result<i64, String> {
    let components: Vec<Component> = entry
        .components
        .into_iter()
        .filter(|component| component.amount != 0)
        .collect();

    if components.iter().any(|component| component.amount < 0) {
        return Err("Komponen transaksi tidak boleh negatif.".into());
    }

    let amount = components
        .iter()
        .try_fold(0_i64, |total, component| {
            total.checked_add(component.amount)
        })
        .ok_or("Total transaksi terlalu besar.")?;

    if amount <= 0 {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }

    sqlx::query(
        "INSERT INTO transactions (
             id, company_id, business_date, member_id, member_name,
             channel, description, direction, amount, reversed_transaction_id
         )
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry.id)
    .bind(company_id)
    .bind(entry.business_date)
    .bind(entry.member_id)
    .bind(entry.member_name)
    .bind(entry.channel.as_str())
    .bind(entry.description)
    .bind(entry.direction.as_str())
    .bind(amount)
    .bind(entry.reversed_transaction_id)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    for component in components {
        sqlx::query(
            "INSERT INTO transaction_components (
                 company_id, transaction_id, component_type, label, amount, loan_id,
                 savings_account_id
             )
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(company_id)
        .bind(&entry.id)
        .bind(component.component_type.as_str())
        .bind(&component.label)
        .bind(component.amount)
        .bind(&component.loan_id)
        .bind(&component.savings_account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    Ok(amount)
}

pub(crate) async fn audit(
    db: &mut SqliteConnection,
    company_id: &str,
    entity_type: AuditEntityType,
    entity_id: &str,
    action: AuditAction,
    after: serde_json::Value,
) -> Result<(), String> {
    audit_change(db, company_id, entity_type, entity_id, action, None, after).await
}

/// An audit event that also keeps the values replaced, for data edited in
/// place (Neraca lines, incentive rates) rather than appended to the ledger.
pub(crate) async fn audit_change(
    db: &mut SqliteConnection,
    company_id: &str,
    entity_type: AuditEntityType,
    entity_id: &str,
    action: AuditAction,
    before: Option<serde_json::Value>,
    after: serde_json::Value,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO audit_events (
             company_id, entity_type, entity_id, action, before_json, after_json, actor
         )
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(company_id)
    .bind(entity_type.as_str())
    .bind(entity_id)
    .bind(action.as_str())
    .bind(before.map(|before| serde_json::to_string(&before).unwrap_or_default()))
    .bind(serde_json::to_string(&after).unwrap_or_default())
    .bind(APP_ACTOR)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}
