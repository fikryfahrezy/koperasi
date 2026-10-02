//! Application use cases for members.

use sqlx::SqlitePool;

use super::posting::{append, audit, Component, Entry, APP_ACTOR};
use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AddMemberInput, AppSnapshot},
    domain::{
        timestamp_id, validate_business_date, AuditAction, AuditEntityType, Channel, ComponentType,
        SavingsAccountType, TransactionDirection,
    },
};

pub(crate) async fn add_member(
    input: AddMemberInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.name.trim().is_empty() {
        return Err("Nama anggota wajib diisi.".into());
    }
    let channel = Channel::try_from(input.channel.as_str())?;

    let mut opening_savings = Vec::with_capacity(input.opening_savings.len());
    for opening in &input.opening_savings {
        let account_type = SavingsAccountType::try_from(opening.account_type.as_str())?;
        if opening.amount < 0 {
            return Err("Saldo awal simpanan tidak boleh negatif.".into());
        }
        if opening_savings
            .iter()
            .any(|(existing, _)| *existing == account_type)
        {
            return Err(format!(
                "Jenis simpanan {} tidak boleh diduplikasi.",
                account_type.as_str().to_lowercase()
            ));
        }
        opening_savings.push((account_type, opening.amount));
    }
    let principal_savings = opening_savings
        .iter()
        .find_map(|(kind, amount)| (*kind == SavingsAccountType::Principal).then_some(*amount))
        .ok_or("Simpanan pokok wajib disertakan.")?;
    if principal_savings < 50_000 {
        return Err("Simpanan pokok minimal Rp50.000.".into());
    }

    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(id, INSTR(id, '-M') + 2) AS INTEGER)), 0) + 1 FROM members WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    let id = format!("{company_id}-M{next:03}");
    let name = input.name.trim();
    sqlx::query("INSERT INTO members (id, company_id, name, joined_at) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(company_id)
        .bind(name)
        .bind(input.joined_at.trim())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    for kind in [
        SavingsAccountType::Principal,
        SavingsAccountType::Mandatory,
        SavingsAccountType::Voluntary,
    ] {
        sqlx::query("INSERT INTO savings_accounts (id, company_id, member_id, account_type) VALUES (?, ?, ?, ?)")
            .bind(format!("SA-{id}-{}", kind.as_str())).bind(company_id).bind(&id).bind(kind.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }

    let transaction_id = timestamp_id("SAV-IN");
    let reference = format!("KBS/SAV/{transaction_id}");
    let description = if opening_savings
        .iter()
        .filter(|(_, amount)| *amount > 0)
        .count()
        == 1
    {
        "Setoran simpanan pokok"
    } else {
        "Setoran awal simpanan"
    };
    let components = opening_savings
        .iter()
        .map(|(kind, amount)| {
            Component::new(
                ComponentType::SavingsDeposit,
                format!("Setoran simpanan {}", kind.as_str().to_lowercase()),
                *amount,
            )
            .for_savings(format!("SA-{id}-{}", kind.as_str()))
        })
        .collect();
    let amount = append(
        &mut db,
        company_id,
        Entry {
            id: transaction_id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: Some(&id),
            member_name: name,
            channel,
            description,
            reference: &reference,
            direction: TransactionDirection::In,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components,
        },
    )
    .await?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Transaction,
        &transaction_id,
        AuditAction::Posted,
        serde_json::json!({"openingSavings": opening_savings.iter().map(|(kind, amount)| (kind.as_str(), amount)).collect::<Vec<_>>(), "amount": amount, "reference": reference}),
    )
    .await?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Member,
        &id,
        AuditAction::Created,
        serde_json::json!({"name": name}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
