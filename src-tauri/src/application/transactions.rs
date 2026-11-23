//! Application use cases for transactions: reversals and general cash entries.

use sqlx::{Row, SqliteConnection, SqlitePool};

use super::posting::{append, audit, loan_balance, savings_balance, Component, Entry};

use super::read_model::get_app_snapshot;
use super::transaction_groups::deletion_set;

use crate::{
    contracts::{AppSnapshot, CashEntryInput, ReverseTransactionInput},
    domain::{
        cash_category, timestamp_id, validate_business_date, AuditAction, AuditEntityType, Channel,
        ComponentType, TransactionDirection,
    },
};

/// Correction without modifying data: a new transaction in the opposite direction references
/// the original transaction. Both are then excluded from balance calculations. A transaction
/// that belongs to a group (one member's imported month) is reversed with its whole group.
pub(crate) async fn reverse_transaction(
    input: ReverseTransactionInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let ids = deletion_set(&mut db, company_id, &input.id).await?;

    let stamp = Stamp {
        business_date: &input.business_date,
    };

    for id in &ids {
        reverse_one(&mut db, company_id, id, &stamp).await?;
    }

    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// When a reversal is recorded.
struct Stamp<'a> {
    business_date: &'a str,
}

/// The transaction itself, or every transaction still in effect in its group, in
/// ledger order. Hapus narrows this down with `deletion_set`.
pub(crate) async fn group_members(
    db: &mut SqliteConnection,
    company_id: &str,
    id: &str,
) -> Result<Vec<String>, String> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT t.id
         FROM transactions t
         WHERE t.company_id = ?
           AND t.id IN (
               SELECT e.id
               FROM effective_transactions e
               WHERE e.company_id = t.company_id
           )
           AND (
               t.id = ?
               OR t.id IN (
                   SELECT g.transaction_id
                   FROM transaction_groups g
                   WHERE g.company_id = ?
                     AND g.group_id = (
                         SELECT group_id
                         FROM transaction_groups
                         WHERE company_id = ? AND transaction_id = ?
                     )
               )
           )
         ORDER BY t.business_date, t.rowid",
    )
    .bind(company_id)
    .bind(id)
    .bind(company_id)
    .bind(company_id)
    .bind(id)
    .fetch_all(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    if !ids.iter().any(|candidate| candidate == id) {
        return Err("Transaksi tidak ditemukan atau sudah dihapus.".into());
    }

    Ok(ids)
}

/// Appends the reversal of one transaction; a reversal that would make a balance negative
/// is refused.
async fn reverse_one(
    db: &mut SqliteConnection,
    company_id: &str,
    id: &str,
    stamp: &Stamp<'_>,
) -> Result<String, String> {
    let original = sqlx::query(
        "SELECT member_id, member_name, channel, description, direction
         FROM effective_transactions
         WHERE company_id = ? AND id = ?",
    )
    .bind(company_id)
    .bind(id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Transaksi tidak ditemukan atau sudah dihapus.")?;

    let get = |column: &str| -> Result<String, String> {
        original.try_get(column).map_err(|error| error.to_string())
    };

    let member_id: Option<String> = original
        .try_get("member_id")
        .map_err(|error| error.to_string())?;

    let channel = Channel::try_from(get("channel")?.as_str())?;
    let direction = TransactionDirection::try_from(get("direction")?.as_str())?.reversed();

    let mut components = Vec::new();

    for component in sqlx::query(
        "SELECT component_type, label, amount, loan_id, savings_account_id
         FROM transaction_components
         WHERE company_id = ? AND transaction_id = ?
         ORDER BY id",
    )
    .bind(company_id)
    .bind(id)
    .fetch_all(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    {
        let component_type: String = component
            .try_get("component_type")
            .map_err(|error| error.to_string())?;

        let component_type = ComponentType::try_from(component_type.as_str())?;

        let label: String = component
            .try_get("label")
            .map_err(|error| error.to_string())?;

        let amount: i64 = component
            .try_get("amount")
            .map_err(|error| error.to_string())?;

        let loan_id: Option<String> = component
            .try_get("loan_id")
            .map_err(|error| error.to_string())?;

        let savings_account_id: Option<String> = component
            .try_get("savings_account_id")
            .map_err(|error| error.to_string())?;

        // Reversing an inflow must not make the balance negative.
        if let Some(account_id) = &savings_account_id {
            if component_type != ComponentType::SavingsWithdrawal
                && savings_balance(&mut *db, company_id, account_id).await? < amount
            {
                return Err(
                    "Transaksi tidak dapat dihapus karena saldo simpanan tidak mencukupi.".into(),
                );
            }
        }

        if let Some(loan_id) = &loan_id {
            if matches!(
                component_type,
                ComponentType::LoanDisbursement | ComponentType::LoanOpening
            ) && loan_balance(&mut *db, company_id, loan_id).await? < amount
            {
                return Err(
                    "Transaksi tidak dapat dihapus karena pinjaman sudah memiliki angsuran.".into(),
                );
            }
        }

        let mut reversal =
            Component::new(ComponentType::Reversal, format!("Hapus · {label}"), amount);

        reversal.loan_id = loan_id;
        reversal.savings_account_id = savings_account_id;
        components.push(reversal);
    }

    let reversal_id = timestamp_id("REV");
    let description = format!("Hapus · {}", get("description")?);

    append(
        &mut *db,
        company_id,
        Entry {
            id: reversal_id.clone(),
            business_date: stamp.business_date,
            member_id: member_id.as_deref(),
            member_name: &get("member_name")?,
            channel,
            description: &description,
            direction,
            reversed_transaction_id: Some(id),
            components,
        },
    )
    .await?;

    audit(
        &mut *db,
        company_id,
        AuditEntityType::Transaction,
        id,
        AuditAction::Reversed,
        serde_json::json!({"reversalId": reversal_id}),
    )
    .await?;

    Ok(reversal_id)
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

    let direction = TransactionDirection::try_from(input.direction.as_str())?;
    let channel = Channel::try_from(input.channel.as_str())?;

    // Optional Buku Kas category column; rows without one are plain cash movements.
    // Kas Buku Tabungan movements have no category.
    let (component_type, label) = match input.category.trim() {
        "" if channel == Channel::SavingsBook => match direction {
            TransactionDirection::In => (ComponentType::SavingsBookIn, "Tabungan masuk"),
            TransactionDirection::Out => (ComponentType::SavingsBookOut, "Tabungan keluar"),
        },
        _ if channel == Channel::SavingsBook => {
            return Err("Mutasi Kas Buku Tabungan tidak memakai kategori.".into())
        }
        "" => (ComponentType::Cash, "Mutasi kas"),
        code => {
            let component_type = ComponentType::try_from(code)?;
            let (label, only) =
                cash_category(component_type).ok_or("Kategori Buku Kas tidak valid.")?;

            match only {
                Some(TransactionDirection::In) if direction != TransactionDirection::In => {
                    return Err(format!("{label} hanya untuk kas masuk."))
                }
                Some(TransactionDirection::Out) if direction != TransactionDirection::Out => {
                    return Err(format!("{label} hanya untuk kas keluar."))
                }
                _ => {}
            }

            (component_type, label)
        }
    };

    match (component_type, direction) {
        (
            ComponentType::CashTopUp | ComponentType::CashTopUpFromSavingsBook,
            TransactionDirection::Out,
        ) => return Err("Tambahan Kas hanya untuk kas masuk.".into()),
        (ComponentType::TransferToSavingsBook, TransactionDirection::In) => {
            return Err("Transfer ke Tabungan hanya untuk kas keluar.".into())
        }
        _ => {}
    }

    validate_business_date(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let id = timestamp_id("CASH");
    let description = input.description.trim();

    if description.is_empty() {
        return Err("Uraian wajib diisi.".into());
    }

    if !input.bon_payments.is_empty() {
        if component_type != ComponentType::CashBondRepayment {
            return Err("Bon yang dibayar hanya untuk Bayar Kas Bon.".into());
        }

        if input.bon_payments.iter().map(|part| part.amount).sum::<i64>() != input.amount {
            return Err("Jumlah harus sama dengan total yang dibayar per bon.".into());
        }

        for (index, part) in input.bon_payments.iter().enumerate() {
            if part.amount <= 0 {
                return Err("Jumlah yang dibayar per bon harus lebih dari nol.".into());
            }

            if input.bon_payments[..index]
                .iter()
                .any(|earlier| earlier.bon_key == part.bon_key)
            {
                return Err("Satu bon hanya boleh muncul sekali.".into());
            }

            let left =
                bon_left(&mut db, company_id, &input.business_date, &part.bon_key).await?;

            if part.amount > left {
                return Err(format!(
                    "Jumlah melebihi sisa bon ({}).",
                    thousands(left)
                ));
            }
        }
    }

    append(
        &mut db,
        company_id,
        Entry {
            id: id.clone(),
            business_date: &input.business_date,
            member_id: None,
            member_name: "Koperasi",
            channel,
            description,
            direction,
            reversed_transaction_id: None,
            components: vec![Component::new(component_type, label, input.amount)],
        },
    )
    .await?;

    for part in &input.bon_payments {
        sqlx::query(
            "INSERT INTO cash_bond_payments (company_id, transaction_id, bon_key, amount)
             VALUES (?, ?, ?, ?)",
        )
        .bind(company_id)
        .bind(&id)
        .bind(&part.bon_key)
        .bind(part.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    audit(
        &mut db,
        company_id,
        AuditEntityType::Transaction,
        &id,
        AuditAction::Posted,
        serde_json::json!({
            "category": label,
            "amount": input.amount,
            "channel": channel.as_str(),
            "bonPayments": &input.bon_payments,
        }),
    )
    .await?;

    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

/// What is left of a bon of Piutang Lainnya on its book year's list (the year
/// of `business_date`): its amount less the Bayar Kas Bon of that year linked
/// to it. A Pengurang of the list pays no bon in particular.
async fn bon_left(
    db: &mut SqliteConnection,
    company_id: &str,
    business_date: &str,
    bon_key: &str,
) -> Result<i64, String> {
    let year: i32 = business_date
        .get(..4)
        .and_then(|year| year.parse().ok())
        .ok_or("Tanggal transaksi tidak valid.")?;

    let amount: Option<i64> = sqlx::query_scalar(
        "SELECT amount
         FROM balance_sheet_details
         WHERE company_id = ? AND year = ? AND item_key = 'OTHER_LOANS' AND bon_key = ?",
    )
    .bind(company_id)
    .bind(year)
    .bind(bon_key)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    let amount = amount.ok_or(format!(
        "Bon yang dipilih tidak ada di Piutang Lainnya {year}."
    ))?;

    let repaid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(p.amount), 0)
         FROM cash_bond_payments p
         JOIN effective_transactions t
           ON t.company_id = p.company_id AND t.id = p.transaction_id
         WHERE p.company_id = ? AND p.bon_key = ?
           AND t.business_date >= ? AND t.business_date < ?",
    )
    .bind(company_id)
    .bind(bon_key)
    .bind(format!("{year:04}-01-01"))
    .bind(format!("{:04}-01-01", year + 1))
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(amount - repaid)
}

/// 1500000 as "1.500.000".
fn thousands(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let mut grouped = String::new();

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }

    if amount < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}
