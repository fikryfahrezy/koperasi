//! Application use cases for savings.

use sqlx::{Row, SqlitePool};

use super::read_model::get_app_snapshot;
use crate::{
    contracts::{AppSnapshot, PaymentInput, SavingsTransactionInput},
    domain::{
        business_period, timestamp_id, LoanStatus, MemberStatus, PeriodStatus, SavingsAccountType,
        SavingsMovement, TransactionDirection, TransactionStatus,
    },
};

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
    if movement == SavingsMovement::Withdrawal && account_type != SavingsAccountType::Voluntary {
        return Err("Hanya simpanan manasuka yang dapat ditarik pada versi ini.".into());
    }
    let period = business_period(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM periods WHERE company_id = ? AND period = ? AND status = ?",
    )
    .bind(company_id)
    .bind(period)
    .bind(PeriodStatus::Locked.as_str())
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let member_name: String = sqlx::query_scalar(
        "SELECT name FROM members WHERE company_id = ? AND id = ? AND status = ?",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .bind(MemberStatus::Active.as_str())
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Anggota aktif tidak ditemukan.")?;
    let account = sqlx::query(
        "SELECT id, balance FROM savings_accounts WHERE company_id = ? AND member_id = ? AND account_type = ?",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .bind(account_type.as_str())
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    let account_id: String = account.try_get("id").map_err(|error| error.to_string())?;
    let balance: i64 = account
        .try_get("balance")
        .map_err(|error| error.to_string())?;
    if movement == SavingsMovement::Withdrawal && balance < input.amount {
        return Err("Saldo manasuka tidak mencukupi.".into());
    }

    let id = timestamp_id(if movement == SavingsMovement::Deposit {
        "SAV-IN"
    } else {
        "SAV-OUT"
    });
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/SAV/{id}")
    } else {
        input.reference.trim().to_string()
    };
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions WHERE company_id = ? AND reference = ?",
    )
    .bind(company_id)
    .bind(&reference)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }
    let is_deposit = movement == SavingsMovement::Deposit;
    let direction = if is_deposit {
        TransactionDirection::In
    } else {
        TransactionDirection::Out
    };
    let transaction_type = if is_deposit {
        "SAVINGS_DEPOSIT"
    } else {
        "SAVINGS_WITHDRAWAL"
    };
    let component_type = transaction_type;
    let description = format!(
        "{} simpanan {}",
        movement.as_str(),
        account_type.as_str().to_lowercase()
    );
    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'Aplikasi lokal')")
        .bind(&id)
        .bind(company_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&input.member_id)
        .bind(&member_name)
        .bind(transaction_type)
        .bind(&description)
        .bind(&reference)
        .bind(direction.as_str())
        .bind(input.amount)
        .bind(TransactionStatus::Posted.as_str())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(company_id)
        .bind(&id)
        .bind(component_type)
        .bind(&description)
        .bind(input.amount)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let new_balance = if is_deposit {
        balance
            .checked_add(input.amount)
            .ok_or("Saldo rekening terlalu besar.")?
    } else {
        balance - input.amount
    };
    sqlx::query("UPDATE savings_accounts SET balance = ? WHERE company_id = ? AND id = ?")
        .bind(new_balance)
        .bind(company_id)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)")
        .bind(company_id)
        .bind(&id)
        .bind(direction.as_str())
        .bind(input.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(company_id)
        .bind(&id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": movement.as_str(), "accountType": account_type.as_str(), "amount": input.amount, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}

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
    let amount = input
        .principal
        .checked_add(input.interest)
        .and_then(|value| value.checked_add(input.wajib))
        .and_then(|value| value.checked_add(input.voluntary))
        .ok_or("Total pembayaran terlalu besar.")?;
    if amount <= 0 {
        return Err("Total pembayaran harus lebih dari nol.".into());
    }
    let period = business_period(&input.business_date)?;
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM periods WHERE company_id = ? AND period = ? AND status = ?",
    )
    .bind(company_id)
    .bind(period)
    .bind(PeriodStatus::Locked.as_str())
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let member_name: String = sqlx::query_scalar(
        "SELECT name FROM members WHERE company_id = ? AND id = ? AND status = ?",
    )
    .bind(company_id)
    .bind(&input.member_id)
    .bind(MemberStatus::Active.as_str())
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Anggota aktif tidak ditemukan.")?;
    let id = timestamp_id("TRX");
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/RCPT/{id}")
    } else {
        input.reference.trim().to_string()
    };
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions WHERE company_id = ? AND reference = ?",
    )
    .bind(company_id)
    .bind(&reference)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }

    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, 'MEMBER_PAYMENT', 'Pembayaran anggota', ?, ?, ?, ?, 'Aplikasi lokal')")
        .bind(&id).bind(company_id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(&input.member_id).bind(&member_name).bind(&reference).bind(TransactionDirection::In.as_str()).bind(amount).bind(TransactionStatus::Posted.as_str()).execute(&mut *db).await.map_err(|error| error.to_string())?;

    let mut remaining_principal = input.principal;
    let loan_rows = sqlx::query("SELECT id, balance FROM loans WHERE company_id = ? AND member_id = ? AND status = ? AND balance > 0 ORDER BY realization_date, id")
        .bind(company_id).bind(&input.member_id).bind(LoanStatus::Active.as_str()).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for loan in loan_rows {
        if remaining_principal == 0 {
            break;
        }
        let loan_id: String = loan.try_get("id").map_err(|error| error.to_string())?;
        let balance: i64 = loan.try_get("balance").map_err(|error| error.to_string())?;
        let allocated = remaining_principal.min(balance);
        sqlx::query("UPDATE loans SET balance = balance - ?, status = CASE WHEN balance - ? = 0 THEN ? ELSE status END WHERE company_id = ? AND id = ?")
            .bind(allocated).bind(allocated).bind(LoanStatus::PaidOff.as_str()).bind(company_id).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES (?, ?, 'LOAN_PRINCIPAL', 'Pokok pinjaman', ?, ?)")
            .bind(company_id).bind(&id).bind(allocated).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        remaining_principal -= allocated;
    }
    if remaining_principal > 0 {
        return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
    }

    if input.interest > 0 {
        sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount) VALUES (?, ?, 'LOAN_INTEREST', 'Bunga pinjaman', ?)").bind(company_id).bind(&id).bind(input.interest).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    for (kind, component_type, label, value) in [
        (
            SavingsAccountType::Mandatory,
            "SAVINGS_WAJIB",
            "Simpanan wajib",
            input.wajib,
        ),
        (
            SavingsAccountType::Voluntary,
            "SAVINGS_VOLUNTARY",
            "Simpanan manasuka",
            input.voluntary,
        ),
    ] {
        if value > 0 {
            let account_id: String = sqlx::query_scalar(
                "SELECT id FROM savings_accounts WHERE company_id = ? AND member_id = ? AND account_type = ?",
            )
            .bind(company_id)
            .bind(&input.member_id)
            .bind(kind.as_str())
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
            let result = sqlx::query(
                "UPDATE savings_accounts SET balance = balance + ? WHERE company_id = ? AND id = ? AND balance <= ?",
            )
            .bind(value)
            .bind(company_id)
            .bind(&account_id)
            .bind(i64::MAX - value)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
            if result.rows_affected() != 1 {
                return Err("Saldo rekening terlalu besar atau rekening tidak ditemukan.".into());
            }
            sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?, ?)").bind(company_id).bind(&id).bind(component_type).bind(label).bind(value).bind(account_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        }
    }
    sqlx::query(
        "INSERT INTO cash_postings (company_id, transaction_id, direction, amount) VALUES (?, ?, ?, ?)",
    )
    .bind(company_id)
    .bind(&id)
    .bind(TransactionDirection::In.as_str())
    .bind(amount)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, 'TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')").bind(company_id).bind(&id).bind(serde_json::to_string(&serde_json::json!({"amount": amount, "reference": reference})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    get_app_snapshot(company_id, pool).await
}
