//! Penulisan ke buku besar. Semua perpindahan uang masuk lewat sini sebagai
//! transaksi baru beserta komponennya; tidak ada saldo yang diubah di tempat.

use sqlx::SqliteConnection;

use crate::domain::{business_period, Channel, PeriodStatus, TransactionDirection};

pub(crate) const APP_ACTOR: &str = "Aplikasi lokal";

pub(crate) struct Component {
    pub(crate) component_type: &'static str,
    pub(crate) label: String,
    pub(crate) amount: i64,
    pub(crate) loan_id: Option<String>,
    pub(crate) savings_account_id: Option<String>,
}

impl Component {
    pub(crate) fn new(component_type: &'static str, label: impl Into<String>, amount: i64) -> Self {
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
    pub(crate) display_date: &'a str,
    pub(crate) display_time: &'a str,
    pub(crate) member_id: Option<&'a str>,
    pub(crate) member_name: &'a str,
    pub(crate) transaction_type: &'a str,
    pub(crate) channel: Channel,
    pub(crate) description: &'a str,
    pub(crate) reference: &'a str,
    pub(crate) direction: TransactionDirection,
    pub(crate) actor: &'a str,
    pub(crate) reversed_transaction_id: Option<&'a str>,
    pub(crate) components: Vec<Component>,
}

/// Menolak posting pada periode yang sudah dikunci.
pub(crate) async fn ensure_period_open(
    db: &mut SqliteConnection,
    company_id: &str,
    business_date: &str,
) -> Result<(), String> {
    let period = business_period(business_date)?;
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
    Ok(())
}

pub(crate) async fn ensure_reference_unused(
    db: &mut SqliteConnection,
    company_id: &str,
    reference: &str,
) -> Result<(), String> {
    let duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions WHERE company_id = ? AND reference = ?",
    )
    .bind(company_id)
    .bind(reference)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }
    Ok(())
}

pub(crate) async fn savings_balance(
    db: &mut SqliteConnection,
    company_id: &str,
    savings_account_id: &str,
) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT balance FROM savings_balances WHERE company_id = ? AND savings_account_id = ?",
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
    sqlx::query_scalar("SELECT balance FROM loan_balances WHERE company_id = ? AND loan_id = ?")
        .bind(company_id)
        .bind(loan_id)
        .fetch_optional(&mut *db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Pinjaman tidak ditemukan.".to_string())
}

/// Menulis satu transaksi. Komponen bernilai nol dilewati; jumlah transaksi
/// adalah total komponennya.
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
    sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, transaction_type, channel, description, reference, direction, amount, reversed_transaction_id, actor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&entry.id)
        .bind(company_id)
        .bind(entry.business_date)
        .bind(entry.display_date)
        .bind(entry.display_time)
        .bind(entry.member_id)
        .bind(entry.member_name)
        .bind(entry.transaction_type)
        .bind(entry.channel.as_str())
        .bind(entry.description)
        .bind(entry.reference)
        .bind(entry.direction.as_str())
        .bind(amount)
        .bind(entry.reversed_transaction_id)
        .bind(entry.actor)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    for component in components {
        sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(company_id)
            .bind(&entry.id)
            .bind(component.component_type)
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
    entity_type: &str,
    entity_id: &str,
    action: &str,
    after: serde_json::Value,
) -> Result<(), String> {
    sqlx::query("INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(company_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(action)
        .bind(serde_json::to_string(&after).unwrap_or_default())
        .bind(APP_ACTOR)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}
