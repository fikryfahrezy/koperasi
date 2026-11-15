//! Neraca lines typed per book year; the rest of the Neraca is derived from
//! the monthly ledger and the cash book on the page. See
//! notes/rules-and-formulas.md (Neraca).

use crate::contracts::{BalanceSheetDetailDto, BalanceSheetDto, SetBalanceSheetInput};
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::BTreeMap;

/// Keys of the typed lines, as in the Neraca of the RAT report.
pub(crate) const BALANCE_SHEET_ITEMS: &[&str] = &[
    // Aktiva
    "CASH_ADJUSTMENT",
    "BANK_BALANCE",
    "OTHER_LOANS",
    "UNCLAIMED_RECEIVABLES",
    "ACCRUED_EXPENSES",
    "FIXED_ASSETS_DESK",
    "FIXED_ASSETS_CHAIR",
    "FIXED_ASSETS",
    // Hutang lancar
    "DEBT_PPBRI",
    "NON_MEMBER_SAVINGS",
    "CAPITAL_RESERVE",
    "EDUCATION_FUND",
    "SOCIAL_FUND",
    "PMDK_FUND",
    // Hutang jangka panjang
    "LONG_TERM_DEBT",
    // Modal sendiri
    "RAT_RESERVE",
    "LOAN_WRITE_OFF_RESERVE",
    "RETAINED_EARNINGS",
    // Pos added by hand to a group (a list each)
    "EXTRA_CURRENT_ASSETS",
    "EXTRA_CURRENT_LIABILITIES",
    "EXTRA_LONG_TERM_LIABILITIES",
    "EXTRA_EQUITY",
];

/// The typed lines of a book year and the rincian behind lines such as
/// Piutang Lainnya and Aktiva Tetap (by key, in their saved order).
pub(crate) async fn get_balance_sheet(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<BalanceSheetDto, String> {
    let rows = sqlx::query(
        "SELECT item_key, amount FROM balance_sheet_items WHERE company_id = ? AND year = ?",
    )
    .bind(company_id)
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let items = rows
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get("item_key").map_err(|error| error.to_string())?,
                row.try_get("amount").map_err(|error| error.to_string())?,
            ))
        })
        .collect::<Result<BTreeMap<String, i64>, String>>()?;
    let rows = sqlx::query(
        "SELECT item_key, name, amount, loan_date, note FROM balance_sheet_details WHERE company_id = ? AND year = ? ORDER BY item_key, position",
    )
    .bind(company_id)
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let mut details: BTreeMap<String, Vec<BalanceSheetDetailDto>> = BTreeMap::new();
    for row in rows {
        let key: String = row.try_get("item_key").map_err(|error| error.to_string())?;
        details.entry(key).or_default().push(BalanceSheetDetailDto {
            name: row.try_get("name").map_err(|error| error.to_string())?,
            amount: row.try_get("amount").map_err(|error| error.to_string())?,
            loan_date: row
                .try_get("loan_date")
                .map_err(|error| error.to_string())?,
            note: row.try_get("note").map_err(|error| error.to_string())?,
        });
    }
    Ok(BalanceSheetDto { items, details })
}

/// Saves the typed lines of a book year (lines left out keep their value) and
/// replaces the rincian of each line in `details`, setting that line to the
/// total of its rincian. Everything is saved together or not at all.
pub(crate) async fn set_balance_sheet(
    input: SetBalanceSheetInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<(), String> {
    if let Some(key) = input
        .items
        .keys()
        .chain(input.details.keys())
        .find(|key| !BALANCE_SHEET_ITEMS.contains(&key.as_str()))
    {
        return Err(format!("Pos neraca tidak dikenal: {key}"));
    }
    if input
        .details
        .values()
        .flatten()
        .any(|detail| detail.name.trim().is_empty())
    {
        return Err("Nama rincian wajib diisi".to_string());
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, amount) in &input.items {
        set_item(&mut db, company_id, input.year, key, *amount).await?;
    }
    for (key, details) in &input.details {
        sqlx::query(
            "DELETE FROM balance_sheet_details WHERE company_id = ? AND year = ? AND item_key = ?",
        )
        .bind(company_id)
        .bind(input.year)
        .bind(key)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
        for (position, detail) in details.iter().enumerate() {
            let loan_date = detail
                .loan_date
                .as_deref()
                .map(str::trim)
                .filter(|date| !date.is_empty());
            sqlx::query(
                "INSERT INTO balance_sheet_details (company_id, year, item_key, position, name, amount, loan_date, note) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(company_id)
            .bind(input.year)
            .bind(key)
            .bind(position as i64)
            .bind(detail.name.trim())
            .bind(detail.amount)
            .bind(loan_date)
            .bind(detail.note.trim())
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        }
        let total: i64 = details.iter().map(|detail| detail.amount).sum();
        set_item(&mut db, company_id, input.year, key, total).await?;
    }
    db.commit().await.map_err(|error| error.to_string())
}

async fn set_item(
    db: &mut SqliteConnection,
    company_id: &str,
    year: i32,
    key: &str,
    amount: i64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO balance_sheet_items (company_id, year, item_key, amount) VALUES (?, ?, ?, ?) ON CONFLICT(company_id, year, item_key) DO UPDATE SET amount = excluded.amount, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(company_id)
    .bind(year)
    .bind(key)
    .bind(amount)
    .execute(db)
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(name: &str, amount: i64, loan_date: Option<&str>) -> BalanceSheetDetailDto {
        BalanceSheetDetailDto {
            name: name.to_string(),
            amount,
            loan_date: loan_date.map(str::to_string),
            note: String::new(),
        }
    }

    fn items(year: i32, key: &str, amount: i64) -> SetBalanceSheetInput {
        SetBalanceSheetInput {
            year,
            items: BTreeMap::from([(key.to_string(), amount)]),
            details: BTreeMap::new(),
        }
    }

    fn rincian(key: &str, rows: Vec<BalanceSheetDetailDto>) -> SetBalanceSheetInput {
        SetBalanceSheetInput {
            year: 2030,
            items: BTreeMap::new(),
            details: BTreeMap::from([(key.to_string(), rows)]),
        }
    }

    #[test]
    fn items_are_kept_per_book_year() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            for input in [
                items(2030, "DEBT_PPBRI", 10_000_000),
                items(2030, "OTHER_LOANS", 28_109_000),
                items(2030, "DEBT_PPBRI", 5_000_000),
            ] {
                set_balance_sheet(input, "default", &pool).await.unwrap();
            }
            let sheet = get_balance_sheet("default", 2030, &pool).await.unwrap();
            assert_eq!(sheet.items.get("DEBT_PPBRI"), Some(&5_000_000));
            assert_eq!(sheet.items.get("OTHER_LOANS"), Some(&28_109_000));
            assert!(get_balance_sheet("default", 2029, &pool)
                .await
                .unwrap()
                .items
                .is_empty());
            assert!(
                set_balance_sheet(items(2030, "UNKNOWN", 1), "default", &pool)
                    .await
                    .is_err()
            );
        });
    }

    #[test]
    fn details_replace_the_rincian_and_set_the_line_total() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            set_balance_sheet(
                rincian("OTHER_LOANS", vec![detail("Lama", 9, None)]),
                "default",
                &pool,
            )
            .await
            .unwrap();
            set_balance_sheet(
                rincian(
                    "OTHER_LOANS",
                    vec![
                        detail("Peminjam A", 3_700_000, Some("2023-07-06")),
                        detail("Peminjam A", -1_391_000, None),
                        detail("Peminjam B", 500_000, Some("2023-01-04")),
                    ],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();
            let sheet = get_balance_sheet("default", 2030, &pool).await.unwrap();
            let rows = &sheet.details["OTHER_LOANS"];
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[1], detail("Peminjam A", -1_391_000, None));
            assert_eq!(rows[2].loan_date.as_deref(), Some("2023-01-04"));
            assert_eq!(sheet.items.get("OTHER_LOANS"), Some(&2_809_000));
            assert!(
                set_balance_sheet(rincian("UNKNOWN", vec![]), "default", &pool)
                    .await
                    .is_err()
            );
            assert!(set_balance_sheet(
                rincian("OTHER_LOANS", vec![detail(" ", 1, None)]),
                "default",
                &pool
            )
            .await
            .is_err());
        });
    }

    #[test]
    fn lines_and_rincian_are_saved_together_or_not_at_all() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let both = |debt: i64, rows| SetBalanceSheetInput {
                year: 2030,
                items: BTreeMap::from([("DEBT_PPBRI".to_string(), debt)]),
                details: BTreeMap::from([("FIXED_ASSETS".to_string(), rows)]),
            };
            set_balance_sheet(
                both(10_000_000, vec![detail("Meja", 1_500_000, None)]),
                "default",
                &pool,
            )
            .await
            .unwrap();
            let sheet = get_balance_sheet("default", 2030, &pool).await.unwrap();
            assert_eq!(sheet.items.get("DEBT_PPBRI"), Some(&10_000_000));
            assert_eq!(sheet.items.get("FIXED_ASSETS"), Some(&1_500_000));
            // A rincian row without a name rejects the whole save: the line
            // typed with it is not changed either.
            assert!(set_balance_sheet(
                both(5_000_000, vec![detail("", 1, None)]),
                "default",
                &pool
            )
            .await
            .is_err());
            let sheet = get_balance_sheet("default", 2030, &pool).await.unwrap();
            assert_eq!(sheet.items.get("DEBT_PPBRI"), Some(&10_000_000));
            assert_eq!(
                sheet.details["FIXED_ASSETS"],
                vec![detail("Meja", 1_500_000, None)]
            );
        });
    }
}
