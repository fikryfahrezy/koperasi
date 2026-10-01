//! Legacy workbook import: mengisi buku besar dari workbook Excel 2026.
//!
//! Tidak ada saldo yang disalin. Semuanya masuk sebagai transaksi:
//! - saldo awal simpanan, pinjaman, dan kas (channel NON_KAS / KAS),
//! - mutasi bulanan per anggota dari SIMPANAN_2026 dan PINJAMAN_2026
//!   (channel POTONGAN, karena umumnya dipotong dari pensiun lewat bank),
//! - baris Buku Kas Harian dari KAS_2026 (channel KAS) sebagai
//!   "Belum diklasifikasi" sampai dihubungkan ke anggota.

use std::collections::{HashMap, HashSet};

use sqlx::{SqliteConnection, SqlitePool};

use crate::{
    application::{
        self,
        posting::{append, Component, Entry},
    },
    contracts::AppSnapshot,
    domain::{
        normalize_name, Channel, InterestType, LoanStatus, MemberStatus, PeriodStatus,
        SavingsAccountStatus, SavingsAccountType, TransactionDirection,
    },
    workbook::*,
};

/// Jenis transaksi riwayat hasil migrasi.
pub(crate) const MIGRATED_SAVINGS: &str = "MIGRATED_SAVINGS";
pub(crate) const MIGRATED_LOAN: &str = "MIGRATED_LOAN";
const MIGRATED_CASH: &str = "MIGRATED_CASH";
const IMPORT_ACTOR: &str = "Import Excel 2026";
/// Saldo kas akhir 2025 menurut Buku Kas Harian.
const OPENING_CASH_2026: i64 = 21_974_442;

/// Tanggal transaksi untuk baris bulan `month_start`. Tanggal kosong memakai awal
/// bulan; tanggal yang salah ketik tahunnya (mis. 02-Sep-2029 di baris
/// September 2026) dipindah ke tahun baris tersebut agar mutasi tetap jatuh di
/// bulan yang benar.
fn history_date(month_start: &str, transaction_date: String) -> String {
    if transaction_date.is_empty() {
        return month_start.to_string();
    }
    if transaction_date.get(0..7) == month_start.get(0..7) {
        return transaction_date;
    }
    if transaction_date.get(5..7) == month_start.get(5..7) {
        if let (Some(period), Some(day)) = (month_start.get(0..7), transaction_date.get(8..10)) {
            return format!("{period}-{day}");
        }
    }
    month_start.to_string()
}

fn display_date(business_date: &str) -> String {
    chrono::NaiveDate::parse_from_str(business_date, "%Y-%m-%d")
        .map(|date| date.format("%d-%b-%Y").to_string())
        .unwrap_or_else(|_| business_date.to_string())
}

/// Komponen penyesuaian agar saldo hasil penjumlahan transaksi sama dengan
/// saldo akhir di sheet ketika baris sheet tidak seimbang.
fn balancing(
    difference: i64,
    increase: &'static str,
    decrease: &'static str,
) -> (&'static str, i64) {
    if difference >= 0 {
        (increase, difference)
    } else {
        (decrease, -difference)
    }
}

struct ImportEntry<'a> {
    id: String,
    business_date: String,
    member_id: Option<&'a str>,
    member_name: &'a str,
    transaction_type: &'static str,
    channel: Channel,
    description: &'a str,
    components: Vec<Component>,
}

/// Menulis transaksi impor; dilewati bila semua komponennya nol. Arah transaksi
/// mengikuti sisi yang lebih besar (uang masuk vs keluar).
async fn import_entry(
    db: &mut SqliteConnection,
    company_id: &str,
    entry: ImportEntry<'_>,
) -> Result<(), String> {
    let components: Vec<Component> = entry
        .components
        .into_iter()
        .filter(|component| component.amount > 0)
        .collect();
    if components.is_empty() {
        return Ok(());
    }
    let (incoming, outgoing) = components.iter().fold((0_i64, 0_i64), |(i, o), c| {
        if matches!(c.component_type, "SAVINGS_WITHDRAWAL" | "LOAN_DISBURSEMENT") {
            (i, o + c.amount)
        } else {
            (i + c.amount, o)
        }
    });
    let display_date = display_date(&entry.business_date);
    append(
        db,
        company_id,
        Entry {
            reference: &entry.id,
            id: entry.id.clone(),
            business_date: &entry.business_date,
            display_date: &display_date,
            display_time: "00:00",
            member_id: entry.member_id,
            member_name: entry.member_name,
            transaction_type: entry.transaction_type,
            channel: entry.channel,
            description: entry.description,
            direction: if incoming >= outgoing {
                TransactionDirection::In
            } else {
                TransactionDirection::Out
            },
            actor: IMPORT_ACTOR,
            reversed_transaction_id: None,
            components,
        },
    )
    .await
    .map(|_| ())
}

pub(crate) async fn seed_database(
    pool: &SqlitePool,
    company_id: &str,
    sheets: &WorkbookSheets,
) -> Result<(), String> {
    let mut db = pool.begin().await.map_err(|error| error.to_string())?;

    // --- Master anggota dan rekening simpanan. -----------------------------
    let mut member_by_name = HashMap::new();
    let mut member_ids = HashSet::new();
    for row in &sheets.master_savings {
        if row.len() < 8 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let member_id = format!("{company_id}-{}", cell_text(&row[0]));
        let name = cell_text(&row[3]);
        member_by_name.insert(normalize_name(&name), member_id.clone());
        sqlx::query(
            "INSERT INTO members (id, company_id, name, joined_at, status) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&member_id)
        .bind(company_id)
        .bind(&name)
        .bind("Migrasi 2026")
        .bind(MemberStatus::Active.as_str())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
        for kind in [
            SavingsAccountType::Principal,
            SavingsAccountType::Mandatory,
            SavingsAccountType::Voluntary,
        ] {
            sqlx::query("INSERT INTO savings_accounts (id, company_id, member_id, account_type, status) VALUES (?, ?, ?, ?, ?)")
                .bind(format!("SA-{member_id}-{}", kind.as_str()))
                .bind(company_id)
                .bind(&member_id)
                .bind(kind.as_str())
                .bind(SavingsAccountStatus::Active.as_str())
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
        }
        member_ids.insert(member_id);
    }

    // --- Master pinjaman (syarat saja, tanpa saldo). ------------------------
    let mut loan_members: HashMap<String, Option<String>> = HashMap::new();
    for row in &sheets.master_loans {
        if row.len() < 16 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let loan_id = format!("{company_id}-{}", cell_text(&row[0]));
        let name = cell_text(&row[3]);
        let member_id = member_by_name.get(&normalize_name(&name)).cloned();
        let status = if cell_text(&row[15]) == "PERLU CEK" {
            LoanStatus::NeedsReview
        } else {
            LoanStatus::Active
        };
        sqlx::query("INSERT INTO loans (id, company_id, member_id, member_name, plafond, rate_annual, tenor, interest_type, guarantee, realization_date, due_date, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&loan_id)
            .bind(company_id)
            .bind(&member_id)
            .bind(&name)
            .bind(cell_money(&row[4])?.max(0))
            .bind(cell_percentage(&row[6])?)
            .bind(cell_int(&row[11])?.max(1))
            .bind(if cell_text(&row[7]) == InterestType::Flat.as_str() { InterestType::Flat.as_str() } else { InterestType::Declining.as_str() })
            .bind(cell_text(&row[8]))
            .bind(cell_date_display(&row[9])?)
            .bind(cell_date_display(&row[10])?)
            .bind(status.as_str())
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        loan_members.insert(loan_id, member_id);
    }

    seed_savings_history(&mut db, company_id, sheets, &member_ids).await?;
    seed_loan_history(&mut db, company_id, sheets, &loan_members).await?;
    seed_cash_book(&mut db, company_id, sheets).await?;

    sqlx::query(
        "INSERT OR IGNORE INTO periods (company_id, period, status) VALUES (?, '2026-09', ?)",
    )
    .bind(company_id)
    .bind(PeriodStatus::Open.as_str())
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO app_meta (company_id, key, value) VALUES (?, 'seed_version', '3')")
        .bind(company_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())
}

/// Saldo awal lalu satu transaksi per anggota per bulan dari SIMPANAN_2026.
async fn seed_savings_history(
    db: &mut SqliteConnection,
    company_id: &str,
    sheets: &WorkbookSheets,
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let account_types = [
        SavingsAccountType::Principal,
        SavingsAccountType::Mandatory,
        SavingsAccountType::Voluntary,
    ];
    let mut running: HashMap<String, [i64; 3]> = HashMap::new();
    for row in &sheets.savings_2026 {
        if row.len() < 17 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let source_id = cell_text(&row[0]);
        let member_id = format!("{company_id}-{source_id}");
        if !member_ids.contains(&member_id) {
            continue;
        }
        let name = cell_text(&row[1]);
        let month_start = cell_date_iso(&row[2])?;
        let period: String = month_start.chars().take(7).collect();
        let account_id = |index: usize| format!("SA-{member_id}-{}", account_types[index].as_str());
        // Nilai negatif di sheet tidak bisa menjadi komponen; selisihnya masuk
        // ke penyesuaian di bawah.
        let money = |index: usize| cell_money(&row[index]).map(|value| value.max(0));

        if !running.contains_key(&member_id) {
            let opening = [money(4)?, money(5)?, money(6)?];
            import_entry(
                db,
                company_id,
                ImportEntry {
                    id: format!("{company_id}-MIG-SIM-{source_id}-OPENING"),
                    business_date: month_start.clone(),
                    member_id: Some(&member_id),
                    member_name: &name,
                    transaction_type: MIGRATED_SAVINGS,
                    channel: Channel::NonCash,
                    description: "Saldo awal simpanan 2026",
                    components: (0..3)
                        .map(|index| {
                            Component::new("SAVINGS_OPENING", "Saldo awal simpanan", opening[index])
                                .for_savings(account_id(index))
                        })
                        .collect(),
                },
            )
            .await?;
            running.insert(member_id.clone(), opening);
        }
        let balance = running.get_mut(&member_id).expect("inserted above");
        let incoming = [money(7)?, money(9)?, money(11)?];
        let outgoing = [money(8)?, money(10)?, money(12)?];
        let shu = money(13)?;
        let closing = [
            cell_money(&row[14])?,
            cell_money(&row[15])?,
            cell_money(&row[16])?,
        ];
        let mut components = Vec::new();
        for index in 0..3 {
            let account = account_id(index);
            components.push(
                Component::new("SAVINGS_DEPOSIT", "Setoran simpanan", incoming[index])
                    .for_savings(&account),
            );
            components.push(
                Component::new("SAVINGS_WITHDRAWAL", "Penarikan simpanan", outgoing[index])
                    .for_savings(&account),
            );
            let shu_part = if index == 2 {
                components.push(
                    Component::new("SAVINGS_SHU", "SHU / kredit internal", shu)
                        .for_savings(&account),
                );
                shu
            } else {
                0
            };
            let expected = balance[index] + incoming[index] - outgoing[index] + shu_part;
            let (component_type, amount) = balancing(
                closing[index] - expected,
                "SAVINGS_DEPOSIT",
                "SAVINGS_WITHDRAWAL",
            );
            components.push(
                Component::new(component_type, "Penyesuaian saldo sheet", amount)
                    .for_savings(&account),
            );
            balance[index] = closing[index];
        }
        import_entry(
            db,
            company_id,
            ImportEntry {
                id: format!("{company_id}-MIG-SIM-{source_id}-{period}"),
                business_date: history_date(&month_start, cell_optional_date_iso(&row[3])?),
                member_id: Some(&member_id),
                member_name: &name,
                transaction_type: MIGRATED_SAVINGS,
                channel: Channel::Deduction,
                description: "Mutasi simpanan (migrasi)",
                components,
            },
        )
        .await?;
    }
    Ok(())
}

/// Saldo awal + tunggakan awal, lalu mutasi bulanan dari PINJAMAN_2026.
async fn seed_loan_history(
    db: &mut SqliteConnection,
    company_id: &str,
    sheets: &WorkbookSheets,
    loan_members: &HashMap<String, Option<String>>,
) -> Result<(), String> {
    let mut running: HashMap<String, i64> = HashMap::new();
    for row in &sheets.loans_2026 {
        if row.len() < 23 || cell_text(&row[0]).is_empty() {
            continue;
        }
        let source_id = cell_text(&row[0]);
        let loan_id = format!("{company_id}-{source_id}");
        let Some(member_id) = loan_members.get(&loan_id) else {
            continue;
        };
        let name = cell_text(&row[1]);
        let month_start = cell_date_iso(&row[2])?;
        let period: String = month_start.chars().take(7).collect();
        let loan_component = |component_type, label: &str, amount| {
            Component::new(component_type, label, amount).for_loan(&loan_id)
        };

        if !running.contains_key(&loan_id) {
            let opening = cell_money(&row[9])?.max(0);
            let (principal_type, principal_amount) = balancing(
                cell_money(&row[16])?,
                "LOAN_OPENING_ARREARS_PRINCIPAL",
                "LOAN_OPENING_PREPAID_PRINCIPAL",
            );
            let (interest_type, interest_amount) = balancing(
                cell_money(&row[17])?,
                "LOAN_OPENING_ARREARS_INTEREST",
                "LOAN_OPENING_PREPAID_INTEREST",
            );
            import_entry(
                db,
                company_id,
                ImportEntry {
                    id: format!("{company_id}-MIG-PIN-{source_id}-OPENING"),
                    business_date: month_start.clone(),
                    member_id: member_id.as_deref(),
                    member_name: &name,
                    transaction_type: MIGRATED_LOAN,
                    channel: Channel::NonCash,
                    description: "Saldo awal pinjaman 2026",
                    components: vec![
                        loan_component("LOAN_OPENING", "Saldo awal pokok pinjaman", opening),
                        loan_component(principal_type, "Tunggakan pokok awal", principal_amount),
                        loan_component(interest_type, "Tunggakan bunga awal", interest_amount),
                    ],
                },
            )
            .await?;
            running.insert(loan_id.clone(), opening);
        }
        let balance = running.get_mut(&loan_id).expect("inserted above");
        // Nilai negatif di sheet tidak bisa menjadi komponen; selisihnya masuk
        // ke penyesuaian di bawah.
        let disbursed = cell_money(&row[10])?.max(0);
        let principal_paid = cell_money(&row[11])?.max(0);
        let closing = cell_money(&row[22])?;
        let (adjust_type, adjust_amount) = balancing(
            closing - (*balance + disbursed - principal_paid),
            "LOAN_DISBURSEMENT",
            "LOAN_PRINCIPAL",
        );
        *balance = closing;
        import_entry(
            db,
            company_id,
            ImportEntry {
                id: format!("{company_id}-MIG-PIN-{source_id}-{period}"),
                business_date: history_date(&month_start, cell_optional_date_iso(&row[3])?),
                member_id: member_id.as_deref(),
                member_name: &name,
                transaction_type: MIGRATED_LOAN,
                channel: Channel::Deduction,
                description: "Mutasi pinjaman (migrasi)",
                components: vec![
                    loan_component("LOAN_DISBURSEMENT", "Pencairan pokok pinjaman", disbursed),
                    loan_component("LOAN_PRINCIPAL", "Pokok pinjaman", principal_paid),
                    loan_component(
                        "LOAN_INTEREST",
                        "Bunga pinjaman",
                        cell_money(&row[12])?.max(0),
                    ),
                    loan_component(
                        "LOAN_PROVISION",
                        "Pendapatan provisi",
                        cell_money(&row[13])?.max(0),
                    ),
                    loan_component(adjust_type, "Penyesuaian saldo sheet", adjust_amount),
                ],
            },
        )
        .await?;
    }
    Ok(())
}

/// Saldo awal kas dan setiap baris Buku Kas Harian 2026.
async fn seed_cash_book(
    db: &mut SqliteConnection,
    company_id: &str,
    sheets: &WorkbookSheets,
) -> Result<(), String> {
    import_entry(
        db,
        company_id,
        ImportEntry {
            id: format!("{company_id}-OPENING-CASH-2026"),
            // Saldo akhir 2025 menjadi saldo awal buku kas 2026.
            business_date: "2025-12-31".into(),
            member_id: None,
            member_name: "Koperasi",
            transaction_type: "OPENING_BALANCE",
            channel: Channel::Cash,
            description: "Saldo Kas Fisik Neraca Posisi 31 Desember 2025",
            components: vec![Component::new(
                "CASH_OPENING",
                "Saldo awal kas",
                OPENING_CASH_2026,
            )],
        },
    )
    .await?;
    // Di bawah Buku Kas Harian ada catatan hitungan (pecahan uang, rincian
    // biaya RAT) yang dipisah beberapa baris kosong. Berhenti di celah pertama
    // yang lebih dari lima baris agar catatan itu tidak ikut menjadi transaksi.
    let mut previous_source_row: Option<i64> = None;
    for row in &sheets.cash_2026 {
        if row.len() < 11 {
            continue;
        }
        let source_row = cell_int(&row[9])?;
        if previous_source_row.is_some_and(|previous| source_row - previous > 5) {
            break;
        }
        previous_source_row = Some(source_row);
        let net_amount = cell_money(&row[5])?
            .checked_sub(cell_money(&row[4])?)
            .ok_or("Nilai mutasi kas terlalu besar.")?;
        if net_amount == 0 {
            continue;
        }
        let amount = net_amount
            .checked_abs()
            .ok_or("Nilai mutasi kas terlalu besar.")?;
        let business_date = cell_date_iso(&row[0])?;
        let description = cell_text(&row[1]);
        let display_date = display_date(&business_date);
        let id = format!("{company_id}-MIG-KAS-{}", cell_text(&row[9]));
        append(
            db,
            company_id,
            Entry {
                reference: &id,
                id: id.clone(),
                business_date: &business_date,
                display_date: &display_date,
                display_time: "00:00",
                member_id: None,
                member_name: "Koperasi",
                transaction_type: MIGRATED_CASH,
                channel: Channel::Cash,
                description: &description,
                direction: if net_amount > 0 {
                    TransactionDirection::In
                } else {
                    TransactionDirection::Out
                },
                actor: IMPORT_ACTOR,
                reversed_transaction_id: None,
                components: vec![Component::new(
                    "UNCLASSIFIED",
                    "Belum diklasifikasi",
                    amount,
                )],
            },
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn import_workbook(
    path: String,
    company_id: &str,
    pool: &SqlitePool,
) -> Result<AppSnapshot, String> {
    let path = std::path::PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Berkas workbook tidak dapat dibuka: {error}"))?;
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    if !path.is_file()
        || !matches!(
            extension.as_deref(),
            Some("xlsx" | "xls" | "xlsm" | "xlsb" | "ods")
        )
    {
        return Err("Berkas impor harus berupa workbook yang didukung.".into());
    }
    let already_seeded: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_meta WHERE company_id = ? AND key = 'seed_version'",
    )
    .bind(company_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    if already_seeded > 0 {
        return Err("Data sudah pernah diimpor untuk perusahaan ini.".into());
    }
    let sheets = tauri::async_runtime::spawn_blocking(move || load_workbook_sheets(&path))
        .await
        .map_err(|error| error.to_string())??;
    seed_database(pool, company_id, &sheets).await?;
    application::get_app_snapshot(company_id, pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn keeps_history_dates_inside_their_sheet_month() {
        assert_eq!(history_date("2026-09-01", String::new()), "2026-09-01");
        assert_eq!(
            history_date("2026-09-01", "2026-09-02".into()),
            "2026-09-02"
        );
        assert_eq!(
            history_date("2026-09-01", "2029-09-02".into()),
            "2026-09-02"
        );
        assert_eq!(
            history_date("2026-09-01", "2026-10-02".into()),
            "2026-09-01"
        );
    }

    // This test seeds an in-memory database from the real cooperative workbook.
    // The workbook contains real members' financial data and is intentionally
    // excluded from git (see .gitignore), so it only runs on machines that have
    // a copy of it under docs/.
    #[test]
    fn seeds_the_cleaned_workbook_into_a_balanced_ledger() {
        let workbook_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/KOPERASI_BINA_SEJAHTERA_2026_DIBERSIHKAN.xlsm"
        ))
        .to_path_buf();
        if !workbook_path.exists() {
            eprintln!(
                "skipping seeds_the_cleaned_workbook_into_a_balanced_ledger: {} not found locally",
                workbook_path.display()
            );
            return;
        }

        tauri::async_runtime::block_on(async {
            let sheets = load_workbook_sheets(&workbook_path).unwrap();
            let pool = crate::database::memory_pool().await;
            seed_database(&pool, "default", &sheets).await.unwrap();

            let snapshot = application::get_app_snapshot("default", &pool)
                .await
                .unwrap();
            assert_eq!(snapshot.members.len(), 144);
            assert_eq!(snapshot.loans.len(), 145);
            // Kas akhir = saldo awal 2025 + seluruh mutasi Buku Kas Harian
            // sampai 10 Sep 2026 (tanpa catatan hitungan di bawahnya).
            assert_eq!(snapshot.totals.cash, 40_895_664);
            // Saldo mengikuti sheet apa adanya, termasuk pokok -50.000 pada
            // empat anggota baru (M133-M136).
            assert_eq!(snapshot.totals.savings, 960_021_990);
            assert_eq!(snapshot.totals.loan_portfolio, 741_754_300);

            let ledger = application::get_monthly_ledger("default", 2026, &pool)
                .await
                .unwrap();
            let september_loans: i64 = ledger
                .loans
                .iter()
                .filter(|row| row.period == "2026-09")
                .map(|row| row.closing_balance)
                .sum();
            assert_eq!(september_loans, 741_754_300);

            let cash_book = application::get_cash_book("default", 2026, "KAS", &pool)
                .await
                .unwrap();
            assert_eq!(cash_book.opening_balance, OPENING_CASH_2026);
            assert_eq!(cash_book.closing_balance, 40_895_664);

            let foreign_key_errors: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(foreign_key_errors, 0);

            // Buku besar hanya bisa ditambah.
            assert!(sqlx::query("UPDATE transactions SET amount = 1")
                .execute(&pool)
                .await
                .is_err());
            assert!(sqlx::query("DELETE FROM transaction_components")
                .execute(&pool)
                .await
                .is_err());
        });
    }
}
