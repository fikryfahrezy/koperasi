//! Legacy workbook import orchestration and database seeding.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::{application, contracts::AppSnapshot, domain::normalize_name, workbook::*};

pub(crate) async fn seed_database(
    pool: &SqlitePool,
    sheets: &WorkbookSheets,
) -> Result<(), String> {
    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;

    let mut latest_savings: HashMap<String, (i64, i64, i64)> = HashMap::new();
    for row in &sheets.savings_2026 {
        if row.len() >= 17 {
            latest_savings.insert(
                cell_text(&row[0]),
                (
                    cell_money(&row[14]),
                    cell_money(&row[15]),
                    cell_money(&row[16]),
                ),
            );
        }
    }

    let mut member_by_name = HashMap::new();
    for row in &sheets.master_savings {
        if row.len() < 8 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let member_id = cell_text(&row[0]);
        let name = cell_text(&row[3]);
        member_by_name.insert(normalize_name(&name), member_id.clone());
        sqlx::query("INSERT INTO members (id, member_number, name, joined_at, status) VALUES (?, ?, ?, ?, 'Aktif')")
            .bind(&member_id)
            // Kolom `No` pada workbook berisi nomor 134 dua kali (M134 dan M135).
            // Member ID hasil pembersihan sudah unik dan berurutan, sehingga menjadi
            // sumber nomor anggota migrasi yang deterministik.
            .bind(format!("KBS-{:0>4}", member_id.trim_start_matches('M')))
            .bind(&name)
            .bind("Migrasi 2026")
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;

        let balances = latest_savings.get(&member_id).copied().unwrap_or((
            cell_money(&row[4]),
            cell_money(&row[5]),
            cell_money(&row[6]),
        ));
        for (kind, balance) in [
            ("POKOK", balances.0),
            ("WAJIB", balances.1),
            ("MANASUKA", balances.2),
        ] {
            sqlx::query("INSERT INTO savings_accounts (id, member_id, account_type, balance) VALUES (?, ?, ?, ?)")
                .bind(format!("SA-{member_id}-{kind}"))
                .bind(&member_id)
                .bind(kind)
                .bind(balance.max(0))
                .execute(&mut *transaction)
                .await
                .map_err(|error| error.to_string())?;
        }
    }

    let mut latest_loan_balance: HashMap<String, i64> = HashMap::new();
    for row in &sheets.loans_2026 {
        if row.len() >= 23 {
            latest_loan_balance.insert(cell_text(&row[0]), cell_money(&row[22]).max(0));
        }
    }

    for row in &sheets.master_loans {
        if row.len() < 16 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let loan_id = cell_text(&row[0]);
        let name = cell_text(&row[3]);
        let plafond = cell_money(&row[4]).max(0);
        let balance = latest_loan_balance
            .get(&loan_id)
            .copied()
            .unwrap_or(plafond);
        let member_id = member_by_name.get(&normalize_name(&name)).cloned();
        let status = if cell_text(&row[15]) == "PERLU CEK" {
            "Perlu review"
        } else if balance == 0 {
            "Lunas"
        } else {
            "Berjalan"
        };
        sqlx::query("INSERT INTO loans (id, member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&loan_id)
            .bind(member_id)
            .bind(&name)
            .bind(plafond)
            .bind(balance)
            .bind(cell_percentage(&row[6]))
            .bind(cell_int(&row[11]).max(1))
            .bind(if cell_text(&row[7]) == "Flat" { "Flat" } else { "Menurun" })
            .bind(cell_date_display(&row[9]))
            .bind(cell_date_display(&row[10]))
            .bind(status)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
    }

    // Buku kas sumber memulai mutasi 2026 dari saldo bawaan Rp21.974.442.
    // Saldo tersebut harus menjadi posting tersendiri agar ledger database dapat
    // menghitung saldo akhir tanpa bergantung pada kolom saldo hasil Excel.
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES ('OPENING-CASH-2026', '2026-01-01', '01-Jan-2026', '00:00', 'Koperasi', 'OPENING_BALANCE', 'Saldo awal kas 2026', 'OPENING/KAS/2026', 'Masuk', 21974442, 'Terposting', 'System Migration')")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount) VALUES ('OPENING-CASH-2026', 'CASH_OPENING', 'Saldo awal kas', 21974442)")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO cash_postings (transaction_id, direction, amount) VALUES ('OPENING-CASH-2026', 'Masuk', 21974442)")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;

    for row in &sheets.cash_2026 {
        if row.len() < 11 {
            continue;
        }
        let cash_out = cell_money(&row[4]);
        let cash_in = cell_money(&row[5]);
        let net_amount = cash_in - cash_out;
        if net_amount == 0 {
            continue;
        }
        let amount = net_amount.abs();
        let source_row = cell_text(&row[9]);
        let id = format!("MIG-KAS-{source_row}");
        let direction = if net_amount > 0 { "Masuk" } else { "Keluar" };
        let business_date = cell_date_iso(&row[0]);
        let display_date = cell_date_display(&row[0]);
        let description = cell_text(&row[1]);
        let reference = format!("MIG/KAS/2026/{source_row}");
        sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, '00:00', ?, 'MIGRATED_CASH', ?, ?, ?, ?, 'Terposting', 'Import Excel 2026')")
            .bind(&id)
            .bind(&business_date)
            .bind(&display_date)
            .bind(&description)
            .bind(&description)
            .bind(&reference)
            .bind(direction)
            .bind(amount)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount) VALUES (?, 'MIGRATION_UNCLASSIFIED', 'Migrasi buku kas', ?)")
            .bind(&id)
            .bind(amount)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, ?, ?)",
        )
        .bind(&id)
        .bind(direction)
        .bind(amount)
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    }

    sqlx::query("INSERT INTO periods (period, status) VALUES ('2026-09', 'OPEN')")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    for (key, value) in [
        ("SAVINGS_PRINCIPAL", 50_000.0),
        ("SAVINGS_MONTHLY", 50_000.0),
        ("LOAN_PROVISION_RATE", 0.01),
        ("LOAN_ANNUAL_RATE", 0.24),
    ] {
        sqlx::query("INSERT INTO parameters (parameter_key, value, effective_date, created_by) VALUES (?, ?, '2026-01-01', 'System Migration')")
            .bind(key).bind(value).execute(&mut *transaction).await.map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO app_meta (key, value) VALUES ('seed_version', '1')")
        .execute(&mut *transaction)
        .await
        .map_err(|error| error.to_string())?;
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn import_workbook(
    path: String,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let already_seeded: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM app_meta WHERE key = 'seed_version'")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    if already_seeded > 0 {
        return Err(
            "Data sudah pernah diimpor. Hapus data aplikasi terlebih dahulu untuk mengimpor ulang."
                .into(),
        );
    }
    let sheets = tauri::async_runtime::spawn_blocking(move || {
        load_workbook_sheets(std::path::Path::new(&path))
    })
    .await
    .map_err(|error| error.to_string())??;
    seed_database(pool, &sheets).await?;
    application::get_app_snapshot(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::path::Path;

    // This test seeds an in-memory database from the real cooperative workbook.
    // The workbook contains real members' financial data and is intentionally
    // excluded from git (see .gitignore), so it only runs on machines that have
    // a copy of it under docs/.
    #[test]
    fn seeds_the_cleaned_workbook_into_a_balanced_database() {
        let workbook_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/KOPERASI_BINA_SEJAHTERA_2026_DIBERSIHKAN.xlsm"
        ))
        .to_path_buf();
        if !workbook_path.exists() {
            eprintln!(
                "skipping seeds_the_cleaned_workbook_into_a_balanced_database: {} not found locally",
                workbook_path.display()
            );
            return;
        }

        tauri::async_runtime::block_on(async {
            let sheets = load_workbook_sheets(&workbook_path).unwrap();
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            sqlx::raw_sql(include_str!("../migrations/001_backend.sql"))
                .execute(&pool)
                .await
                .unwrap();
            seed_database(&pool, &sheets).await.unwrap();

            let member_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM members")
                .fetch_one(&pool)
                .await
                .unwrap();
            let loan_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loans")
                .fetch_one(&pool)
                .await
                .unwrap();
            let cash: i64 = sqlx::query_scalar("SELECT SUM(CASE direction WHEN 'Masuk' THEN amount ELSE -amount END) FROM cash_postings")
                .fetch_one(&pool)
                .await
                .unwrap();
            let savings: i64 = sqlx::query_scalar("SELECT SUM(balance) FROM savings_accounts")
                .fetch_one(&pool)
                .await
                .unwrap();
            let foreign_key_errors: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
                    .fetch_one(&pool)
                    .await
                    .unwrap();

            assert_eq!(member_count, 144);
            assert_eq!(loan_count, 145);
            assert_eq!(cash, 135_681_811);
            assert_eq!(savings, 960_221_990);
            assert_eq!(foreign_key_errors, 0);
        });
    }
}
