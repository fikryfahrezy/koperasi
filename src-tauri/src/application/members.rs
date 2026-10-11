//! Application use cases for members.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::posting::{append, audit, Component, Entry, APP_ACTOR};
use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AddMemberInput, AppSnapshot, UpdateMemberInput},
    domain::{
        timestamp_id, validate_business_date, AuditAction, AuditEntityType, Channel, ComponentType,
        SavingsAccountType, TransactionDirection,
    },
};

/// Validates an ID Anggota: required and not used by another member of the company.
async fn check_member_number(
    db: &mut SqliteConnection,
    company_id: &str,
    member_number: &str,
    member_id: Option<&str>,
) -> Result<(), String> {
    if member_number.is_empty() {
        return Err("ID anggota wajib diisi.".into());
    }
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT name FROM members WHERE company_id = ? AND member_number = ? COLLATE NOCASE AND id <> ?",
    )
    .bind(company_id)
    .bind(member_number)
    .bind(member_id.unwrap_or(""))
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    match owner {
        Some(name) => Err(format!(
            "ID anggota {member_number} sudah dipakai oleh {name}."
        )),
        None => Ok(()),
    }
}

pub(crate) async fn update_member(
    input: UpdateMemberInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Nama anggota wajib diisi.".into());
    }
    let member_number = input.member_number.trim();
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let previous = sqlx::query(
        "SELECT name, COALESCE(member_number, '') member_number, joined_at FROM members WHERE company_id = ? AND id = ?",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Anggota tidak ditemukan.")?;
    let previous_name: String = previous
        .try_get("name")
        .map_err(|error| error.to_string())?;
    let previous_number: String = previous
        .try_get("member_number")
        .map_err(|error| error.to_string())?;
    let previous_joined_at: String = previous
        .try_get("joined_at")
        .map_err(|error| error.to_string())?;
    let joined_at = match input.joined_at.trim() {
        "" => previous_joined_at.as_str(),
        value => {
            validate_business_date(value)?;
            value
        }
    };
    check_member_number(&mut db, company_id, member_number, Some(&input.member_id)).await?;
    if previous_name == name && previous_number == member_number && previous_joined_at == joined_at
    {
        db.commit().await.map_err(|error| error.to_string())?;
        return get_app_snapshot(company_id, pool).await;
    }
    sqlx::query(
        "UPDATE members SET name = ?, member_number = ?, joined_at = ? WHERE company_id = ? AND id = ?",
    )
        .bind(name)
        .bind(member_number)
        .bind(joined_at)
        .bind(company_id)
        .bind(&input.member_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    // Loans show the borrower's current name; posted transactions keep the name
    // they were recorded under.
    if previous_name != name {
        sqlx::query("UPDATE loans SET member_name = ? WHERE company_id = ? AND member_id = ?")
            .bind(name)
            .bind(company_id)
            .bind(&input.member_id)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    }
    audit(
        &mut db,
        company_id,
        AuditEntityType::Member,
        &input.member_id,
        AuditAction::Updated,
        serde_json::json!({
            "name": name,
            "previousName": previous_name,
            "memberNumber": member_number,
            "previousMemberNumber": previous_number,
            "joinedAt": joined_at,
            "previousJoinedAt": previous_joined_at,
        }),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

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
    let member_number = input.member_number.trim();
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    check_member_number(&mut db, company_id, member_number, None).await?;
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(id, INSTR(id, '-M') + 2) AS INTEGER)), 0) + 1 FROM members WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    let id = format!("{company_id}-M{next:03}");
    let name = input.name.trim();
    sqlx::query(
        "INSERT INTO members (id, company_id, member_number, name, joined_at) VALUES (?, ?, ?, ?, ?)",
    )
        .bind(&id)
        .bind(company_id)
        .bind(member_number)
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
        serde_json::json!({"name": name, "memberNumber": member_number}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
