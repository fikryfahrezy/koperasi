//! Application use cases for read model.

use sqlx::{Row, SqlitePool};

use crate::contracts::*;
use crate::domain::{LoanStatus, MemberStatus, SavingsAccountType, TransactionDirection};

macro_rules! try_column {
    ($row:expr, $column:literal, $ty:ty) => {
        $row.try_get::<$ty, _>($column)
            .map_err(|error| error.to_string())?
    };
    ($row:expr, $column:literal) => {
        $row.try_get($column).map_err(|error| error.to_string())?
    };
}

async fn snapshot(company_id: &str, pool: &SqlitePool) -> Result<AppSnapshot, String> {
    let member_rows = sqlx::query(
        "SELECT m.id, m.member_number, m.name, m.joined_at, m.status, COALESCE(SUM(sa.balance), 0) savings, COALESCE(SUM(CASE WHEN sa.account_type = ? THEN sa.balance ELSE 0 END), 0) principal_savings, COALESCE(SUM(CASE WHEN sa.account_type = ? THEN sa.balance ELSE 0 END), 0) mandatory_savings, COALESCE(SUM(CASE WHEN sa.account_type = ? THEN sa.balance ELSE 0 END), 0) voluntary_savings, COALESCE((SELECT SUM(l.balance) FROM loans l WHERE l.company_id = m.company_id AND l.member_id = m.id AND l.status IN (?, ?)), 0) loan_balance FROM members m LEFT JOIN savings_accounts sa ON sa.company_id = m.company_id AND sa.member_id = m.id WHERE m.company_id = ? GROUP BY m.id ORDER BY m.name"
    ).bind(SavingsAccountType::Principal.as_str()).bind(SavingsAccountType::Mandatory.as_str()).bind(SavingsAccountType::Voluntary.as_str()).bind(LoanStatus::Active.as_str()).bind(LoanStatus::NeedsReview.as_str()).bind(company_id).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let members = member_rows
        .into_iter()
        .map(|row| -> Result<MemberDto, String> {
            Ok(MemberDto {
                id: try_column!(row, "id"),
                member_number: try_column!(row, "member_number"),
                name: try_column!(row, "name"),
                joined_at: try_column!(row, "joined_at"),
                status: MemberStatus::try_from(try_column!(row, "status", String).as_str())?,
                savings: try_column!(row, "savings"),
                principal_savings: try_column!(row, "principal_savings"),
                mandatory_savings: try_column!(row, "mandatory_savings"),
                voluntary_savings: try_column!(row, "voluntary_savings"),
                loan_balance: try_column!(row, "loan_balance"),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let loan_rows = sqlx::query("SELECT id, COALESCE(member_id, '') member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status FROM loans WHERE company_id = ? ORDER BY CASE status WHEN ? THEN 0 WHEN ? THEN 1 WHEN ? THEN 2 ELSE 3 END, id LIMIT 250")
        .bind(company_id).bind(LoanStatus::NeedsReview.as_str()).bind(LoanStatus::Draft.as_str()).bind(LoanStatus::Active.as_str()).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let loans = loan_rows
        .into_iter()
        .map(|row| -> Result<LoanDto, String> {
            Ok(LoanDto {
                id: try_column!(row, "id"),
                member_id: try_column!(row, "member_id"),
                member_name: try_column!(row, "member_name"),
                plafond: try_column!(row, "plafond"),
                balance: try_column!(row, "balance"),
                rate: try_column!(row, "rate_annual"),
                tenor: try_column!(row, "tenor"),
                interest_type: crate::domain::InterestType::try_from(
                    try_column!(row, "interest_type", String).as_str(),
                )?,
                realization_date: try_column!(row, "realization_date"),
                due_date: try_column!(row, "due_date"),
                status: LoanStatus::try_from(try_column!(row, "status", String).as_str())?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let transaction_rows = sqlx::query("SELECT id, display_date, display_time, member_name, description, reference, direction, amount, status, actor FROM transactions WHERE company_id = ? ORDER BY business_date DESC, id DESC LIMIT 120")
        .bind(company_id).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let mut transactions = Vec::new();
    for row in transaction_rows {
        let id: String = try_column!(row, "id");
        let component_rows = sqlx::query(
            "SELECT label, amount FROM transaction_components WHERE company_id = ? AND transaction_id = ? ORDER BY id",
        )
        .bind(company_id)
        .bind(&id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
        transactions.push(TransactionDto {
            id,
            date: try_column!(row, "display_date"),
            time: try_column!(row, "display_time"),
            member_name: try_column!(row, "member_name"),
            description: try_column!(row, "description"),
            reference: try_column!(row, "reference"),
            direction: TransactionDirection::try_from(
                try_column!(row, "direction", String).as_str(),
            )?,
            amount: try_column!(row, "amount"),
            status: crate::domain::TransactionStatus::try_from(
                try_column!(row, "status", String).as_str(),
            )?,
            actor: try_column!(row, "actor"),
            components: component_rows
                .into_iter()
                .map(|component| -> Result<ComponentDto, String> {
                    Ok(ComponentDto {
                        label: try_column!(component, "label"),
                        amount: try_column!(component, "amount"),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        });
    }

    let cash: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(CASE direction WHEN ? THEN amount ELSE -amount END), 0) FROM cash_postings WHERE company_id = ?")
        .bind(TransactionDirection::In.as_str()).bind(company_id).fetch_one(pool).await.map_err(|error| error.to_string())?;
    let savings: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance), 0) FROM savings_accounts WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let loan_portfolio: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance), 0) FROM loans WHERE company_id = ? AND status IN (?, ?)",
    )
    .bind(company_id)
    .bind(LoanStatus::Active.as_str())
    .bind(LoanStatus::NeedsReview.as_str())
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
