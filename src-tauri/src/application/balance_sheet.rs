//! Neraca lines typed per book year; the rest of the Neraca is derived from
//! the monthly ledger and the cash book on the page. See
//! notes/rules-and-formulas.md (Neraca).

use super::posting::audit_change;
use crate::contracts::{BalanceSheetDetailDto, BalanceSheetDto, SetBalanceSheetInput};
use crate::domain::{AuditAction, AuditEntityType};
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::BTreeMap;

/// Keys of the typed lines, as in the Neraca of the RAT report.
pub(crate) const BALANCE_SHEET_ITEMS: &[&str] = &[
    // Aktiva
    "CASH_ADJUSTMENT",
    "BANK_BALANCE",
    // Saldo bank at the end of January..November (BANK_BALANCE is 31 Desember)
    "BANK_BALANCE_01",
    "BANK_BALANCE_02",
    "BANK_BALANCE_03",
    "BANK_BALANCE_04",
    "BANK_BALANCE_05",
    "BANK_BALANCE_06",
    "BANK_BALANCE_07",
    "BANK_BALANCE_08",
    "BANK_BALANCE_09",
    "BANK_BALANCE_10",
    "BANK_BALANCE_11",
    "OTHER_LOANS",
    "UNCLAIMED_RECEIVABLES",
    "ACCRUED_EXPENSES",
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
    // Laba of a year whose transactions are incomplete (2025), typed from the sheet
    "NET_INCOME",
    // Added to the derived Piutang / Simpanan Manasuka lines of such a year
    "MEMBER_LOANS_ADJUSTMENT",
    "NON_MEMBER_LOANS_ADJUSTMENT",
    "VOLUNTARY_SAVINGS_ADJUSTMENT",
    // Pos added by hand to a group (a list each)
    "EXTRA_CURRENT_ASSETS",
    "EXTRA_CURRENT_LIABILITIES",
    "EXTRA_LONG_TERM_LIABILITIES",
    "EXTRA_EQUITY",
];

/// Lines that are a list (`balance_sheet_details`): their amount is the total of
/// the list, computed when the Neraca is shown and never stored as a line.
const LIST_ITEMS: &[&str] = &[
    "OTHER_LOANS",
    "FIXED_ASSETS",
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
        "SELECT item_key, amount
         FROM balance_sheet_items
         WHERE company_id = ? AND year = ?",
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
        "SELECT item_key, name, amount, loan_date, note
         FROM balance_sheet_details
         WHERE company_id = ? AND year = ?
         ORDER BY item_key, position",
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

    let prior_items = sums(
        sqlx::query(
            "SELECT item_key AS key, SUM(amount) AS amount
             FROM balance_sheet_items
             WHERE company_id = ? AND year < ?
             GROUP BY item_key",
        )
        .bind(company_id)
        .bind(year),
        pool,
    )
    .await?;

    let prior_categories = sums(
        sqlx::query(
            "SELECT component_type AS key, SUM(amount) AS amount
             FROM effective_components
             WHERE company_id = ? AND channel = 'KAS' AND business_date < ?
             GROUP BY component_type",
        )
        .bind(company_id)
        .bind(format!("{year:04}-01-01")),
        pool,
    )
    .await?;

    Ok(BalanceSheetDto {
        items,
        details,
        prior_items,
        prior_categories,
    })
}

/// Rows of `key` and `amount` as a map.
async fn sums<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    pool: &SqlitePool,
) -> Result<BTreeMap<String, i64>, String> {
    query
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get("key").map_err(|error| error.to_string())?,
                row.try_get("amount").map_err(|error| error.to_string())?,
            ))
        })
        .collect()
}

/// Saves the typed lines of a book year (lines left out keep their value) and
/// replaces the rincian of each line in `details`. A list line has no stored
/// amount: its total is the sum of its rincian. Everything is saved together or
/// not at all.
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

    if let Some(key) = input
        .items
        .keys()
        .find(|key| LIST_ITEMS.contains(&key.as_str()))
    {
        return Err(format!("Pos neraca {key} diisi lewat rinciannya."));
    }

    if input
        .details
        .values()
        .flatten()
        .any(|detail| detail.name.trim().is_empty())
    {
        return Err("Nama rincian wajib diisi".to_string());
    }

    // Lines and rincian are replaced in place, so the audit keeps what they were.
    // Read before the transaction: the pool has a single connection.
    let previous = get_balance_sheet(company_id, input.year, pool).await?;
    let before = serde_json::json!({
        "items": previous
            .items
            .iter()
            .filter(|(key, _)| input.items.contains_key(*key))
            .collect::<BTreeMap<_, _>>(),
        "details": previous
            .details
            .iter()
            .filter(|(key, _)| input.details.contains_key(*key))
            .collect::<BTreeMap<_, _>>(),
    });

    let mut db = pool.begin().await.map_err(|error| error.to_string())?;

    for (key, amount) in &input.items {
        set_item(&mut db, company_id, input.year, key, *amount).await?;
    }

    for (key, details) in &input.details {
        sqlx::query(
            "DELETE FROM balance_sheet_details
             WHERE company_id = ? AND year = ? AND item_key = ?",
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
                "INSERT INTO balance_sheet_details (
                     company_id, year, item_key, position, name, amount, loan_date, note
                 )
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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
    }

    audit_change(
        &mut db,
        company_id,
        AuditEntityType::BalanceSheet,
        &input.year.to_string(),
        AuditAction::Updated,
        Some(before),
        serde_json::json!({"items": &input.items, "details": &input.details}),
    )
    .await?;

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
        "INSERT INTO balance_sheet_items (company_id, year, item_key, amount)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (company_id, year, item_key)
         DO UPDATE SET amount = excluded.amount",
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
                items(2030, "DEBT_PPBRI", 5_000_000),
            ] {
                set_balance_sheet(input, "default", &pool).await.unwrap();
            }

            let sheet = get_balance_sheet("default", 2030, &pool).await.unwrap();
            assert_eq!(sheet.items.get("DEBT_PPBRI"), Some(&5_000_000));

            // A list line is only its rincian; it cannot be typed as one amount.
            assert!(
                set_balance_sheet(items(2030, "OTHER_LOANS", 28_109_000), "default", &pool)
                    .await
                    .is_err()
            );

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
    fn earlier_book_years_are_added_up_for_the_opening() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;

            for input in [
                items(2029, "DEBT_PPBRI", 10_000_000),
                items(2030, "DEBT_PPBRI", 2_000_000),
                items(2031, "DEBT_PPBRI", 500_000),
            ] {
                set_balance_sheet(input, "default", &pool).await.unwrap();
            }

            let sheet = get_balance_sheet("default", 2031, &pool).await.unwrap();
            assert_eq!(sheet.items.get("DEBT_PPBRI"), Some(&500_000));
            assert_eq!(sheet.prior_items.get("DEBT_PPBRI"), Some(&12_000_000));

            let first = get_balance_sheet("default", 2029, &pool).await.unwrap();
            assert!(first.prior_items.is_empty());
            assert!(first.prior_categories.is_empty());
        });
    }

    #[test]
    fn details_replace_the_rincian_without_a_stored_total() {
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
            assert_eq!(sheet.items.get("OTHER_LOANS"), None);

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
            assert_eq!(sheet.items.get("FIXED_ASSETS"), None);

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

    #[test]
    fn saving_keeps_the_replaced_values_in_the_audit() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            let save = |items: Vec<(&str, i64)>,
                        details: Vec<(&str, Vec<BalanceSheetDetailDto>)>| {
                SetBalanceSheetInput {
                    year: 2026,
                    items: items
                        .into_iter()
                        .map(|(key, amount)| (key.into(), amount))
                        .collect(),
                    details: details
                        .into_iter()
                        .map(|(key, rows)| (key.into(), rows))
                        .collect(),
                }
            };

            set_balance_sheet(
                save(
                    vec![("BANK_BALANCE", 1_000)],
                    vec![("OTHER_LOANS", vec![detail("A", 300, None)])],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();

            set_balance_sheet(
                save(
                    vec![("BANK_BALANCE", 2_000), ("BANK_BALANCE_09", 1_500)],
                    vec![("OTHER_LOANS", vec![detail("B", 500, None)])],
                ),
                "default",
                &pool,
            )
            .await
            .unwrap();

            let (before, after): (String, String) = sqlx::query_as(
                "SELECT before_json, after_json
                 FROM audit_events
                 WHERE entity_type = 'BALANCE_SHEET' AND entity_id = '2026'
                 ORDER BY id DESC
                 LIMIT 1",
            )
            .fetch_one(&pool)
            .await
            .unwrap();

            let before: serde_json::Value = serde_json::from_str(&before).unwrap();
            let after: serde_json::Value = serde_json::from_str(&after).unwrap();
            assert_eq!(before["items"]["BANK_BALANCE"], 1_000);
            assert_eq!(before["details"]["OTHER_LOANS"][0]["name"], "A");
            assert_eq!(after["items"]["BANK_BALANCE"], 2_000);
            // The saldo bank of a month end is a line of its own.
            assert_eq!(after["items"]["BANK_BALANCE_09"], 1_500);
            assert_eq!(after["details"]["OTHER_LOANS"][0]["name"], "B");
        });
    }
}
