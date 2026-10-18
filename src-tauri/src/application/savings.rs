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

/// The trimmed text the user typed, or `fallback` when it is empty.
pub(crate) fn trimmed_or(input: &str, fallback: String) -> String {
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
    let requested: Vec<(&str, i64)> = if input.amounts.is_empty() {
        vec![(input.account_type.as_str(), input.amount)]
    } else {
        input
            .amounts
            .iter()
            .map(|item| (item.account_type.as_str(), item.amount))
            .collect()
    };
    let mut amounts: Vec<(SavingsAccountType, i64)> = Vec::new();
    for (account_type, amount) in requested {
        let account_type = SavingsAccountType::try_from(account_type)?;
        if amount < 0 {
            return Err("Nominal transaksi tidak boleh negatif.".into());
        }
        if amounts.iter().any(|(other, _)| *other == account_type) {
            return Err("Jenis simpanan yang sama tercantum dua kali.".into());
        }
        if amount > 0 {
            amounts.push((account_type, amount));
        }
    }
    if amounts.is_empty() {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }
    // Pokok, Wajib, Manasuka: the order of the SIMPANAN columns.
    let order = |account_type: &SavingsAccountType| match account_type {
        SavingsAccountType::Principal => 0,
        SavingsAccountType::Mandatory => 1,
        SavingsAccountType::Voluntary => 2,
    };
    amounts.sort_by_key(|(account_type, _)| order(account_type));
    let movement = SavingsMovement::try_from(input.movement.as_str())?;
    let channel = Channel::try_from(input.channel.as_str())?;
    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name = member_name(&mut db, company_id, &input.member_id).await?;
    let is_deposit = movement == SavingsMovement::Deposit;
    let (component_type, direction) = if is_deposit {
        (ComponentType::SavingsDeposit, TransactionDirection::In)
    } else {
        (ComponentType::SavingsWithdrawal, TransactionDirection::Out)
    };
    // Descriptions follow the daily cash ledger, e.g. "Pengambilan Manasuka <nama>"
    // or "Setoran Wajib & Manasuka <nama>".
    let account_label = |account_type: &SavingsAccountType| match account_type {
        SavingsAccountType::Principal => "Simpanan Pokok",
        SavingsAccountType::Mandatory => "Simpanan Wajib",
        SavingsAccountType::Voluntary => "Manasuka",
    };
    let mut components = Vec::new();
    for (account_type, amount) in &amounts {
        let account_id =
            savings_account_id(&mut db, company_id, &input.member_id, *account_type).await?;
        if !is_deposit && savings_balance(&mut db, company_id, &account_id).await? < *amount {
            return Err(if amounts.len() == 1 {
                "Saldo simpanan tidak mencukupi.".into()
            } else {
                format!(
                    "Saldo {} tidak mencukupi.",
                    account_label(account_type).to_lowercase()
                )
            });
        }
        components.push(
            Component::new(component_type, account_label(account_type), *amount)
                .for_savings(&account_id),
        );
    }
    let accounts = if let [(account_type, _)] = amounts.as_slice() {
        account_label(account_type).to_string()
    } else {
        let names: Vec<&str> = amounts
            .iter()
            .map(|(account_type, _)| match account_type {
                SavingsAccountType::Principal => "Pokok",
                SavingsAccountType::Mandatory => "Wajib",
                SavingsAccountType::Voluntary => "Manasuka",
            })
            .collect();
        let (last, rest) = names.split_last().expect("at least two savings types");
        format!("Simpanan {} & {last}", rest.join(", "))
    };
    let description = trimmed_or(
        &input.description,
        if is_deposit {
            format!("Setoran {accounts} {member_name}")
        } else {
            format!("Pengambilan {accounts} {member_name}")
        },
    );

    let id = timestamp_id(if is_deposit { "SAV-IN" } else { "SAV-OUT" });
    let reference = trimmed_or(&input.reference, format!("KBS/SAV/{id}"));
    ensure_reference_unused(&mut db, company_id, &reference).await?;
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
            description: &description,
            reference: &reference,
            direction,
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
        serde_json::json!({
            "movement": movement.as_str(),
            "amounts": amounts.iter().map(|(account_type, amount)| serde_json::json!({"accountType": account_type.as_str(), "amount": amount})).collect::<Vec<_>>(),
            "amount": amount,
            "channel": channel.as_str(),
            "reference": reference,
        }),
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
    // A loan without a member is a Non Anggota installment; with a member, the
    // principal and interest go to that loan and the savings to the member.
    let loan_id = input.loan_id.trim();
    if !loan_id.is_empty() && input.member_id.trim().is_empty() {
        return post_loan_payment(input, channel, company_id, pool).await;
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name = member_name(&mut db, company_id, &input.member_id).await?;
    let id = timestamp_id("TRX");
    let reference = trimmed_or(&input.reference, format!("KBS/RCPT/{id}"));
    ensure_reference_unused(&mut db, company_id, &reference).await?;

    let mut components = Vec::new();
    if !input.loans.is_empty() {
        components = loan_lines(&input, company_id, &mut db).await?;
    }
    // Principal is allocated to the chosen loan, or else to the oldest
    // outstanding loan first.
    let mut remaining_principal = input.principal;
    let mut first_loan: Option<String> = None;
    let loan_ids: Vec<String> = if input.principal == 0 && input.interest == 0 {
        Vec::new()
    } else {
        sqlx::query(
        "SELECT l.id FROM loans l JOIN loan_balances b ON b.company_id = l.company_id AND b.loan_id = l.id WHERE l.company_id = ? AND l.member_id = ? AND b.balance > 0 AND (? = '' OR l.id = ?) ORDER BY l.realization_date, l.id",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .bind(loan_id)
    .bind(loan_id)
    .fetch_all(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|row| row.try_get("id").map_err(|error| error.to_string()))
    .collect::<Result<_, String>>()?
    };
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
            description: &trimmed_or(&input.description, format!("Setoran {member_name}")),
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

/// Principal and interest components for a payment that lists each loan, as
/// in a Buku Kas row like "Setoran Pinjaman <nama>" that pays two loans.
async fn loan_lines(
    input: &PaymentInput,
    company_id: &str,
    db: &mut SqliteConnection,
) -> Result<Vec<Component>, String> {
    if input.principal != 0 || input.interest != 0 || !input.loan_id.trim().is_empty() {
        return Err("Isi pokok dan bunga per pinjaman saja.".into());
    }
    let mut components = Vec::new();
    let mut seen = Vec::new();
    for line in &input.loans {
        let loan_id = line.loan_id.trim();
        if line.principal < 0 || line.interest < 0 {
            return Err("Komponen pembayaran tidak boleh negatif.".into());
        }
        if seen.contains(&loan_id) {
            return Err("Pinjaman yang sama tercantum dua kali.".into());
        }
        seen.push(loan_id);
        if line.principal == 0 && line.interest == 0 {
            continue;
        }
        let belongs: Option<String> = sqlx::query_scalar(
            "SELECT id FROM loans WHERE company_id = ? AND id = ? AND member_id = ?",
        )
        .bind(company_id)
        .bind(loan_id)
        .bind(&input.member_id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
        if belongs.is_none() {
            return Err("Pinjaman tidak ditemukan untuk anggota ini.".into());
        }
        if line.principal > loan_balance(db, company_id, loan_id).await? {
            return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
        }
        if line.principal > 0 {
            components.push(
                Component::new(
                    ComponentType::LoanPrincipal,
                    "Pokok pinjaman",
                    line.principal,
                )
                .for_loan(loan_id),
            );
        }
        if line.interest > 0 {
            components.push(
                Component::new(ComponentType::LoanInterest, "Bunga pinjaman", line.interest)
                    .for_loan(loan_id),
            );
        }
    }
    Ok(components)
}

/// Installment for one loan, as in the daily cash ledger rows for Non Anggota borrowers
/// (e.g. "Mas Roby 13 feb 25 15 juta"). Only principal and interest; there are no savings.
async fn post_loan_payment(
    input: PaymentInput,
    channel: Channel,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.wajib != 0 || input.voluntary != 0 {
        return Err("Pembayaran per pinjaman tidak memuat simpanan.".into());
    }
    let loan_id = input.loan_id.trim();
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let loan =
        sqlx::query("SELECT member_id, member_name FROM loans WHERE company_id = ? AND id = ?")
            .bind(company_id)
            .bind(loan_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Pinjaman tidak ditemukan.")?;
    let member_id: Option<String> = loan
        .try_get("member_id")
        .map_err(|error| error.to_string())?;
    let borrower_name: String = loan
        .try_get("member_name")
        .map_err(|error| error.to_string())?;
    if input.principal > loan_balance(&mut db, company_id, loan_id).await? {
        return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
    }
    let id = timestamp_id("TRX");
    let reference = trimmed_or(&input.reference, format!("KBS/RCPT/{id}"));
    ensure_reference_unused(&mut db, company_id, &reference).await?;
    let amount = append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            display_date: &input.display_date,
            display_time: &input.display_time,
            member_id: member_id.as_deref(),
            member_name: &borrower_name,
            channel,
            description: &trimmed_or(&input.description, format!("Setoran {borrower_name}")),
            reference: &reference,
            direction: TransactionDirection::In,
            actor: APP_ACTOR,
            reversed_transaction_id: None,
            components: vec![
                Component::new(
                    ComponentType::LoanPrincipal,
                    "Pokok pinjaman",
                    input.principal,
                )
                .for_loan(loan_id),
                Component::new(
                    ComponentType::LoanInterest,
                    "Bunga pinjaman",
                    input.interest,
                )
                .for_loan(loan_id),
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
        serde_json::json!({"amount": amount, "loanId": loan_id, "channel": channel.as_str(), "reference": reference}),
    )
    .await?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
