//! Read model: semua saldo dihitung dari buku besar (lihat view di skema).

use std::collections::HashMap;

use sqlx::{sqlite::SqliteRow, Row, SqlitePool};

use crate::contracts::*;
use crate::domain::{
    Channel, LoanStatus, MemberStatus, SavingsAccountType, TransactionDirection, TransactionStatus,
};
// Keep recognizing transaction types already stored by older workbook imports.
const MIGRATED_SAVINGS: &str = "MIGRATED_SAVINGS";
const MIGRATED_LOAN: &str = "MIGRATED_LOAN";

macro_rules! try_column {
    ($row:expr, $column:literal, $ty:ty) => {
        $row.try_get::<$ty, _>($column)
            .map_err(|error| error.to_string())?
    };
    ($row:expr, $column:literal) => {
        $row.try_get($column).map_err(|error| error.to_string())?
    };
}

const TRANSACTION_COLUMNS: &str = "t.id, t.business_date, t.display_date, t.display_time, t.member_name, t.transaction_type, t.channel, t.description, t.reference, t.direction, t.amount, t.actor, (t.transaction_type = 'REVERSAL' OR EXISTS (SELECT 1 FROM transactions r WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id)) AS excluded, EXISTS (SELECT 1 FROM transactions r WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id) AS reversed";

/// Status turunan: transaksi yang sudah dibalik berstatus "Dibalik".
fn transaction_dto(
    row: &SqliteRow,
    components: &mut HashMap<String, Vec<ComponentDto>>,
) -> Result<TransactionDto, String> {
    let id: String = try_column!(row, "id");
    let reversed: bool = try_column!(row, "reversed");
    Ok(TransactionDto {
        components: components.remove(&id).unwrap_or_default(),
        id,
        date: try_column!(row, "display_date"),
        time: try_column!(row, "display_time"),
        member_name: try_column!(row, "member_name"),
        transaction_type: try_column!(row, "transaction_type"),
        channel: try_column!(row, "channel"),
        description: try_column!(row, "description"),
        reference: try_column!(row, "reference"),
        direction: TransactionDirection::try_from(try_column!(row, "direction", String).as_str())?,
        amount: try_column!(row, "amount"),
        status: if reversed {
            TransactionStatus::Reversed
        } else {
            TransactionStatus::Posted
        },
        actor: try_column!(row, "actor"),
    })
}

/// Komponen untuk sekumpulan transaksi dalam satu query.
async fn components_for(
    company_id: &str,
    filter_sql: &str,
    binds: &[&str],
    pool: &SqlitePool,
) -> Result<HashMap<String, Vec<ComponentDto>>, String> {
    let sql = format!(
        "SELECT c.transaction_id, c.label, c.amount FROM transaction_components c JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id WHERE c.company_id = ? AND {filter_sql} ORDER BY c.id"
    );
    let mut query = sqlx::query(&sql).bind(company_id);
    for value in binds {
        query = query.bind(*value);
    }
    let mut map: HashMap<String, Vec<ComponentDto>> = HashMap::new();
    for row in query
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?
    {
        let transaction_id: String = try_column!(row, "transaction_id");
        map.entry(transaction_id).or_default().push(ComponentDto {
            label: try_column!(row, "label"),
            amount: try_column!(row, "amount"),
        });
    }
    Ok(map)
}

async fn snapshot(company_id: &str, pool: &SqlitePool) -> Result<AppSnapshot, String> {
    let member_rows = sqlx::query(
        "SELECT m.id, m.name, m.joined_at, m.status, COALESCE(SUM(CASE WHEN b.account_type = ? THEN b.balance ELSE 0 END), 0) principal_savings FROM members m LEFT JOIN savings_balances b ON b.company_id = m.company_id AND b.member_id = m.id WHERE m.company_id = ? GROUP BY m.id ORDER BY m.name"
    ).bind(SavingsAccountType::Principal.as_str()).bind(company_id).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let members = member_rows
        .into_iter()
        .map(|row| -> Result<MemberDto, String> {
            Ok(MemberDto {
                id: try_column!(row, "id"),
                name: try_column!(row, "name"),
                joined_at: try_column!(row, "joined_at"),
                status: MemberStatus::try_from(try_column!(row, "status", String).as_str())?,
                principal_savings: try_column!(row, "principal_savings"),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let savings_balances = sqlx::query(
        "SELECT member_id, account_type, balance FROM savings_balances WHERE company_id = ? AND account_type IN (?, ?) ORDER BY member_id, account_type",
    )
    .bind(company_id)
    .bind(SavingsAccountType::Mandatory.as_str())
    .bind(SavingsAccountType::Voluntary.as_str())
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|row| -> Result<SavingsBalanceDto, String> {
        Ok(SavingsBalanceDto {
            member_id: try_column!(row, "member_id"),
            account_type: try_column!(row, "account_type"),
            balance: try_column!(row, "balance"),
        })
    })
    .collect::<Result<Vec<_>, _>>()?;

    // Lunas diturunkan dari saldo: pinjaman yang pernah bergerak dan saldonya nol.
    let loans = sqlx::query("SELECT l.id, COALESCE(l.member_id, '') member_id, l.member_name, l.plafond, b.balance, b.movements, l.rate_annual, l.tenor, l.interest_type, l.realization_date, l.due_date, l.guarantee, l.status FROM loans l JOIN loan_balances b ON b.company_id = l.company_id AND b.loan_id = l.id WHERE l.company_id = ? ORDER BY l.id")
        .bind(company_id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| -> Result<LoanDto, String> {
            let stored = LoanStatus::try_from(try_column!(row, "status", String).as_str())?;
            let balance: i64 = try_column!(row, "balance");
            let movements: i64 = try_column!(row, "movements");
            let status = if stored != LoanStatus::Draft && balance <= 0 && movements > 0 {
                LoanStatus::PaidOff
            } else {
                stored
            };
            Ok(LoanDto {
                id: try_column!(row, "id"),
                member_id: try_column!(row, "member_id"),
                member_name: try_column!(row, "member_name"),
                plafond: try_column!(row, "plafond"),
                balance,
                rate: try_column!(row, "rate_annual"),
                tenor: try_column!(row, "tenor"),
                interest_type: crate::domain::InterestType::try_from(
                    try_column!(row, "interest_type", String).as_str(),
                )?,
                realization_date: try_column!(row, "realization_date"),
                due_date: try_column!(row, "due_date"),
                guarantee: try_column!(row, "guarantee"),
                status,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Riwayat migrasi per anggota sangat banyak; daftar transaksi terbaru hanya
    // memuat transaksi operasional.
    let recent_filter = "t.transaction_type NOT IN (?, ?) AND t.channel != ?";
    let recent_binds = [MIGRATED_SAVINGS, MIGRATED_LOAN, Channel::NonCash.as_str()];
    let transaction_rows = sqlx::query(&format!(
        "SELECT {TRANSACTION_COLUMNS} FROM transactions t WHERE t.company_id = ? AND {recent_filter} ORDER BY t.business_date DESC, t.created_at DESC, t.id DESC LIMIT 120"
    ))
    .bind(company_id)
    .bind(recent_binds[0])
    .bind(recent_binds[1])
    .bind(recent_binds[2])
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let ids: Vec<String> = transaction_rows
        .iter()
        .map(|row| row.try_get("id").map_err(|error| error.to_string()))
        .collect::<Result<_, _>>()?;
    let placeholders = vec!["?"; ids.len().max(1)].join(", ");
    let id_refs: Vec<&str> = if ids.is_empty() {
        vec![""]
    } else {
        ids.iter().map(String::as_str).collect()
    };
    let mut components = components_for(
        company_id,
        &format!("t.id IN ({placeholders})"),
        &id_refs,
        pool,
    )
    .await?;
    let transactions = transaction_rows
        .iter()
        .map(|row| transaction_dto(row, &mut components))
        .collect::<Result<Vec<_>, _>>()?;

    let cash: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(CASE direction WHEN ? THEN amount ELSE -amount END), 0) FROM effective_transactions WHERE company_id = ? AND channel = ?")
        .bind(TransactionDirection::In.as_str()).bind(company_id).bind(Channel::Cash.as_str()).fetch_one(pool).await.map_err(|error| error.to_string())?;
    let savings: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance), 0) FROM savings_balances WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let loan_portfolio: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(b.balance), 0) FROM loan_balances b JOIN loans l ON l.company_id = b.company_id AND l.id = b.loan_id WHERE b.company_id = ? AND l.status != ? AND b.balance > 0",
    )
    .bind(company_id)
    .bind(LoanStatus::Draft.as_str())
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE company_id = ? AND status = ?")
            .bind(company_id)
            .bind(MemberStatus::Active.as_str())
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;

    Ok(AppSnapshot {
        members,
        savings_balances,
        loans,
        transactions,
        totals: TotalsDto {
            cash,
            savings,
            loan_portfolio,
            members: member_count,
        },
    })
}

pub(crate) async fn get_app_snapshot(
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    snapshot(company_id, pool).await
}

/// Buku kas per channel untuk satu tahun, dengan saldo berjalan seperti Buku
/// Kas Harian. Transaksi yang dibalik dan reversalnya tetap tampil tetapi
/// tidak mengubah saldo.
pub(crate) async fn get_cash_book(
    company_id: &str,
    year: i32,
    channel: &str,
    pool: &SqlitePool,
) -> Result<CashBookDto, String> {
    let channel = Channel::try_from(channel)?;
    let year_start = format!("{year:04}-01-01");
    let next_year_start = format!("{:04}-01-01", year + 1);
    let opening_balance: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(CASE direction WHEN ? THEN amount ELSE -amount END), 0) FROM effective_transactions WHERE company_id = ? AND channel = ? AND business_date < ?")
        .bind(TransactionDirection::In.as_str())
        .bind(company_id)
        .bind(channel.as_str())
        .bind(&year_start)
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())?;

    let filter = "t.channel = ? AND t.business_date >= ? AND t.business_date < ?";
    let rows = sqlx::query(&format!(
        "SELECT {TRANSACTION_COLUMNS} FROM transactions t WHERE t.company_id = ? AND {filter} ORDER BY t.business_date, t.rowid"
    ))
    .bind(company_id)
    .bind(channel.as_str())
    .bind(&year_start)
    .bind(&next_year_start)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let mut components = components_for(
        company_id,
        filter,
        &[channel.as_str(), &year_start, &next_year_start],
        pool,
    )
    .await?;

    let (mut balance, mut total_in, mut total_out) = (opening_balance, 0_i64, 0_i64);
    let mut cash_rows = Vec::with_capacity(rows.len());
    for row in &rows {
        let excluded: bool = try_column!(row, "excluded");
        let business_date: String = try_column!(row, "business_date");
        let transaction = transaction_dto(row, &mut components)?;
        if !excluded {
            if transaction.direction == TransactionDirection::In {
                balance += transaction.amount;
                total_in += transaction.amount;
            } else {
                balance -= transaction.amount;
                total_out += transaction.amount;
            }
        }
        cash_rows.push(CashBookRowDto {
            transaction,
            business_date,
            balance,
        });
    }
    Ok(CashBookDto {
        channel: channel.as_str().to_string(),
        year,
        opening_balance,
        total_in,
        total_out,
        closing_balance: balance,
        rows: cash_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_legacy_transactions_without_showing_monthly_imports_as_recent_activity() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            for (id, transaction_type, channel) in [
                ("legacy-savings", "MIGRATED_SAVINGS", "POTONGAN"),
                ("legacy-loan", "MIGRATED_LOAN", "POTONGAN"),
                ("legacy-cash", "MIGRATED_CASH", "KAS"),
                ("legacy-opening", "OPENING_SAVINGS", "NON_KAS"),
            ] {
                sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_name, transaction_type, channel, description, reference, direction, amount, actor) VALUES (?, 'default', '2026-09-01', '01 Sep 2026', '00:00', '-', ?, ?, 'Existing import', ?, 'Masuk', 1000, 'Import Excel 2026')")
                    .bind(id).bind(transaction_type).bind(channel).bind(id)
                    .execute(&pool).await.unwrap();
            }
            let snapshot = get_app_snapshot("default", &pool).await.unwrap();
            assert_eq!(snapshot.transactions.len(), 1);
            assert_eq!(snapshot.transactions[0].id, "legacy-cash");
            let book = get_cash_book("default", 2026, "KAS", &pool).await.unwrap();
            assert_eq!(book.rows.len(), 1);
            assert_eq!(book.rows[0].transaction.id, "legacy-cash");
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 4);
        });
    }
}
