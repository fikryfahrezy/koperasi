//! Application use cases for read model.

use sqlx::{Row, SqlitePool};

use crate::contracts::*;

async fn snapshot(pool: &SqlitePool) -> Result<AppSnapshot, String> {
    let member_rows = sqlx::query(
        "SELECT m.id, m.member_number, m.name, m.joined_at, m.status, COALESCE(SUM(sa.balance), 0) savings, COALESCE(SUM(CASE WHEN sa.account_type = 'POKOK' THEN sa.balance ELSE 0 END), 0) principal_savings, COALESCE(SUM(CASE WHEN sa.account_type = 'WAJIB' THEN sa.balance ELSE 0 END), 0) mandatory_savings, COALESCE(SUM(CASE WHEN sa.account_type = 'MANASUKA' THEN sa.balance ELSE 0 END), 0) voluntary_savings, COALESCE((SELECT SUM(l.balance) FROM loans l WHERE l.member_id = m.id AND l.status IN ('Berjalan', 'Perlu review')), 0) loan_balance FROM members m LEFT JOIN savings_accounts sa ON sa.member_id = m.id GROUP BY m.id ORDER BY m.name"
    ).fetch_all(pool).await.map_err(|error| error.to_string())?;
    let members = member_rows
        .into_iter()
        .map(|row| MemberDto {
            id: row.get("id"),
            member_number: row.get("member_number"),
            name: row.get("name"),
            joined_at: row.get("joined_at"),
            status: row.get("status"),
            savings: row.get("savings"),
            principal_savings: row.get("principal_savings"),
            mandatory_savings: row.get("mandatory_savings"),
            voluntary_savings: row.get("voluntary_savings"),
            loan_balance: row.get("loan_balance"),
        })
        .collect();

    let loan_rows = sqlx::query("SELECT id, COALESCE(member_id, '') member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status FROM loans ORDER BY CASE status WHEN 'Perlu review' THEN 0 WHEN 'Draf' THEN 1 WHEN 'Berjalan' THEN 2 ELSE 3 END, id LIMIT 250")
        .fetch_all(pool).await.map_err(|error| error.to_string())?;
    let loans = loan_rows
        .into_iter()
        .map(|row| LoanDto {
            id: row.get("id"),
            member_id: row.get("member_id"),
            member_name: row.get("member_name"),
            plafond: row.get("plafond"),
            balance: row.get("balance"),
            rate: row.get("rate_annual"),
            tenor: row.get("tenor"),
            interest_type: row.get("interest_type"),
            realization_date: row.get("realization_date"),
            due_date: row.get("due_date"),
            status: row.get("status"),
        })
        .collect();

    let transaction_rows = sqlx::query("SELECT id, display_date, display_time, member_name, description, reference, direction, amount, status, actor FROM transactions ORDER BY business_date DESC, id DESC LIMIT 120")
        .fetch_all(pool).await.map_err(|error| error.to_string())?;
    let mut transactions = Vec::new();
    for row in transaction_rows {
        let id: String = row.get("id");
        let component_rows = sqlx::query(
            "SELECT label, amount FROM transaction_components WHERE transaction_id = ? ORDER BY id",
        )
        .bind(&id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
        transactions.push(TransactionDto {
            id,
            date: row.get("display_date"),
            time: row.get("display_time"),
            member_name: row.get("member_name"),
            description: row.get("description"),
            reference: row.get("reference"),
            direction: row.get("direction"),
            amount: row.get("amount"),
            status: row.get("status"),
            actor: row.get("actor"),
            components: component_rows
                .into_iter()
                .map(|component| ComponentDto {
                    label: component.get("label"),
                    amount: component.get("amount"),
                })
                .collect(),
        });
    }

    let cash: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(CASE direction WHEN 'Masuk' THEN amount ELSE -amount END), 0) FROM cash_postings")
        .fetch_one(pool).await.map_err(|error| error.to_string())?;
    let savings: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(balance), 0) FROM savings_accounts")
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())?;
    let loan_portfolio: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance), 0) FROM loans WHERE status IN ('Berjalan', 'Perlu review')",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let member_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE status = 'Aktif'")
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

pub(crate) async fn get_app_snapshot(pool: &SqlitePool) -> Result<AppSnapshot, String> {
    snapshot(pool).await
}
