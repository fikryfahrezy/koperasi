use calamine::{open_workbook_auto, Data, DataType, Reader};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};
use std::{
    collections::HashMap,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};

struct WorkbookSheets {
    master_savings: Vec<Vec<Data>>,
    savings_2026: Vec<Vec<Data>>,
    master_loans: Vec<Vec<Data>>,
    loans_2026: Vec<Vec<Data>>,
    cash_2026: Vec<Vec<Data>>,
}

fn load_workbook_sheets(path: &Path) -> Result<WorkbookSheets, String> {
    let mut workbook = open_workbook_auto(path).map_err(|error| error.to_string())?;

    let mut sheet_rows = |name: &str| -> Result<Vec<Vec<Data>>, String> {
        let range = workbook
            .worksheet_range(name)
            .map_err(|_| format!("Sheet '{name}' tidak ditemukan dalam workbook"))?;
        Ok(range.rows().skip(1).map(|row| row.to_vec()).collect())
    };

    Ok(WorkbookSheets {
        master_savings: sheet_rows("MASTER_SIMPANAN")?,
        savings_2026: sheet_rows("SIMPANAN_2026")?,
        master_loans: sheet_rows("MASTER_PINJAMAN")?,
        loans_2026: sheet_rows("PINJAMAN_2026")?,
        cash_2026: sheet_rows("KAS_2026")?,
    })
}

struct AppState {
    db: SqlitePool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemberDto {
    id: String,
    member_number: String,
    name: String,
    joined_at: String,
    status: String,
    savings: i64,
    principal_savings: i64,
    mandatory_savings: i64,
    voluntary_savings: i64,
    loan_balance: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoanDto {
    id: String,
    member_id: String,
    member_name: String,
    plafond: i64,
    balance: i64,
    rate: f64,
    tenor: i64,
    interest_type: String,
    realization_date: String,
    due_date: String,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentDto {
    label: String,
    amount: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDto {
    id: String,
    date: String,
    time: String,
    member_name: String,
    description: String,
    reference: String,
    direction: String,
    amount: i64,
    status: String,
    components: Vec<ComponentDto>,
    actor: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TotalsDto {
    cash: i64,
    savings: i64,
    loan_portfolio: i64,
    members: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    members: Vec<MemberDto>,
    loans: Vec<LoanDto>,
    transactions: Vec<TransactionDto>,
    totals: TotalsDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddMemberInput {
    name: String,
    member_number: String,
    joined_at: String,
    principal_savings: i64,
    business_date: String,
    display_date: String,
    display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLoanInput {
    member_id: String,
    plafond: i64,
    tenor: i64,
    interest_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentInput {
    member_id: String,
    principal: i64,
    interest: i64,
    wajib: i64,
    voluntary: i64,
    reference: String,
    business_date: String,
    display_date: String,
    display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavingsTransactionInput {
    member_id: String,
    account_type: String,
    movement: String,
    amount: i64,
    reference: String,
    business_date: String,
    display_date: String,
    display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisburseLoanInput {
    loan_id: String,
    business_date: String,
    display_date: String,
    display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReverseTransactionInput {
    id: String,
    business_date: String,
    display_date: String,
    display_time: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveParametersInput {
    principal_savings: i64,
    mandatory_savings: i64,
    provision_rate: f64,
    annual_rate: f64,
    effective_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FinancialParametersDto {
    principal_savings: i64,
    mandatory_savings: i64,
    provision_rate: f64,
    annual_rate: f64,
    effective_date: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditEventDto {
    id: i64,
    entity_type: String,
    entity_id: String,
    action: String,
    actor: String,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminStateDto {
    parameters: FinancialParametersDto,
    audit_events: Vec<AuditEventDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoanPreview {
    principal_installment: i64,
    first_interest: i64,
    provision: i64,
    first_total: i64,
    annual_rate: f64,
}

fn timestamp_id(prefix: &str) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis}-{sequence}")
}

fn money(value: &str) -> i64 {
    if value.trim().is_empty() || value.trim() == "-" || value.trim() == "`" {
        return 0;
    }
    let negative = value.contains('(');
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    let parsed = digits.parse::<i64>().unwrap_or(0);
    if negative {
        -parsed
    } else {
        parsed
    }
}

fn percentage(value: &str) -> f64 {
    value
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or(0.0)
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

// A workbook cell is either already text (member names, statuses, loan ids, ...)
// or a raw calamine-typed number/date; these helpers read either shape so the
// import logic doesn't care how a given cell happens to be stored in the .xlsm.
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) if value.fract() == 0.0 => format!("{}", *value as i64),
        Data::Float(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

fn cell_money(cell: &Data) -> i64 {
    match cell {
        Data::Float(value) => value.round() as i64,
        Data::Int(value) => *value,
        Data::String(value) => money(value),
        _ => 0,
    }
}

// Excel stores a percentage such as 24% as the fraction 0.24; the rest of the
// app (loans.rate_annual, LoanPreview) works with the whole percentage number.
fn cell_percentage(cell: &Data) -> f64 {
    match cell {
        Data::Float(value) => value * 100.0,
        Data::Int(value) => *value as f64 * 100.0,
        Data::String(value) => percentage(value),
        _ => 0.0,
    }
}

fn cell_int(cell: &Data) -> i64 {
    match cell {
        Data::Float(value) => value.round() as i64,
        Data::Int(value) => *value,
        Data::String(value) => value.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

fn cell_date_iso(cell: &Data) -> String {
    cell.as_datetime()
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "2026-01-01".to_string())
}

fn cell_date_display(cell: &Data) -> String {
    cell.as_datetime()
        .map(|value| value.format("%d-%b-%Y").to_string())
        .unwrap_or_default()
}

fn add_months(value: &str, months: i64) -> Result<String, String> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err("Tanggal bisnis tidak valid.".into());
    }
    let year = parts[0]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    let month = parts[1]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    let day = parts[2]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err("Tanggal bisnis tidak valid.".into());
    }
    let absolute_month = year * 12 + month - 1 + months;
    let due_year = absolute_month.div_euclid(12);
    let due_month = absolute_month.rem_euclid(12) + 1;
    let leap = due_year % 4 == 0 && (due_year % 100 != 0 || due_year % 400 == 0);
    let max_day = match due_month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    Ok(format!(
        "{due_year:04}-{due_month:02}-{:02}",
        day.min(max_day)
    ))
}

fn calculate_loan_preview(
    plafond: i64,
    tenor: i64,
    annual_rate: f64,
    provision_rate: f64,
) -> LoanPreview {
    let principal_installment = ((plafond as f64 / tenor as f64) / 1000.0).ceil() as i64 * 1000;
    let first_interest = (plafond as f64 * annual_rate / 12.0).round() as i64;
    let provision = (plafond as f64 * provision_rate).round() as i64;
    LoanPreview {
        principal_installment,
        first_interest,
        provision,
        first_total: principal_installment + first_interest,
        annual_rate: annual_rate * 100.0,
    }
}

async fn initialize_database(app: &tauri::App) -> Result<SqlitePool, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&app_dir).map_err(|error| error.to_string())?;
    let options = SqliteConnectOptions::new()
        .filename(app_dir.join("koperasi-v2.db"))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::raw_sql(include_str!("../migrations/001_backend.sql"))
        .execute(&pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(pool)
}

async fn seed_database(pool: &SqlitePool, sheets: &WorkbookSheets) -> Result<(), String> {
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
        let balance = latest_loan_balance.get(&loan_id).copied().unwrap_or(plafond);
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

#[tauri::command]
async fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    snapshot(&state.db).await
}

#[tauri::command]
async fn import_workbook(path: String, state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    let already_seeded: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM app_meta WHERE key = 'seed_version'")
            .fetch_one(&state.db)
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
    seed_database(&state.db, &sheets).await?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn add_member(
    input: AddMemberInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    if input.name.trim().is_empty() || input.member_number.trim().is_empty() {
        return Err("Nama dan nomor anggota wajib diisi.".into());
    }
    if input.principal_savings < 50_000 {
        return Err("Simpanan pokok minimal Rp50.000.".into());
    }
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM members WHERE UPPER(member_number) = UPPER(?)")
            .bind(input.member_number.trim())
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor anggota sudah digunakan.".into());
    }
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(id, 2) AS INTEGER)), 0) + 1 FROM members WHERE id GLOB 'M[0-9]*'",
    )
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let id = format!("M{next:03}");
    sqlx::query("INSERT INTO members (id, member_number, name, joined_at, status) VALUES (?, ?, ?, ?, 'Aktif')")
        .bind(&id).bind(input.member_number.trim()).bind(input.name.trim()).bind(input.joined_at.trim()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for kind in ["POKOK", "WAJIB", "MANASUKA"] {
        sqlx::query("INSERT INTO savings_accounts (id, member_id, account_type, balance) VALUES (?, ?, ?, ?)")
            .bind(format!("SA-{id}-{kind}")).bind(&id).bind(kind).bind(0_i64).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    let transaction_id = timestamp_id("SAV-IN");
    let account_id = format!("SA-{id}-POKOK");
    let reference = format!("KBS/SAV/{transaction_id}");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&transaction_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&id)
        .bind(input.name.trim())
        .bind(&reference)
        .bind(input.principal_savings)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, 'SAVINGS_DEPOSIT', 'Setoran simpanan pokok', ?, ?)")
        .bind(&transaction_id)
        .bind(input.principal_savings)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE savings_accounts SET balance = ? WHERE id = ?")
        .bind(input.principal_savings)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
    )
    .bind(&transaction_id)
    .bind(input.principal_savings)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(&transaction_id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": "Setoran", "accountType": "POKOK", "amount": input.principal_savings, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('MEMBER', ?, 'CREATED', ?, 'Aplikasi lokal')")
        .bind(&id).bind(serde_json::to_string(&serde_json::json!({"name": input.name, "memberNumber": input.member_number, "principalSavings": input.principal_savings})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn preview_loan(
    input: CreateLoanInput,
    state: State<'_, AppState>,
) -> Result<LoanPreview, String> {
    if input.plafond <= 0 || input.tenor <= 0 {
        return Err("Plafond dan tenor harus lebih dari nol.".into());
    }
    let annual_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(&state.db).await.map_err(|error| error.to_string())?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(&state.db).await.map_err(|error| error.to_string())?;
    Ok(calculate_loan_preview(
        input.plafond,
        input.tenor,
        annual_rate,
        provision_rate,
    ))
}

#[tauri::command]
async fn create_loan(
    input: CreateLoanInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    if input.plafond <= 0
        || input.tenor <= 0
        || !matches!(input.interest_type.as_str(), "Menurun" | "Flat")
    {
        return Err("Data pinjaman tidak valid.".into());
    }
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Anggota aktif tidak ditemukan.")?;
    let rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_ANNUAL_RATE' AND effective_date <= '2026-09-13' ORDER BY effective_date DESC LIMIT 1")
        .fetch_one(&mut *db).await.map_err(|error| error.to_string())?;
    let id = timestamp_id("LOAN");
    sqlx::query("INSERT INTO loans (id, member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES (?, ?, ?, ?, 0, ?, ?, ?, '-', '-', 'Draf')")
        .bind(&id).bind(&input.member_id).bind(&member_name).bind(input.plafond).bind(rate * 100.0).bind(input.tenor).bind(&input.interest_type).execute(&mut *db).await.map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('LOAN', ?, 'DRAFT_CREATED', ?, 'Aplikasi lokal')")
        .bind(&id).bind(serde_json::to_string(&serde_json::json!({"memberId": input.member_id, "plafond": input.plafond, "tenor": input.tenor, "interestType": input.interest_type})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn disburse_loan(
    input: DisburseLoanInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    let period = input
        .business_date
        .get(0..7)
        .ok_or("Tanggal bisnis tidak valid.")?;
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM periods WHERE period = ? AND status = 'LOCKED'")
            .bind(period)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }

    let loan = sqlx::query(
        "SELECT member_id, member_name, plafond, tenor, status, balance FROM loans WHERE id = ?",
    )
    .bind(&input.loan_id)
    .fetch_optional(&mut *db)
    .await
    .map_err(|error| error.to_string())?
    .ok_or("Pinjaman tidak ditemukan.")?;
    let status: String = loan.get("status");
    let balance: i64 = loan.get("balance");
    if status != "Draf" || balance != 0 {
        return Err("Hanya pinjaman draf dengan saldo nol yang dapat dicairkan.".into());
    }
    let member_id: Option<String> = loan.get("member_id");
    let member_name: String = loan.get("member_name");
    let plafond: i64 = loan.get("plafond");
    let tenor: i64 = loan.get("tenor");
    let due_date = add_months(&input.business_date, tenor)?;
    let provision_rate: f64 = sqlx::query_scalar("SELECT value FROM parameters WHERE parameter_key = 'LOAN_PROVISION_RATE' AND effective_date <= ? ORDER BY effective_date DESC LIMIT 1")
        .bind(&input.business_date)
        .fetch_one(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let provision = (plafond as f64 * provision_rate).round() as i64;

    let disbursement_id = timestamp_id("DISB");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'LOAN_DISBURSEMENT', 'Pencairan pinjaman', ?, 'Keluar', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&disbursement_id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&member_id)
        .bind(&member_name)
        .bind(format!("KBS/DISB/{}/{}", input.loan_id, disbursement_id))
        .bind(plafond)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_DISBURSEMENT', 'Pencairan pokok pinjaman', ?, ?)")
        .bind(&disbursement_id)
        .bind(plafond)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Keluar', ?)",
    )
    .bind(&disbursement_id)
    .bind(plafond)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;

    if provision > 0 {
        let provision_id = timestamp_id("PROV");
        sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'LOAN_PROVISION', 'Provisi pencairan pinjaman', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
            .bind(&provision_id)
            .bind(&input.business_date)
            .bind(&input.display_date)
            .bind(&input.display_time)
            .bind(&member_id)
            .bind(&member_name)
            .bind(format!("KBS/PROV/{}/{}", input.loan_id, provision_id))
            .bind(provision)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_PROVISION', 'Pendapatan provisi', ?, ?)")
            .bind(&provision_id)
            .bind(provision)
            .bind(&input.loan_id)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
        sqlx::query(
            "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
        )
        .bind(&provision_id)
        .bind(provision)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    }

    sqlx::query("UPDATE loans SET balance = plafond, realization_date = ?, due_date = ?, status = 'Berjalan' WHERE id = ?")
        .bind(&input.business_date)
        .bind(&due_date)
        .bind(&input.loan_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('LOAN', ?, 'DISBURSED', ?, 'Aplikasi lokal')")
        .bind(&input.loan_id)
        .bind(serde_json::to_string(&serde_json::json!({"plafond": plafond, "provision": provision, "dueDate": due_date})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn post_savings_transaction(
    input: SavingsTransactionInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    if input.amount <= 0 {
        return Err("Nominal transaksi harus lebih dari nol.".into());
    }
    if !matches!(input.account_type.as_str(), "POKOK" | "WAJIB" | "MANASUKA") {
        return Err("Jenis simpanan tidak valid.".into());
    }
    if !matches!(input.movement.as_str(), "Setoran" | "Penarikan") {
        return Err("Jenis mutasi tidak valid.".into());
    }
    if input.movement == "Penarikan" && input.account_type != "MANASUKA" {
        return Err("Hanya simpanan manasuka yang dapat ditarik pada versi ini.".into());
    }
    let period = input
        .business_date
        .get(0..7)
        .ok_or("Tanggal bisnis tidak valid.")?;
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM periods WHERE period = ? AND status = 'LOCKED'")
            .bind(period)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
            .fetch_optional(&mut *db)
            .await
            .map_err(|error| error.to_string())?
            .ok_or("Anggota aktif tidak ditemukan.")?;
    let account = sqlx::query(
        "SELECT id, balance FROM savings_accounts WHERE member_id = ? AND account_type = ?",
    )
    .bind(&input.member_id)
    .bind(&input.account_type)
    .fetch_one(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    let account_id: String = account.get("id");
    let balance: i64 = account.get("balance");
    if input.movement == "Penarikan" && balance < input.amount {
        return Err("Saldo manasuka tidak mencukupi.".into());
    }

    let id = timestamp_id(if input.movement == "Setoran" {
        "SAV-IN"
    } else {
        "SAV-OUT"
    });
    let reference = if input.reference.trim().is_empty() {
        format!("KBS/SAV/{id}")
    } else {
        input.reference.trim().to_string()
    };
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE reference = ?")
            .bind(&reference)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }
    let is_deposit = input.movement == "Setoran";
    let direction = if is_deposit { "Masuk" } else { "Keluar" };
    let transaction_type = if is_deposit {
        "SAVINGS_DEPOSIT"
    } else {
        "SAVINGS_WITHDRAWAL"
    };
    let component_type = transaction_type;
    let description = format!(
        "{} simpanan {}",
        input.movement,
        input.account_type.to_lowercase()
    );
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&id)
        .bind(&input.business_date)
        .bind(&input.display_date)
        .bind(&input.display_time)
        .bind(&input.member_id)
        .bind(&member_name)
        .bind(transaction_type)
        .bind(&description)
        .bind(&reference)
        .bind(direction)
        .bind(input.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(component_type)
        .bind(&description)
        .bind(input.amount)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let delta = if is_deposit {
        input.amount
    } else {
        -input.amount
    };
    sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
        .bind(delta)
        .bind(&account_id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(direction)
        .bind(input.amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')")
        .bind(&id)
        .bind(serde_json::to_string(&serde_json::json!({"movement": input.movement, "accountType": input.account_type, "amount": input.amount, "reference": reference})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn post_payment(
    input: PaymentInput,
    state: State<'_, AppState>,
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
    let amount = input.principal + input.interest + input.wajib + input.voluntary;
    if amount <= 0 {
        return Err("Total pembayaran harus lebih dari nol.".into());
    }
    let period = input
        .business_date
        .get(0..7)
        .ok_or("Tanggal bisnis tidak valid.")?;
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM periods WHERE period = ? AND status = 'LOCKED'")
            .bind(period)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let member_name: String =
        sqlx::query_scalar("SELECT name FROM members WHERE id = ? AND status = 'Aktif'")
            .bind(&input.member_id)
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
    let duplicate: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE reference = ?")
            .bind(&reference)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if duplicate > 0 {
        return Err("Nomor referensi sudah pernah diposting.".into());
    }

    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, actor) VALUES (?, ?, ?, ?, ?, ?, 'MEMBER_PAYMENT', 'Pembayaran anggota', ?, 'Masuk', ?, 'Terposting', 'Aplikasi lokal')")
        .bind(&id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(&input.member_id).bind(&member_name).bind(&reference).bind(amount).execute(&mut *db).await.map_err(|error| error.to_string())?;

    let mut remaining_principal = input.principal;
    let loan_rows = sqlx::query("SELECT id, balance FROM loans WHERE member_id = ? AND status = 'Berjalan' AND balance > 0 ORDER BY realization_date, id")
        .bind(&input.member_id).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for loan in loan_rows {
        if remaining_principal == 0 {
            break;
        }
        let loan_id: String = loan.get("id");
        let balance: i64 = loan.get("balance");
        let allocated = remaining_principal.min(balance);
        sqlx::query("UPDATE loans SET balance = balance - ?, status = CASE WHEN balance - ? = 0 THEN 'Lunas' ELSE status END WHERE id = ?")
            .bind(allocated).bind(allocated).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id) VALUES (?, 'LOAN_PRINCIPAL', 'Pokok pinjaman', ?, ?)")
            .bind(&id).bind(allocated).bind(&loan_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        remaining_principal -= allocated;
    }
    if remaining_principal > 0 {
        return Err("Pembayaran pokok melebihi saldo pinjaman berjalan.".into());
    }

    if input.interest > 0 {
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount) VALUES (?, 'LOAN_INTEREST', 'Bunga pinjaman', ?)").bind(&id).bind(input.interest).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    for (kind, component_type, label, value) in [
        ("WAJIB", "SAVINGS_WAJIB", "Simpanan wajib", input.wajib),
        (
            "MANASUKA",
            "SAVINGS_VOLUNTARY",
            "Simpanan manasuka",
            input.voluntary,
        ),
    ] {
        if value > 0 {
            let account_id: String = sqlx::query_scalar(
                "SELECT id FROM savings_accounts WHERE member_id = ? AND account_type = ?",
            )
            .bind(&input.member_id)
            .bind(kind)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
                .bind(value)
                .bind(&account_id)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, savings_account_id) VALUES (?, ?, ?, ?, ?)").bind(&id).bind(component_type).bind(label).bind(value).bind(account_id).execute(&mut *db).await.map_err(|error| error.to_string())?;
        }
    }
    sqlx::query(
        "INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, 'Masuk', ?)",
    )
    .bind(&id)
    .bind(amount)
    .execute(&mut *db)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('TRANSACTION', ?, 'POSTED', ?, 'Aplikasi lokal')").bind(&id).bind(serde_json::to_string(&serde_json::json!({"amount": amount, "reference": reference})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

#[tauri::command]
async fn reverse_transaction(
    input: ReverseTransactionInput,
    state: State<'_, AppState>,
) -> Result<AppSnapshot, String> {
    let period = input
        .business_date
        .get(0..7)
        .ok_or("Tanggal bisnis tidak valid.")?;
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let locked: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM periods WHERE period = ? AND status = 'LOCKED'")
            .bind(period)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if locked > 0 {
        return Err("Periode transaksi sudah dikunci.".into());
    }
    let id = &input.id;
    let original = sqlx::query("SELECT business_date, display_date, display_time, member_id, member_name, description, reference, direction, amount, status FROM transactions WHERE id = ?")
        .bind(id).fetch_optional(&mut *db).await.map_err(|error| error.to_string())?.ok_or("Transaksi tidak ditemukan.")?;
    let status: String = original.get("status");
    if status != "Terposting" {
        return Err("Hanya transaksi terposting yang dapat dibalik.".into());
    }
    let components = sqlx::query("SELECT component_type, label, amount, loan_id, savings_account_id FROM transaction_components WHERE transaction_id = ?")
        .bind(id).fetch_all(&mut *db).await.map_err(|error| error.to_string())?;
    for component in &components {
        let component_type: String = component.get("component_type");
        let value: i64 = component.get("amount");
        if component_type == "LOAN_PRINCIPAL" {
            let loan_id: Option<String> = component.try_get("loan_id").ok();
            if let Some(loan_id) = loan_id {
                sqlx::query(
                    "UPDATE loans SET balance = balance + ?, status = 'Berjalan' WHERE id = ?",
                )
                .bind(value)
                .bind(loan_id)
                .execute(&mut *db)
                .await
                .map_err(|error| error.to_string())?;
            }
        } else if component_type == "LOAN_DISBURSEMENT" {
            let loan_id: Option<String> = component.try_get("loan_id").ok();
            if let Some(loan_id) = loan_id {
                let result = sqlx::query("UPDATE loans SET balance = balance - ?, status = 'Draf', realization_date = '-', due_date = '-' WHERE id = ? AND balance = ?")
                    .bind(value)
                    .bind(loan_id)
                    .bind(value)
                    .execute(&mut *db)
                    .await
                    .map_err(|error| error.to_string())?;
                if result.rows_affected() == 0 {
                    return Err(
                        "Pencairan tidak dapat dibalik setelah pinjaman memiliki pembayaran."
                            .into(),
                    );
                }
            }
        } else if component_type.starts_with("SAVINGS_") {
            let account_id: Option<String> = component.try_get("savings_account_id").ok();
            if let Some(account_id) = account_id {
                let result = if component_type == "SAVINGS_WITHDRAWAL" {
                    sqlx::query("UPDATE savings_accounts SET balance = balance + ? WHERE id = ?")
                        .bind(value)
                        .bind(account_id)
                        .execute(&mut *db)
                        .await
                        .map_err(|error| error.to_string())?
                } else {
                    sqlx::query("UPDATE savings_accounts SET balance = balance - ? WHERE id = ? AND balance >= ?")
                        .bind(value)
                        .bind(account_id)
                        .bind(value)
                        .execute(&mut *db)
                        .await
                        .map_err(|error| error.to_string())?
                };
                if result.rows_affected() == 0 {
                    return Err("Reversal ditolak karena saldo sub-ledger tidak mencukupi.".into());
                }
            }
        }
    }
    sqlx::query("UPDATE transactions SET status = 'Dibalik' WHERE id = ?")
        .bind(id)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    let reversal_id = timestamp_id("REV");
    let direction: String = original.get("direction");
    let reversal_direction = if direction == "Masuk" {
        "Keluar"
    } else {
        "Masuk"
    };
    let reference: String = original.get("reference");
    let amount: i64 = original.get("amount");
    sqlx::query("INSERT INTO transactions (id, business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, reversed_transaction_id, actor) VALUES (?, ?, ?, ?, ?, ?, 'REVERSAL', ?, ?, ?, ?, 'Terposting', ?, 'Aplikasi lokal')")
        .bind(&reversal_id).bind(&input.business_date).bind(&input.display_date).bind(&input.display_time).bind(original.get::<Option<String>, _>("member_id")).bind(original.get::<String, _>("member_name")).bind(format!("Reversal · {}", original.get::<String, _>("description"))).bind(format!("{reference}/REV/{}", timestamp_id("R"))).bind(reversal_direction).bind(amount).bind(id).execute(&mut *db).await.map_err(|error| error.to_string())?;
    for component in components {
        sqlx::query("INSERT INTO transaction_components (transaction_id, component_type, label, amount, loan_id, savings_account_id) VALUES (?, 'REVERSAL', ?, ?, ?, ?)")
            .bind(&reversal_id).bind(format!("Reversal · {}", component.get::<String, _>("label"))).bind(component.get::<i64, _>("amount")).bind(component.get::<Option<String>, _>("loan_id")).bind(component.get::<Option<String>, _>("savings_account_id")).execute(&mut *db).await.map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO cash_postings (transaction_id, direction, amount) VALUES (?, ?, ?)")
        .bind(&reversal_id)
        .bind(reversal_direction)
        .bind(amount)
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, before_json, after_json, actor) VALUES ('TRANSACTION', ?, 'REVERSED', ?, ?, 'Aplikasi lokal')")
        .bind(id).bind(serde_json::to_string(&serde_json::json!({"status": "Terposting"})).unwrap_or_default()).bind(serde_json::to_string(&serde_json::json!({"status": "Dibalik", "reversalId": reversal_id})).unwrap_or_default()).execute(&mut *db).await.map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    snapshot(&state.db).await
}

async fn admin_state(pool: &SqlitePool) -> Result<AdminStateDto, String> {
    let parameter_rows = sqlx::query("SELECT parameter_key, value, effective_date FROM parameters WHERE effective_date = (SELECT MAX(effective_date) FROM parameters WHERE effective_date <= '2026-09-13')")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let mut values = HashMap::new();
    let mut effective_date = String::new();
    for row in parameter_rows {
        values.insert(
            row.get::<String, _>("parameter_key"),
            row.get::<f64, _>("value"),
        );
        effective_date = row.get("effective_date");
    }
    let parameters = FinancialParametersDto {
        principal_savings: values.get("SAVINGS_PRINCIPAL").copied().unwrap_or(0.0) as i64,
        mandatory_savings: values.get("SAVINGS_MONTHLY").copied().unwrap_or(0.0) as i64,
        provision_rate: values.get("LOAN_PROVISION_RATE").copied().unwrap_or(0.0) * 100.0,
        annual_rate: values.get("LOAN_ANNUAL_RATE").copied().unwrap_or(0.0) * 100.0,
        effective_date,
    };
    let audit_rows = sqlx::query("SELECT id, entity_type, entity_id, action, actor, created_at FROM audit_events ORDER BY id DESC LIMIT 50")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    let audit_events = audit_rows
        .into_iter()
        .map(|row| AuditEventDto {
            id: row.get("id"),
            entity_type: row.get("entity_type"),
            entity_id: row.get("entity_id"),
            action: row.get("action"),
            actor: row.get("actor"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(AdminStateDto {
        parameters,
        audit_events,
    })
}

#[tauri::command]
async fn get_admin_state(state: State<'_, AppState>) -> Result<AdminStateDto, String> {
    admin_state(&state.db).await
}

#[tauri::command]
async fn save_financial_parameters(
    input: SaveParametersInput,
    state: State<'_, AppState>,
) -> Result<AdminStateDto, String> {
    if input.principal_savings < 0
        || input.mandatory_savings < 0
        || input.provision_rate < 0.0
        || input.annual_rate < 0.0
        || input.effective_date.len() != 10
    {
        return Err("Nilai atau tanggal berlaku parameter tidak valid.".into());
    }
    let mut db = state.db.begin().await.map_err(|error| error.to_string())?;
    let existing: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM parameters WHERE effective_date = ?")
            .bind(&input.effective_date)
            .fetch_one(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    if existing > 0 {
        return Err("Versi parameter untuk tanggal berlaku tersebut sudah ada.".into());
    }
    for (key, value) in [
        ("SAVINGS_PRINCIPAL", input.principal_savings as f64),
        ("SAVINGS_MONTHLY", input.mandatory_savings as f64),
        ("LOAN_PROVISION_RATE", input.provision_rate / 100.0),
        ("LOAN_ANNUAL_RATE", input.annual_rate / 100.0),
    ] {
        sqlx::query("INSERT INTO parameters (parameter_key, value, effective_date, created_by) VALUES (?, ?, ?, 'Aplikasi lokal')")
            .bind(key)
            .bind(value)
            .bind(&input.effective_date)
            .execute(&mut *db)
            .await
            .map_err(|error| error.to_string())?;
    }
    sqlx::query("INSERT INTO audit_events (entity_type, entity_id, action, after_json, actor) VALUES ('PARAMETERS', ?, 'VERSION_CREATED', ?, 'Aplikasi lokal')")
        .bind(&input.effective_date)
        .bind(serde_json::to_string(&serde_json::json!({"principalSavings": input.principal_savings, "mandatorySavings": input.mandatory_savings, "provisionRate": input.provision_rate, "annualRate": input.annual_rate})).unwrap_or_default())
        .execute(&mut *db)
        .await
        .map_err(|error| error.to_string())?;
    db.commit().await.map_err(|error| error.to_string())?;
    admin_state(&state.db).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indonesian_money_and_source_corrections() {
        assert_eq!(money("1,250,000"), 1_250_000);
        assert_eq!(money("(12,950,000)"), -12_950_000);
        assert_eq!(money("-"), 0);
    }

    #[test]
    fn converts_and_advances_business_dates() {
        assert_eq!(add_months("2026-01-31", 1).unwrap(), "2026-02-28");
        assert_eq!(add_months("2024-01-31", 1).unwrap(), "2024-02-29");
    }

    #[test]
    fn calculates_parameterized_loan_preview() {
        let preview = calculate_loan_preview(10_000_000, 24, 0.24, 0.01);
        assert_eq!(preview.principal_installment, 417_000);
        assert_eq!(preview.first_interest, 200_000);
        assert_eq!(preview.provision, 100_000);
        assert_eq!(preview.first_total, 617_000);
        assert_eq!(preview.annual_rate, 24.0);
    }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(initialize_database(app))
                .map_err(std::io::Error::other)?;
            app.manage(AppState { db: pool });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_upload::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            import_workbook,
            add_member,
            preview_loan,
            create_loan,
            disburse_loan,
            post_savings_transaction,
            post_payment,
            reverse_transaction,
            get_admin_state,
            save_financial_parameters
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
