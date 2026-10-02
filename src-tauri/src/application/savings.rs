//! Application use cases for savings and member payments.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::posting::{
    append, audit, ensure_reference_unused, loan_balance, savings_balance, Component, Entry,
    APP_ACTOR,
};
use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, PaymentInput, SavingsTransactionInput},
    domain::{
        timestamp_id, validate_business_date, AuditAction, AuditEntityType, Channel, ComponentType,
        SavingsAccountType, SavingsMovement, TransactionDirection,
    },
};

pub(crate) async fn member_name(
    db: &mut SqliteConnection,
    company_id: &str,
    member_id: &str,
) -> Result<String, String> {
    sqlx::query_scalar("SELECT name FROM members WHERE company_id = ? AND id = ?")
        .bind(company_id)
        .bind(member_id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Anggota tidak ditemukan.".to_string())
}

pub(crate) async fn savings_account_id(
    db: &mut SqliteConnection,
    company_id: &str,
    member_id: &str,
    account_type: SavingsAccountType,
) -> Result<String, String> {
    sqlx::query_scalar(
        "SELECT id FROM savings_accounts WHERE company_id = ? AND member_id = ? AND account_type = ?",
    )
    .bind(company_id)
    .bind(member_id)
    .bind(account_type.as_str())
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Rekening simpanan tidak ditemukan.".to_string())
}

fn reference_or(input: &str, fallback: String) -> String {
    if input.trim().is_empty() {
        fallback
    } else {
        input.trim().to_string()
    }
}

pub(crate) async fn post_savings_transaction(
    input: SavingsTransactionInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.amount <= 0 {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }
    let account_type = SavingsAccountType::try_from(input.account_type.as_str())?;
    let movement = SavingsMovement::try_from(input.movement.as_str())?;
    let channel = Channel::try_from(input.channel.as_str())?;
    if movement == SavingsMovement::Withdrawal && account_type != SavingsAccountType::Voluntary {
        return Err("Hanya simpanan manasuka yang dapat ditarik pada versi ini.".into());
    }
    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name = member_name(&mut db, company_id, &input.member_id).await?;
    let account_id =
        savings_account_id(&mut db, company_id, &input.member_id, account_type).await?;
    let is_deposit = movement == SavingsMovement::Deposit;
    if !is_deposit && savings_balance(&mut db, company_id, &account_id).await? < input.amount {
        return Err("Saldo manasuka tidak mencukupi.".into());
    }

    let id = timestamp_id(if is_deposit { "SAV-IN" } else { "SAV-OUT" });
    let reference = reference_or(&input.reference, format!("KBS/SAV/{id}"));
    ensure_reference_unused(&mut db, company_id, &reference).await?;
    let (component_type, direction) = if is_deposit {
        (ComponentType::SavingsDeposit, TransactionDirection::In)
    } else {
        (ComponentType::SavingsWithdrawal, TransactionDirection::Out)
    };
    // Descriptions follow the daily cash ledger, e.g. a voluntary savings withdrawal by Hj Aisyah.
    let account_label = match account_type {
        SavingsAccountType::Principal => "Simpanan Pokok",
        SavingsAccountType::Mandatory => "Simpanan Wajib",
        SavingsAccountType::Voluntary => "Manasuka",
    };
    let description = if is_deposit {
        format!("Setoran {account_label} {member_name}")
    } else {
        format!("Pengambilan {account_label} {member_name}")
    };
    append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: Some(&input.member_id),
            member_name: &member_name,
            channel,
            description: &description,
            reference: &reference,
            direction,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components: vec![
                Component::new(component_type, &description, input.amount).for_savings(&account_id)
            ],
        },
    )
    .await?;
    audit(
        &mut db,
        company_id,
        AuditEntityType::Transaction,
        &id,
        AuditAction::Posted,
        serde_json::json!({"movement": movement.as_str(), "accountType": account_type.as_str(), "amount": input.amount, "channel": channel.as_str(), "reference": reference}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// Monthly member payment: principal installment + interest and mandatory/voluntary savings
/// in a single transaction.
pub(crate) async fn post_payment(
    input: PaymentInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if [
        input.principal,
        input.interest,
        input.wajib,
        input.voluntary,
    ]
    .iter()
    .any(|value| *value < 0)
    {
        return Err("Komponen pembayaran tidak boleh negatif.".into());
    }
    let channel = Channel::try_from(input.channel.as_str())?;
    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name = member_name(&mut db, company_id, &input.member_id).await?;
    let id = timestamp_id("TRX");
    let reference = reference_or(&input.reference, format!("KBS/RCPT/{id}"));
    ensure_reference_unused(&mut db, company_id, &reference).await?;

    // Principal is allocated to the oldest outstanding loan first.
    let mut components = Vec::new();
    let mut remaining_principal = input.principal;
    let mut first_loan: Option<String> = None;
    let loan_ids: Vec<String> = sqlx::query(
        "SELECT l.id FROM loans l JOIN loan_balances b ON b.company_id = l.company_id AND b.loan_id = l.id WHERE l.company_id = ? AND l.member_id = ? AND b.balance > 0 ORDER BY l.realization_date, l.id",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .fetch_all(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|row| row.try_get("id").map_err(|error| error.to_string()))
    .collect::<Result<_, String>>()?;
    for loan_id in loan_ids {
        if first_loan.is_none() {
            first_loan = Some(loan_id.clone());
        }
        if remaining_principal == 0 {
            break;
        }
        let allocated = remaining_principal.min(loan_balance(&mut db, company_id, &loan_id).await?);
        components.push(
            Component::new(ComponentType::LoanPrincipal, "Pokok pinjaman", allocated)
                .for_loan(&loan_id),
        );
        remaining_principal -= allocated;
    }
    if remaining_principal > 0 {
        return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
    }
    if input.interest > 0 {
        let loan_id = first_loan.ok_or("Anggota tidak memiliki pinjaman berjalan untuk bunga.")?;
        components.push(
            Component::new(
                ComponentType::LoanInterest,
                "Bunga pinjaman",
                input.interest,
            )
            .for_loan(loan_id),
        );
    }
    for (kind, label, value) in [
        (SavingsAccountType::Mandatory, "Simpanan wajib", input.wajib),
        (
            SavingsAccountType::Voluntary,
            "Simpanan manasuka",
            input.voluntary,
        ),
    ] {
        if value > 0 {
            let account_id =
                savings_account_id(&mut db, company_id, &input.member_id, kind).await?;
            components.push(
                Component::new(ComponentType::SavingsDeposit, label, value).for_savings(account_id),
            );
        }
    }

    let amount = append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: Some(&input.member_id),
            member_name: &member_name,
            channel,
            description: &format!("Setoran {member_name}"),
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
        &id,
        AuditAction::Posted,
        serde_json::json!({"amount": amount, "channel": channel.as_str(), "reference": reference}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
