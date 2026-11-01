//! Neraca lines typed per book year; the rest of the Neraca is derived from
//! the monthly ledger and the cash book on the page. See
//! notes/rules-and-formulas.md (Neraca).

use crate::contracts::SetBalanceSheetItemsInput;
use sqlx::{Row, SqlitePool};
use std::collections::BTreeMap;

/// Keys of the typed lines, as in NERACA of BUKU KAS HARIAN.
pub(crate) const BALANCE_SHEET_ITEMS: &[&str] = &[
    // Aktiva
    "CASH_ADJUSTMENT",
    "BANK_BALANCE",
    "OTHER_LOANS",
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
];

pub(crate) async fn get_balance_sheet_items(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<BTreeMap<String, i64>, String> {
    let rows = sqlx::query(
        "SELECT item_key, amount FROM balance_sheet_items WHERE company_id = ? AND year = ?",
    )
    .bind(company_id)
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("item_key").map_err(|error| error.to_string())?,
                row.try_get("amount").map_err(|error| error.to_string())?,
            ))
        })
        .collect()
}

/// Saves the typed lines of a book year; lines left out keep their value.
pub(crate) async fn set_balance_sheet_items(
    input: SetBalanceSheetItemsInput,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<(), String> {
    if let Some(key) = input
        .items
        .keys()
        .find(|key| !BALANCE_SHEET_ITEMS.contains(&key.as_str()))
    {
        return Err(format!("Pos neraca tidak dikenal: {key}"));
    }
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;
    for (key, amount) in &input.items {
        sqlx::query(
            "INSERT INTO balance_sheet_items (company_id, year, item_key, amount) VALUES (?, ?, ?, ?) ON CONFLICT(company_id, year, item_key) DO UPDATE SET amount = excluded.amount, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(company_id)
        .bind(input.year)
        .bind(key)
        .bind(amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }
    db.commit().await.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_kept_per_book_year() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let save = |year, key: &str, amount| SetBalanceSheetItemsInput {
                year,
                items: BTreeMap::from([(key.to_string(), amount)]),
            };
            set_balance_sheet_items(save(2030, "DEBT_PPBRI", 10_000_000), "default", &pool)
                .await
                .unwrap();
            set_balance_sheet_items(save(2030, "OTHER_LOANS", 28_109_000), "default", &pool)
                .await
                .unwrap();
            set_balance_sheet_items(save(2030, "DEBT_PPBRI", 5_000_000), "default", &pool)
                .await
                .unwrap();
            let items = get_balance_sheet_items("default", 2030, &pool)
                .await
                .unwrap();
            assert_eq!(items.get("DEBT_PPBRI"), Some(&5_000_000));
            assert_eq!(items.get("OTHER_LOANS"), Some(&28_109_000));
            assert!(get_balance_sheet_items("default", 2029, &pool)
                .await
                .unwrap()
                .is_empty());
            assert!(
                set_balance_sheet_items(save(2030, "UNKNOWN", 1), "default", &pool)
                    .await
                    .is_err()
            );
        });
    }
}
