//! Application use cases for transactions: reversals and general cash entries.

use sqlx::{Row, SqlitePool};

use super::posting::{
    append, audit, ensure_period_open, ensure_reference_unused, loan_balance, savings_balance,
    Component, Entry, APP_ACTOR,
};
use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, CashEntryInput, ReverseTransactionInput},
    domain::{
        timestamp_id, AuditAction, AuditEntityType, Channel, ComponentType, TransactionDirection,
        TransactionType,
    },
};

/// Correction without modifying data: a new transaction in the opposite direction references
/// the original transaction. Both are then excluded from balance calculations.
pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    ensure_period_open(&mut db, company_id, &input.business_date).await?;
    let id = &input.id;
    let original = sqlx::query("SELECT member_id, member_name, transaction_type, channel, description, reference, direction FROM effective_transactions WHERE company_id = ? AND id = ?")
        .bind(company_id)
        .bind(id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or("Transaksi tidak ditemukan atau sudah dibalik.")?;
    let get = |column: &str| -> Result<String, String> {
        original.try_get(column).map_err(|error| error.to_string())
    };
    let member_id: Option<String> = original
        .try_get("member_id")
        .map_err(|error| error.to_string())?;
    let channel = Channel::try_from(get("channel")?.as_str())?;
    let direction = TransactionDirection::try_from(get("direction")?.as_str())?.reversed();

    let mut components = Vec::new();
    for component in sqlx::query("SELECT component_type, label, amount, loan_id, savings_account_id FROM transaction_components WHERE company_id = ? AND transaction_id = ? ORDER BY id")
        .bind(company_id)
        .bind(id)
        .fetch_all(&mut *db)
        .await
        .map_err(|error| error.to_string())?
    {
        let component_type: String = component.try_get("component_type").map_err(|error| error.to_string())?;
        let component_type = ComponentType::try_from(component_type.as_str())?;
        let label: String = component.try_get("label").map_err(|error| error.to_string())?;
        let amount: i64 = component.try_get("amount").map_err(|error| error.to_string())?;
        let loan_id: Option<String> = component.try_get("loan_id").map_err(|error| error.to_string())?;
        let savings_account_id: Option<String> = component.try_get("savings_account_id").map_err(|error| error.to_string())?;
        // Reversing an inflow must not make the balance negative.
        if let Some(account_id) = &savings_account_id {
            if component_type != ComponentType::SavingsWithdrawal
                && savings_balance(&mut db, company_id, account_id).await? < amount
            {
                return Err("Reversal ditolak karena saldo simpanan tidak mencukupi.".into());
            }
        }
        if let Some(loan_id) = &loan_id {
            if matches!(component_type, ComponentType::LoanDisbursement | ComponentType::LoanOpening)
                && loan_balance(&mut db, company_id, loan_id).await? < amount
            {
                return Err("Reversal ditolak karena pinjaman sudah memiliki angsuran.".into());
            }
        }
        let mut reversal = Component::new(ComponentType::Reversal, format!("Reversal · {label}"), amount);
        reversal.loan_id = loan_id;
        reversal.savings_account_id = savings_account_id;
        components.push(reversal);
    }

    let reversal_id = timestamp_id("REV");
    let description = format!("Reversal · {}", get("description")?);
    let reference = format!("{}/REV/{reversal_id}", get("reference")?);
    append(
        &mut db,
        company_id,
        Entry {
            id: reversal_id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: member_id.as_deref(),
            member_name: &get("member_name")?,
            transaction_type: TransactionType::Reversal,
            channel,
            description: &description,
            reference: &reference,
            direction,
            actor: APP_ACTOR,
            reversed_transaction_id: Some(id),
            components,
        },
    )
    .await?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Transaction,
        id,
        AuditAction::Reversed,
        serde_json::json!({"reversalId": reversal_id}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// General cash inflows/outflows (expenses, additional cash, bank transfers, ...).
pub(crate) async fn post_cash_entry(
    input: CashEntryInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.amount <= 0 {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }
    let category = input.category.trim();
    if category.is_empty() {
        return Err("Kategori transaksi wajib diisi.".into());
    }
    let direction = TransactionDirection::try_from(input.direction.as_str())?;
    let channel = Channel::try_from(input.channel.as_str())?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    ensure_period_open(&mut db, company_id, &input.business_date).await?;
    let id = timestamp_id("CASH");
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/CASH/{id}")
    } else {
        input.reference.trim().to_string()
    };
    ensure_reference_unused(&mut db, company_id, &reference).await?;
    let description = if input.description.trim().is_empty() {
        category.to_string()
    } else {
        input.description.trim().to_string()
    };
    let transaction_type = if direction == TransactionDirection::In {
        TransactionType::CashIncome
    } else {
        TransactionType::CashExpense
    };
    append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: None,
            member_name: "Koperasi",
            transaction_type,
            channel,
            description: &description,
            reference: &reference,
            direction,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components: vec![Component::new(
                ComponentType::CashOther,
                category,
                input.amount,
            )],
        },
    )
    .await?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Transaction,
        &id,
        AuditAction::Posted,
        serde_json::json!({"category": category, "amount": input.amount, "channel": channel.as_str()}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
