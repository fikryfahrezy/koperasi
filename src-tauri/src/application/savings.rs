//! Application use cases for savings and member payments.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::posting::{append, audit, loan_balance, savings_balance, Component, Entry};

use super::read_model::get_app_snapshot;

use crate::{
    contracts::{AppSnapshot, PaymentInput, SavingsAmountInput, SavingsTransactionInput},
    domain::{
        timestamp_id, validate_business_date, AuditAction, AuditEntityType, Channel, ComponentType,
        LoanGroup, SavingsAccountType, SavingsMovement, TransactionDirection,
    },
};

pub(crate) async fn member_name(
    db: &mut SqliteConnection,
    company_id: &str,
    member_id: &str,
) -> Result<String, String> {
    sqlx::query_scalar(
        "SELECT name
         FROM members
         WHERE company_id = ? AND id = ?",
    )
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
        "SELECT id
         FROM savings_accounts
         WHERE company_id = ? AND member_id = ? AND account_type = ?",
    )
    .bind(company_id)
    .bind(member_id)
    .bind(account_type.as_str())
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "Rekening simpanan tidak ditemukan.".to_string())
}

/// The Manasuka account of a penabung Non Anggota by name (case-insensitive),
/// with the name as recorded. A deposit under a new name opens the account.
pub(crate) async fn non_member_account(
    db: &mut SqliteConnection,
    company_id: &str,
    name: &str,
    open: bool,
) -> Result<(String, String), String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");

    if name.is_empty() {
        return Err("Pilih anggota atau isi nama penabung non anggota.".into());
    }

    let found = sqlx::query(
        "SELECT id, holder_name
         FROM savings_accounts
         WHERE company_id = ? AND member_id IS NULL AND holder_name = ? COLLATE NOCASE",
    )
    .bind(company_id)
    .bind(&name)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    if let Some(row) = found {
        return Ok((
            row.try_get("id").map_err(|error| error.to_string())?,
            row.try_get("holder_name")
                .map_err(|error| error.to_string())?,
        ));
    }

    if !open {
        return Err("Penabung non anggota tidak ditemukan.".into());
    }

    // Member names as stored may hold extra spaces; compare them the way the
    // saver's name was cleaned.
    let normalized = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let wanted = normalized(&name);

    let taken = sqlx::query_scalar::<_, String>("SELECT name FROM members WHERE company_id = ?")
        .bind(company_id)
        .fetch_all(&mut *db)
        .await
        .map_err(|error| error.to_string())?
        .iter()
        .any(|member| normalized(member) == wanted);

    if taken {
        return Err(format!(
            "{name} terdaftar sebagai anggota; pilih anggotanya, bukan penabung non anggota."
        ));
    }

    let id = timestamp_id("SA-NA");

    sqlx::query(
        "INSERT INTO savings_accounts (id, company_id, member_id, holder_name, account_type)
         VALUES (?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(company_id)
    .bind(&name)
    .bind(SavingsAccountType::Voluntary.as_str())
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    Ok((id, name))
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
    let is_deposit = movement == SavingsMovement::Deposit;
    // A penabung Non Anggota has no member, only a Manasuka account.
    let non_member = input.member_id.trim().is_empty();

    let (member_name, non_member_account_id) = if non_member {
        if amounts
            .iter()
            .any(|(account_type, _)| *account_type != SavingsAccountType::Voluntary)
        {
            return Err("Penabung non anggota hanya punya simpanan manasuka.".into());
        }

        let (id, name) =
            non_member_account(&mut db, company_id, &input.saver_name, is_deposit).await?;
        (name, Some(id))
    } else {
        (
            member_name(&mut db, company_id, &input.member_id).await?,
            None,
        )
    };

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
        let account_id = match &non_member_account_id {
            Some(id) => id.clone(),
            None => {
                savings_account_id(&mut db, company_id, &input.member_id, *account_type).await?
            }
        };

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
    let amount = append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            member_id: (!non_member).then_some(input.member_id.as_str()),
            member_name: &member_name,
            channel,
            description: &description,
            direction,
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
        }),
    )
    .await?;

    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// Pengambilan simpanan: the only way savings leave the koperasi. Deposits go
/// through `post_payment`, with or without a loan installment.
pub(crate) async fn post_savings_withdrawal(
    input: SavingsTransactionInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let input = SavingsTransactionInput {
        movement: SavingsMovement::Withdrawal.as_str().into(),
        ..input
    };

    post_savings_transaction(input, company_id, pool).await
}

/// Monthly member payment: principal installment + interest and principal/mandatory/voluntary
/// savings in a single transaction. A payment of savings only is posted as a
/// Setoran simpanan (e.g. "Setoran Simpanan Wajib & Manasuka <nama>"), as the
/// Buku Kas writes it.
pub(crate) async fn post_payment(
    input: PaymentInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if [
        input.principal,
        input.interest,
        input.pokok,
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

    if (!loan_id.is_empty() || !input.loans.is_empty()) && input.member_id.trim().is_empty() {
        return post_loan_payment(input, channel, company_id, pool).await;
    }

    let pays_loan = !loan_id.is_empty()
        || input
            .loans
            .iter()
            .any(|line| line.principal != 0 || line.interest != 0)
        || input.principal != 0
        || input.interest != 0;

    if !pays_loan {
        let amounts = [
            (SavingsAccountType::Principal, input.pokok),
            (SavingsAccountType::Mandatory, input.wajib),
            (SavingsAccountType::Voluntary, input.voluntary),
        ]
        .into_iter()
        .filter(|(_, amount)| *amount > 0)
        .map(|(account_type, amount)| SavingsAmountInput {
            account_type: account_type.as_str().into(),
            amount,
        })
        .collect();

        let deposit = SavingsTransactionInput {
            member_id: input.member_id,
            saver_name: input.saver_name,
            account_type: String::new(),
            movement: SavingsMovement::Deposit.as_str().into(),
            amount: 0,
            amounts,
            description: input.description,
            channel: input.channel,
            business_date: input.business_date,
        };

        return post_savings_transaction(deposit, company_id, pool).await;
    }

    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let member_name = member_name(&mut db, company_id, &input.member_id).await?;
    let id = timestamp_id("TRX");
    let mut components = Vec::new();

    if !input.loans.is_empty() {
        components = loan_lines(
            &input,
            LoanOwner::Member(&input.member_id),
            company_id,
            &mut db,
        )
        .await?;
    }

    // Principal is allocated to the chosen loan, or else to the oldest
    // outstanding loan first.
    let mut remaining_principal = input.principal;
    let mut first_loan: Option<String> = None;

    let loan_ids: Vec<String> = if input.principal == 0 && input.interest == 0 {
        Vec::new()
    } else {
        sqlx::query(
            "SELECT l.id
             FROM loans l
             JOIN loan_balances b ON b.company_id = l.company_id AND b.loan_id = l.id
             WHERE l.company_id = ? AND l.member_id = ? AND b.balance > 0 AND (? = '' OR l.id = ?)
             ORDER BY l.realization_date, l.id",
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
        (SavingsAccountType::Principal, "Simpanan pokok", input.pokok),
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
            member_id: Some(&input.member_id),
            member_name: &member_name,
            channel,
            description: &trimmed_or(&input.description, format!("Setoran {member_name}")),
            direction: TransactionDirection::In,
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
        serde_json::json!({"amount": amount, "channel": channel.as_str()}),
    )
    .await?;

    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// Whose loans a per-loan payment may list: a member's, or a Non Anggota
/// borrower's, matched by name as on the loan form (case and outer spaces
/// ignored), since Non Anggota borrowers have no member record.
enum LoanOwner<'a> {
    Member(&'a str),
    NonMember(&'a str),
}

/// Principal and interest components for a payment that lists each loan, as
/// in a Buku Kas row like "Setoran Pinjaman <nama>" that pays two loans.
async fn loan_lines(
    input: &PaymentInput,
    owner: LoanOwner<'_>,
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

        let belongs: Option<String> = match owner {
            LoanOwner::Member(member_id) => sqlx::query_scalar(
                "SELECT id
                 FROM loans
                 WHERE company_id = ? AND id = ? AND member_id = ?",
            )
            .bind(company_id)
            .bind(loan_id)
            .bind(member_id),
            LoanOwner::NonMember(name) => sqlx::query_scalar(
                "SELECT id
                 FROM loans
                 WHERE company_id = ?
                   AND id = ?
                   AND loan_group = ?
                   AND COALESCE(member_id, '') = ''
                   AND lower(trim(member_name)) = lower(trim(?))",
            )
            .bind(company_id)
            .bind(loan_id)
            .bind(LoanGroup::NonMember.as_str())
            .bind(name),
        }
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?;

        if belongs.is_none() {
            return Err(match owner {
                LoanOwner::Member(_) => "Pinjaman tidak ditemukan untuk anggota ini.".into(),
                LoanOwner::NonMember(_) => "Pinjaman tidak ditemukan untuk peminjam ini.".into(),
            });
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
/// (e.g. "Mas <nama> 13 feb 25 15 juta"), or for each running loan of one borrower in a
/// single row (e.g. "Setoran <nama>"). Only principal and interest; there are no savings.
async fn post_loan_payment(
    input: PaymentInput,
    channel: Channel,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    if input.wajib != 0 || input.voluntary != 0 {
        return Err("Pembayaran per pinjaman tidak memuat simpanan.".into());
    }

    // Paying several loans: the borrower is the one of the first listed loan.
    let loan_id = match input.loans.first() {
        Some(line) => line.loan_id.trim(),
        None => input.loan_id.trim(),
    };

    let mut db = pool.begin().await.map_err(|error| error.to_string())?;

    let loan = sqlx::query(
        // A member's loan is paid under the member's current name.
        "SELECT
             l.member_id,
             COALESCE(
                 (SELECT m.name FROM members m WHERE m.company_id = l.company_id AND m.id = l.member_id),
                 l.member_name
             ) AS member_name
         FROM loans l
         WHERE l.company_id = ? AND l.id = ?",
    )
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

    let components = if input.loans.is_empty() {
        if input.principal > loan_balance(&mut db, company_id, loan_id).await? {
            return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
        }

        vec![
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
        ]
    } else {
        loan_lines(
            &input,
            LoanOwner::NonMember(&borrower_name),
            company_id,
            &mut db,
        )
        .await?
    };

    let id = timestamp_id("TRX");
    let amount = append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            member_id: member_id.as_deref(),
            member_name: &borrower_name,
            channel,
            description: &trimmed_or(&input.description, format!("Setoran {borrower_name}")),
            direction: TransactionDirection::In,
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
        if input.loans.is_empty() {
            serde_json::json!({"amount": amount, "loanId": loan_id, "channel": channel.as_str()})
        } else {
            let loan_ids: Vec<&str> = input.loans.iter().map(|line| line.loan_id.trim()).collect();
            serde_json::json!({"amount": amount, "loanIds": loan_ids, "channel": channel.as_str()})
        },
    )
    .await?;

    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
