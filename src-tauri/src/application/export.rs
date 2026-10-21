//! Excel exports of the Buku Kas, Simpanan and Pinjaman pages, laid out like
//! the cooperative's workbooks (BUKU KAS HARIAN sheet TAHUN 2026, DATA SIMPANAN
//! sheet SIMPANAN, DATA PINJAMAN sheet PINJAMAN BULANAN) with their formulas.
//!
//! Every derived cell is written as the sheet's formula when that formula gives
//! the value the page shows; where the app's rules go beyond the sheet (e.g. the
//! income of a replaced loan moving to the new loan), the cell holds the value.

use std::collections::{BTreeSet, HashMap};

use sqlx::{Row, SqlitePool};

use super::{get_app_snapshot, get_cash_book, get_monthly_ledger, list_companies};
use crate::contracts::{CashBookDto, LoanDto, LoanMonthDto, MonthlyLedgerDto, SavingsMonthDto};
use crate::domain::{
    Channel, InterestType, LoanGroup, TransactionDirection, TransactionStatus, CASH_CATEGORIES,
};
use crate::xlsx::{Expr, Sheet, Style};

const MONTHS: [&str; 12] = [
    "JANUARI",
    "FEBRUARI",
    "MARET",
    "APRIL",
    "MEI",
    "JUNI",
    "JULI",
    "AGUSTUS",
    "SEPTEMBER",
    "OKTOBER",
    "NOVEMBER",
    "DESEMBER",
];

/// "2026-09" -> "SEPTEMBER 2026".
fn month_label(period: &str) -> String {
    let year = period.get(0..4).unwrap_or_default();
    let month: usize = period.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(0);
    match MONTHS.get(month.wrapping_sub(1)) {
        Some(name) => format!("{name} {year}"),
        None => period.to_string(),
    }
}

/// The month before (`delta` = -1) or after (`delta` = 1) a `YYYY-MM` period.
fn shift_period(period: &str, delta: i32) -> String {
    let year: i32 = period.get(0..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    let month: i32 = period.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    let index = year * 12 + (month - 1) + delta;
    format!(
        "{:04}-{:02}",
        index.div_euclid(12),
        index.rem_euclid(12) + 1
    )
}

fn quarter_of(period: &str) -> usize {
    let month: usize = period.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    (month.clamp(1, 12) - 1) / 3
}

/// `YYYY-MM-DD` for a date stored as `YYYY-MM-DD` or `DD-Mon-YYYY` (imported rows).
fn iso_date(value: &str) -> String {
    const SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = value.split('-');
    if let (Some(day), Some(month), Some(year), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    {
        if let (Ok(day), Some(month)) = (
            day.parse::<u32>(),
            SHORT.iter().position(|name| *name == month),
        ) {
            if year.len() == 4 {
                return format!("{year}-{:02}-{day:02}", month + 1);
            }
        }
    }
    value.to_string()
}

async fn company_name(company_id: &str, pool: &SqlitePool) -> Result<String, String> {
    Ok(list_companies(pool)
        .await?
        .into_iter()
        .find(|company| company.id == company_id)
        .map(|company| company.name)
        .unwrap_or_default())
}

fn year_periods(ledger: &MonthlyLedgerDto, year: i32) -> Vec<String> {
    let prefix = format!("{year:04}-");
    let mut periods: Vec<String> = ledger
        .periods
        .iter()
        .filter(|period| period.starts_with(&prefix))
        .cloned()
        .collect();
    periods.sort();
    periods
}

/// The page to export.
pub(crate) enum ExportKind {
    CashBook,
    Savings,
    Loans,
}

impl ExportKind {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "cash-book" => Ok(Self::CashBook),
            "savings" => Ok(Self::Savings),
            "loans" => Ok(Self::Loans),
            _ => Err(format!("Jenis ekspor tidak valid: {value}")),
        }
    }

    /// Suggested file name, after the cooperative's workbooks.
    pub(crate) fn file_name(&self, year: i32) -> String {
        match self {
            Self::CashBook => format!("BUKU KAS HARIAN TAHUN {year}.xlsx"),
            Self::Savings => format!("DATA SIMPANAN TAHUN {year}.xlsx"),
            Self::Loans => format!("DATA PINJAMAN TAHUN {year}.xlsx"),
        }
    }
}

/// The .xlsx bytes of a page for the year.
pub(crate) async fn export_workbook(
    kind: &ExportKind,
    company_id: &str,
    year: i32,
    through: Option<&str>,
    pool: &SqlitePool,
) -> Result<Vec<u8>, String> {
    let sheet = match kind {
        ExportKind::CashBook => {
            let book = get_cash_book(company_id, year, Channel::Cash.as_str(), pool).await?;
            let expenses = cash_book_expenses(company_id, year, pool).await?;
            cash_book_sheet(&book, &expenses)
        }
        ExportKind::Savings => {
            let ledger = get_monthly_ledger(company_id, year, through, pool).await?;
            let snapshot = get_app_snapshot(company_id, pool).await?;
            let names: HashMap<String, String> = snapshot
                .members
                .into_iter()
                .map(|member| (member.id, member.name))
                .collect();
            let company = company_name(company_id, pool).await?;
            savings_sheet(&ledger, year, &names, &company)
        }
        ExportKind::Loans => {
            let ledger = get_monthly_ledger(company_id, year, through, pool).await?;
            let snapshot = get_app_snapshot(company_id, pool).await?;
            let company = company_name(company_id, pool).await?;
            loans_sheet(&ledger, year, &snapshot.loans, &company)
        }
    };
    sheet.to_xlsx()
}

// --- Buku Kas ---------------------------------------------------------------

/// Expense columns of TAHUN 2026 (L–W): the Buku Kas categories after the cash transfers.
fn expense_columns() -> impl Iterator<Item = (u32, &'static str, &'static str)> {
    CASH_CATEGORIES
        .iter()
        .skip(3)
        .enumerate()
        .map(|(index, (kind, label))| (12 + index as u32, kind.as_str(), *label))
}

/// Amount of each expense category per transaction of the year.
async fn cash_book_expenses(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<HashMap<(String, String), i64>, String> {
    let rows = sqlx::query(
        "SELECT c.transaction_id, c.component_type, SUM(c.amount) amount FROM transaction_components c JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id WHERE c.company_id = ? AND t.business_date >= ? AND t.business_date < ? GROUP BY c.transaction_id, c.component_type",
    )
    .bind(company_id)
    .bind(format!("{year:04}-01-01"))
    .bind(format!("{:04}-01-01", year + 1))
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let kinds: Vec<&str> = expense_columns().map(|(_, kind, _)| kind).collect();
    let mut expenses = HashMap::new();
    for row in rows {
        let kind: String = row.try_get("component_type").map_err(|e| e.to_string())?;
        if kinds.contains(&kind.as_str()) {
            let id: String = row.try_get("transaction_id").map_err(|e| e.to_string())?;
            let amount: i64 = row.try_get("amount").map_err(|e| e.to_string())?;
            expenses.insert((id, kind), amount);
        }
    }
    Ok(expenses)
}

/// TAHUN 2026: Tanggal, Uraian, Bukti, Kode, Debet, Kredit, Jumlah Saldo Kas
/// (`=G3-E4+F4`), Keterangan, KAS BUKU TABUNGAN (Keluar, Masuk), Kas + Tabungan
/// and the expense columns. Deleted (reversed) rows are left out, as on the page.
fn cash_book_sheet(book: &CashBookDto, expenses: &HashMap<(String, String), i64>) -> Sheet {
    const DEBET: u32 = 5;
    const KREDIT: u32 = 6;
    const SALDO: u32 = 7;
    const KELUAR: u32 = 9;
    const MASUK: u32 = 10;
    const KAS_TABUNGAN: u32 = 11;
    let last_col = 11 + expense_columns().count() as u32;

    let mut sheet = Sheet::new(&format!("TAHUN {}", book.year));
    sheet.header((1, 1), (2, 1), "Tanggal");
    sheet.header((1, 2), (2, 2), "Uraian");
    sheet.header((1, 3), (2, 3), "Bukti");
    sheet.header((1, 4), (2, 4), "Kode");
    sheet.header((1, DEBET), (1, KREDIT), "Mutasi");
    sheet.header((2, DEBET), (2, DEBET), "Debet");
    sheet.header((2, KREDIT), (2, KREDIT), "Kredit");
    sheet.header((1, SALDO), (2, SALDO), "Jumlah Saldo Kas");
    sheet.header((1, 8), (2, 8), "Keterangan");
    sheet.header((1, KELUAR), (1, MASUK), "KAS BUKU TABUNGAN");
    sheet.header((2, KELUAR), (2, KELUAR), "Keluar");
    sheet.header((2, MASUK), (2, MASUK), "Masuk");
    sheet.header((1, KAS_TABUNGAN), (2, KAS_TABUNGAN), "Kas + Tabungan");
    for (col, _, label) in expense_columns() {
        sheet.header((1, col), (2, col), label);
        sheet.width(col, 14.0);
    }
    for (col, width) in [
        (1, 13.0),
        (2, 45.0),
        (3, 12.0),
        (4, 8.0),
        (5, 14.0),
        (6, 14.0),
        (7, 16.0),
        (8, 18.0),
        (9, 14.0),
        (10, 14.0),
        (11, 16.0),
    ] {
        sheet.width(col, width);
    }
    sheet.freeze(3, 1);

    let border_row = |sheet: &mut Sheet, row: u32, style: Style| {
        for col in 1..=last_col {
            let text = col <= 4 || col == 8;
            sheet.style(
                row,
                col,
                match (style, text) {
                    (Style::TotalNumber, true) => Style::TotalText,
                    (_, true) => Style::Text,
                    _ => style,
                },
            );
        }
    };

    border_row(&mut sheet, 3, Style::Number);
    sheet.text(
        3,
        2,
        &format!(
            "Saldo Kas Fisik Neraca Posisi 31 Desember {}",
            book.year - 1
        ),
        Style::TotalText,
    );
    sheet.number(3, SALDO, book.opening_balance as f64, Style::TotalNumber);
    if let Some(opening) = book.savings_book_opening_balance {
        sheet.number(3, MASUK, opening as f64, Style::TotalNumber);
        let total = Expr::cell(3, SALDO).add(Expr::cell(3, MASUK));
        sheet.formula(3, KAS_TABUNGAN, &total, Style::TotalNumber);
    }

    let mut row = 3;
    let mut last_date = "";
    for item in book.rows.iter().filter(|item| {
        item.transaction.status != TransactionStatus::Reversed && !item.transaction.is_reversal
    }) {
        row += 1;
        border_row(&mut sheet, row, Style::Number);
        if item.transaction.business_date != last_date {
            sheet.date(row, 1, &item.transaction.business_date);
            last_date = &item.transaction.business_date;
        }
        let transaction = &item.transaction;
        sheet.text(row, 2, &transaction.description, Style::Text);
        if transaction.channel == Channel::Cash {
            let col = if transaction.direction == TransactionDirection::Out {
                DEBET
            } else {
                KREDIT
            };
            sheet.number(row, col, transaction.amount as f64, Style::Number);
        }
        let balance = Expr::cell(row - 1, SALDO)
            .sub(Expr::cell(row, DEBET))
            .add(Expr::cell(row, KREDIT));
        sheet.checked(row, SALDO, &balance, item.balance, Style::Number);
        sheet.amount(row, KELUAR, item.savings_book_out, Style::Number);
        sheet.amount(row, MASUK, item.savings_book_in, Style::Number);
        for (col, kind, _) in expense_columns() {
            if let Some(amount) = expenses.get(&(transaction.id.clone(), kind.to_string())) {
                sheet.amount(row, col, *amount, Style::Number);
            }
        }
    }

    let total_row = row + 1;
    border_row(&mut sheet, total_row, Style::TotalNumber);
    sheet.text(total_row, 2, "JUMLAH", Style::TotalText);
    if row >= 4 {
        let mut sum_cols = vec![DEBET, KREDIT, KELUAR, MASUK];
        sum_cols.extend(expense_columns().map(|(col, _, _)| col));
        for col in sum_cols {
            let sum = Expr::Sum((4, col), (row, col));
            sheet.formula(total_row, col, &sum, Style::TotalNumber);
        }
    }
    let closing = Expr::cell(3, SALDO)
        .sub(Expr::cell(total_row, DEBET))
        .add(Expr::cell(total_row, KREDIT));
    sheet.checked(
        total_row,
        SALDO,
        &closing,
        book.closing_balance,
        Style::TotalNumber,
    );
    sheet
}

// --- Simpanan ---------------------------------------------------------------

/// Columns of one month on SIMPANAN: Tanggal, Debet/Kredit of Pokok, Wajib and
/// Manasuka, SHU (January only), then the month's Saldo Pokok, Wajib,
/// Manasuka and Total.
struct SavingsBlock {
    period: String,
    start: u32,
    shu: Option<u32>,
    balance: u32,
}

impl SavingsBlock {
    fn movement(&self, slot: u32, credit: bool) -> u32 {
        self.start + 1 + slot * 2 + u32::from(credit)
    }
    fn total(&self) -> u32 {
        self.balance + 3
    }
}

/// SIMPANAN: Saldo bulan Desember, then for every month the mutasi and
/// `Saldo = saldo bulan lalu + Kredit - Debet` (+ SHU in January), Total
/// Simpanan, and finally Ratas Simpanan, Insentif Simpanan and Pembulatan.
fn savings_sheet(
    ledger: &MonthlyLedgerDto,
    year: i32,
    names: &HashMap<String, String>,
    company: &str,
) -> Sheet {
    const HEAD: u32 = 5;
    const FIRST: u32 = 10;
    let periods = year_periods(ledger, year);
    let rows: HashMap<(&str, &str), &SavingsMonthDto> = ledger
        .savings
        .iter()
        .map(|row| ((row.member_id.as_str(), row.period.as_str()), row))
        .collect();
    let members: BTreeSet<&str> = ledger
        .savings
        .iter()
        .filter(|row| periods.contains(&row.period))
        .map(|row| row.member_id.as_str())
        .collect();

    let mut sheet = Sheet::new("SIMPANAN");
    let title = if company.is_empty() {
        "DATA SIMPANAN".to_string()
    } else {
        format!("DATA SIMPANAN {}", company.to_uppercase())
    };
    sheet.text(2, 2, &title, Style::Title);
    sheet.text(3, 2, &format!("TAHUN {year}"), Style::Title);

    // Layout.
    let mut blocks = Vec::new();
    let mut col = 7;
    for period in &periods {
        let shu = period.ends_with("-01").then_some(col + 7);
        let balance = col + 7 + u32::from(shu.is_some());
        blocks.push(SavingsBlock {
            period: period.clone(),
            start: col,
            shu,
            balance,
        });
        col = balance + 4;
    }
    let (ratas, insentif, pembulatan) = (col, col + 1, col + 2);
    let rate_cell = (HEAD + 3, insentif);

    // Headers.
    sheet.header((HEAD, 1), (HEAD + 4, 1), "No");
    sheet.header((HEAD, 2), (HEAD + 4, 2), "Nama Pensiunan");
    sheet.header((HEAD, 3), (HEAD, (col - 1).max(6)), "B U L A N");
    let previous = periods
        .first()
        .map(|period| shift_period(period, -1))
        .unwrap_or_else(|| format!("{:04}-12", year - 1));
    sheet.header(
        (HEAD + 1, 3),
        (HEAD + 1, 5),
        &format!("SALDO BULAN {}", month_label(&previous)),
    );
    sheet.header(
        (HEAD + 1, 6),
        (HEAD + 4, 6),
        &format!("TOTAL SIMPANAN {}", month_label(&previous)),
    );
    for (col, label) in [(3, "Pokok"), (4, "Wajib"), (5, "Manasuka")] {
        sheet.header((HEAD + 2, col), (HEAD + 4, col), label);
    }
    for block in &blocks {
        let label = month_label(&block.period);
        sheet.header(
            (HEAD + 1, block.start),
            (HEAD + 1, block.balance + 2),
            &format!("BULAN {label}"),
        );
        sheet.header(
            (HEAD + 1, block.total()),
            (HEAD + 4, block.total()),
            &format!("TOTAL SIMPANAN BULAN {label}"),
        );
        sheet.header(
            (HEAD + 2, block.start),
            (HEAD + 2, block.balance - 1),
            &format!("MUTASI BULAN {label}"),
        );
        sheet.header(
            (HEAD + 2, block.balance),
            (HEAD + 2, block.balance + 2),
            &format!("Saldo Simpanan Bulan {label}"),
        );
        sheet.header((HEAD + 3, block.start), (HEAD + 4, block.start), "Tanggal");
        for (slot, label) in ["Simpanan Pokok", "Simpanan Wajib", "Simpanan Manasuka"]
            .iter()
            .enumerate()
        {
            let debet = block.movement(slot as u32, false);
            sheet.header((HEAD + 3, debet), (HEAD + 3, debet + 1), label);
            sheet.header((HEAD + 4, debet), (HEAD + 4, debet), "Debet");
            sheet.header((HEAD + 4, debet + 1), (HEAD + 4, debet + 1), "Kredit");
        }
        if let Some(shu) = block.shu {
            sheet.header(
                (HEAD + 3, shu),
                (HEAD + 4, shu),
                &format!("SHU Tahun Buku {}", year - 1),
            );
        }
        for (offset, label) in ["Pokok", "Wajib", "Manasuka"].iter().enumerate() {
            let col = block.balance + offset as u32;
            sheet.header((HEAD + 3, col), (HEAD + 4, col), label);
        }
        sheet.width(block.start, 12.0);
    }
    let last_period = periods.last().cloned().unwrap_or_default();
    if !periods.is_empty() {
        sheet.header(
            (HEAD, ratas),
            (HEAD + 4, ratas),
            &format!("Ratas Simpanan s/d {}", month_label(&last_period)),
        );
        sheet.header(
            (HEAD, insentif),
            (HEAD + 2, pembulatan),
            "Insentif Simpanan",
        );
        sheet.header(rate_cell, (HEAD + 4, insentif), "");
        sheet.number(
            rate_cell.0,
            rate_cell.1,
            ledger.savings_incentive_rate,
            Style::HeaderPercent,
        );
        sheet.header((HEAD + 3, pembulatan), (HEAD + 4, pembulatan), "Pembulatan");
    }
    sheet.width(1, 5.0);
    sheet.width(2, 32.0);
    for col in 3..col + 3 {
        if !blocks.iter().any(|block| block.start == col) {
            sheet.width(col, 13.0);
        }
    }
    sheet.freeze(FIRST, 3);

    // Members.
    let numeric_cols: Vec<u32> = (3..=6)
        .chain(
            blocks
                .iter()
                .flat_map(|block| (block.start + 1..=block.total()).collect::<Vec<_>>()),
        )
        .chain(if periods.is_empty() {
            vec![]
        } else {
            vec![ratas, insentif, pembulatan]
        })
        .collect();
    let mut row = FIRST - 1;
    for (index, member_id) in members.iter().enumerate() {
        row += 1;
        for col in 1..col + 3 {
            sheet.style(row, col, Style::Number);
        }
        sheet.number(row, 1, (index + 1) as f64, Style::Integer);
        let name = names
            .get(*member_id)
            .map(String::as_str)
            .unwrap_or(member_id);
        sheet.text(row, 2, name, Style::Text);
        if let Some(first) = periods
            .first()
            .and_then(|period| rows.get(&(*member_id, period.as_str())))
        {
            sheet.number(row, 3, first.principal_opening as f64, Style::Number);
            sheet.number(row, 4, first.mandatory_opening as f64, Style::Number);
            sheet.number(row, 5, first.voluntary_opening as f64, Style::Number);
        }
        let opening_total = Expr::sum_of((3..=5).map(|col| Expr::cell(row, col)));
        sheet.formula(row, 6, &opening_total, Style::Number);

        let mut previous_balance = 3;
        for block in &blocks {
            if let Some(month) = rows.get(&(*member_id, block.period.as_str())) {
                if !month.transaction_date.is_empty() {
                    sheet.date(row, block.start, &month.transaction_date);
                }
                let moves = [
                    (
                        month.principal_out,
                        month.principal_in,
                        month.principal_closing,
                    ),
                    (
                        month.mandatory_out,
                        month.mandatory_in,
                        month.mandatory_closing,
                    ),
                    (
                        month.voluntary_out,
                        month.voluntary_in,
                        month.voluntary_closing,
                    ),
                ];
                for (slot, (debet, kredit, _)) in moves.iter().enumerate() {
                    let slot = slot as u32;
                    sheet.amount(row, block.movement(slot, false), *debet, Style::Number);
                    sheet.amount(row, block.movement(slot, true), *kredit, Style::Number);
                }
                if let Some(shu) = block.shu {
                    sheet.amount(row, shu, month.shu, Style::Number);
                }
                for (slot, (_, _, closing)) in moves.iter().enumerate() {
                    let slot32 = slot as u32;
                    let mut balance = Expr::cell(row, previous_balance + slot32)
                        .add(Expr::cell(row, block.movement(slot32, true)))
                        .sub(Expr::cell(row, block.movement(slot32, false)));
                    if let (Some(shu), 2) = (block.shu, slot) {
                        balance = balance.add(Expr::cell(row, shu));
                    }
                    sheet.checked(
                        row,
                        block.balance + slot32,
                        &balance,
                        *closing,
                        Style::Number,
                    );
                }
                let total = Expr::sum_of((0..3).map(|slot| Expr::cell(row, block.balance + slot)));
                sheet.formula(row, block.total(), &total, Style::Number);
            }
            previous_balance = block.balance;
        }

        // Ratas Simpanan: average Total Simpanan from January; months before
        // the member joined count as 0.
        if let Some(last) = rows.get(&(*member_id, last_period.as_str())) {
            let average = Expr::sum_of(blocks.iter().map(|block| Expr::cell(row, block.total())))
                .div(Expr::num(blocks.len() as f64));
            sheet.checked(row, ratas, &average, last.savings_average, Style::Number);
            let incentive = Expr::cell(row, ratas).mul(Expr::Fixed(rate_cell.0, rate_cell.1));
            sheet.checked(
                row,
                insentif,
                &incentive,
                last.savings_incentive_raw,
                Style::Number,
            );
            let rounded = Expr::cell(row, insentif).round_up(-3);
            sheet.checked(
                row,
                pembulatan,
                &rounded,
                last.savings_incentive,
                Style::Number,
            );
        }
    }

    let total_row = row + 2;
    for col in 1..col + 3 {
        sheet.style(total_row, col, Style::TotalNumber);
    }
    sheet.text(total_row, 1, "JUMLAH", Style::TotalText);
    sheet.style(total_row, 2, Style::TotalText);
    if row >= FIRST {
        for col in numeric_cols {
            if blocks.iter().any(|block| block.start == col) {
                continue;
            }
            let sum = Expr::Sum((FIRST, col), (row, col));
            sheet.formula(total_row, col, &sum, Style::TotalNumber);
        }
    }
    sheet
}

// --- Pinjaman ---------------------------------------------------------------

/// Columns of one month on PINJAMAN BULANAN: Kewajiban setor s/d bulan ini
/// (Pokok, Bunga, Jumlah), Mutasi (Tanggal, Jumlah, Pokok Debet, Pokok Kredit,
/// Bunga, Provisi), Saldo Posisi and Tunggakan s/d bulan ini (Pokok, Bunga, Jumlah).
struct LoanBlock {
    period: String,
    start: u32,
}

impl LoanBlock {
    const WIDTH: u32 = 13;
    fn due(&self) -> u32 {
        self.start
    }
    fn date(&self) -> u32 {
        self.start + 3
    }
    fn cash_in(&self) -> u32 {
        self.start + 4
    }
    fn debet(&self) -> u32 {
        self.start + 5
    }
    fn kredit(&self) -> u32 {
        self.start + 6
    }
    fn interest(&self) -> u32 {
        self.start + 7
    }
    fn provision(&self) -> u32 {
        self.start + 8
    }
    fn balance(&self) -> u32 {
        self.start + 9
    }
    fn arrears(&self) -> u32 {
        self.start + 10
    }
}

const PLAFOND: u32 = 3;
const RATE: u32 = 4;
const TENOR: u32 = 10;
const OPENING: u32 = 11;
const INSTALLMENT: u32 = 12;
const PRIOR_ARREARS: u32 = 15;

/// Scheduled installment of a month as a formula term: the loan's
/// `Kewajiban setor tiap bulan` cell when it matches, otherwise the amount
/// (e.g. the last, smaller installment) or nothing when none falls due.
fn installment_term(sheet: &Sheet, row: u32, col: u32, amount: i64) -> Option<Expr> {
    if amount == 0 {
        None
    } else if (sheet.value(row, col) - amount as f64).abs() < 0.5 {
        Some(Expr::cell(row, col))
    } else {
        Some(Expr::num(amount as f64))
    }
}

/// Interest falling due in a month: `ROUNDUP(saldo bulan lalu*D/12,0)` for
/// Menurun, the `Kewajiban setor tiap bulan` Bunga cell for Plat.
fn interest_term(
    sheet: &Sheet,
    row: u32,
    interest_type: InterestType,
    previous_balance: u32,
    amount: i64,
) -> Option<Expr> {
    if amount == 0 {
        return None;
    }
    if interest_type == InterestType::Declining {
        let declining = Expr::cell(row, previous_balance)
            .mul(Expr::cell(row, RATE))
            .div(Expr::num(12.0))
            .round_up(0);
        if (sheet.eval(&declining) - amount as f64).abs() < 0.5 {
            return Some(declining);
        }
    }
    installment_term(sheet, row, INSTALLMENT + 1, amount)
}

fn plus(base: Expr, term: Option<Expr>) -> Expr {
    match term {
        Some(term) => base.add(term),
        None => base,
    }
}

/// PINJAMAN BULANAN: both sections (Anggota and Non Anggota PP BRI) with their
/// SUB JUMLAH, the months from January, then Kewajiban s/d bulan depan,
/// Rekomendasi, Jumlah Pemasukan per triwulan and Insentif Pinjaman.
fn loans_sheet(ledger: &MonthlyLedgerDto, year: i32, loans: &[LoanDto], company: &str) -> Sheet {
    const HEAD: u32 = 3;
    let periods = year_periods(ledger, year);
    let months: HashMap<(&str, &str), &LoanMonthDto> = ledger
        .loans
        .iter()
        .filter(|row| row.shown)
        .map(|row| ((row.loan_id.as_str(), row.period.as_str()), row))
        .collect();
    let shown: BTreeSet<&str> = ledger
        .loans
        .iter()
        .filter(|row| row.shown && periods.contains(&row.period))
        .map(|row| row.loan_id.as_str())
        .collect();
    let loan_by_id: HashMap<&str, &LoanDto> =
        loans.iter().map(|loan| (loan.id.as_str(), loan)).collect();

    let mut sheet = Sheet::new("PINJAMAN BULANAN");
    let title = if company.is_empty() {
        format!("DATA PINJAMAN TAHUN {year}")
    } else {
        format!("DATA PINJAMAN {} TAHUN {year}", company.to_uppercase())
    };
    sheet.text(1, 1, &title, Style::Title);

    // Layout.
    let blocks: Vec<LoanBlock> = periods
        .iter()
        .enumerate()
        .map(|(index, period)| LoanBlock {
            period: period.clone(),
            start: 18 + LoanBlock::WIDTH * index as u32,
        })
        .collect();
    let tail = 18 + LoanBlock::WIDTH * blocks.len() as u32;
    let recommender = tail + 3;
    let income = move |quarter: u32| tail + 4 + quarter * 3;
    let incentive = tail + 16;
    let rate_cell = (HEAD + 3, incentive);
    let last_col = if periods.is_empty() {
        17
    } else {
        incentive + 1
    };

    // Headers.
    let first_period = periods
        .first()
        .cloned()
        .unwrap_or_else(|| format!("{year:04}-01"));
    let previous = month_label(&shift_period(&first_period, -1));
    for (col, label) in [
        (1, "No"),
        (2, "Nama"),
        (PLAFOND, "Plafond"),
        (6, "PG/TN"),
        (7, "Jenis Pinjaman"),
        (8, "Tanggal Realisasi"),
        (9, "Tanggal Jatuh Tempo"),
        (TENOR, "Jangka Waktu"),
    ] {
        sheet.header((HEAD, col), (HEAD + 3, col), label);
    }
    sheet.header((HEAD, RATE), (HEAD + 3, RATE + 1), "Bunga per-bulan");
    sheet.header(
        (HEAD, OPENING),
        (HEAD + 3, OPENING),
        &format!("Saldo Posisi {previous}"),
    );
    let three = |sheet: &mut Sheet, col: u32, label: &str, labels: [&str; 3]| {
        sheet.header((HEAD, col), (HEAD + 1, col + 2), label);
        for (offset, label) in labels.iter().enumerate() {
            let col = col + offset as u32;
            sheet.header((HEAD + 2, col), (HEAD + 3, col), label);
        }
    };
    const PBJ: [&str; 3] = ["Pokok", "Bunga", "Jumlah"];
    three(&mut sheet, INSTALLMENT, "Kewajiban setor tiap bulan", PBJ);
    three(
        &mut sheet,
        PRIOR_ARREARS,
        &format!("Tunggakan s/d {previous}"),
        PBJ,
    );
    for block in &blocks {
        let label = month_label(&block.period);
        three(
            &mut sheet,
            block.due(),
            &format!("Kewajiban setor s/d {label}"),
            PBJ,
        );
        sheet.header(
            (HEAD, block.date()),
            (HEAD, block.provision()),
            &format!("BULAN {label}"),
        );
        sheet.header(
            (HEAD + 1, block.date()),
            (HEAD + 1, block.provision()),
            &format!("Mutasi bulan {label}"),
        );
        for (col, label) in [
            (block.date(), "Tanggal"),
            (block.cash_in(), "Jumlah"),
            (block.interest(), "Bunga"),
            (block.provision(), "Provisi"),
        ] {
            sheet.header((HEAD + 2, col), (HEAD + 3, col), label);
        }
        sheet.header(
            (HEAD + 2, block.debet()),
            (HEAD + 2, block.kredit()),
            "Pokok",
        );
        sheet.header(
            (HEAD + 3, block.debet()),
            (HEAD + 3, block.debet()),
            "Debet",
        );
        sheet.header(
            (HEAD + 3, block.kredit()),
            (HEAD + 3, block.kredit()),
            "Kredit",
        );
        sheet.header(
            (HEAD, block.balance()),
            (HEAD + 3, block.balance()),
            &format!("Saldo Posisi {label}"),
        );
        three(
            &mut sheet,
            block.arrears(),
            &format!("Tunggakan s/d {label}"),
            PBJ,
        );
        sheet.width(block.date(), 12.0);
    }
    let last_period = periods.last().cloned().unwrap_or_default();
    if !periods.is_empty() {
        three(
            &mut sheet,
            tail,
            &format!(
                "Kewajiban s/d {}",
                month_label(&shift_period(&last_period, 1))
            ),
            PBJ,
        );
        sheet.header((HEAD, recommender), (HEAD + 3, recommender), "Rekomendasi");
        for (quarter, label) in ["JAN s/d MAR", "APR s/d JUN", "JUL s/d SEP", "OKT s/d DES"]
            .iter()
            .enumerate()
        {
            three(
                &mut sheet,
                income(quarter as u32),
                &format!("JUMLAH PEMASUKAN {label} {}", year % 100),
                ["BUNGA", "PROVISI", "JUMLAH"],
            );
        }
        sheet.header(
            (HEAD, incentive),
            (HEAD + 2, incentive + 1),
            "Insentif Pinjaman",
        );
        sheet.number(
            rate_cell.0,
            rate_cell.1,
            ledger.loan_incentive_rate,
            Style::HeaderPercent,
        );
        sheet.header(
            (HEAD + 3, incentive + 1),
            (HEAD + 3, incentive + 1),
            "Pembulatan",
        );
        sheet.width(recommender, 16.0);
    }
    sheet.width(1, 5.0);
    sheet.width(2, 32.0);
    for col in 3..=last_col {
        if col != recommender && !blocks.iter().any(|block| block.date() == col) {
            sheet.width(col, 13.0);
        }
    }
    sheet.width(RATE + 1, 9.0);
    sheet.width(6, 7.0);
    sheet.width(TENOR, 8.0);
    sheet.freeze(HEAD + 4, 3);

    // Numeric columns summed in SUB JUMLAH and JUMLAH TOTAL.
    let mut numeric: Vec<u32> = vec![PLAFOND];
    numeric.extend(OPENING..=PRIOR_ARREARS + 2);
    for block in &blocks {
        numeric.extend(
            (block.start..block.start + LoanBlock::WIDTH).filter(|col| *col != block.date()),
        );
    }
    if !periods.is_empty() {
        numeric.extend(tail..tail + 3);
        numeric.extend(tail + 4..=incentive + 1);
    }

    let mut row = HEAD + 3;
    let mut subtotals = Vec::new();
    for (group, title) in [
        (LoanGroup::Member, "PINJAMAN ANGGOTA PP BRI"),
        (LoanGroup::NonMember, "PINJAMAN NON ANGGOTA PP BRI"),
    ] {
        let section: Vec<&LoanDto> = shown
            .iter()
            .filter_map(|id| loan_by_id.get(id).copied())
            .filter(|loan| loan.loan_group == group)
            .collect();
        row += 1;
        sheet.text(row, 2, title, Style::TotalText);
        let first = row + 1;
        for (index, loan) in section.iter().enumerate() {
            row += 1;
            loan_row(&mut sheet, row, index + 1, loan, &blocks, &months, last_col);
            if let Some(last) = months.get(&(loan.id.as_str(), last_period.as_str())) {
                loan_tail(
                    &mut sheet, row, loan, last, &blocks, tail, income, incentive, rate_cell,
                );
            }
            if !periods.is_empty() {
                sheet.text(row, recommender, &loan.recommender, Style::Text);
            }
        }
        row += 1;
        for col in 1..=last_col {
            sheet.style(row, col, Style::TotalNumber);
        }
        sheet.text(row, 1, "SUB JUMLAH", Style::TotalText);
        sheet.merge((row, 1), (row, 2));
        sheet.text(
            row,
            8,
            &format!("{} ORANG", section.len()),
            Style::TotalText,
        );
        if !section.is_empty() {
            for &col in &numeric {
                let sum = Expr::Sum((first, col), (row - 1, col));
                sheet.formula(row, col, &sum, Style::TotalNumber);
            }
        }
        subtotals.push(row);
        row += 1;
    }
    row += 1;
    for col in 1..=last_col {
        sheet.style(row, col, Style::TotalNumber);
    }
    sheet.text(row, 1, "JUMLAH TOTAL", Style::TotalText);
    sheet.merge((row, 1), (row, 2));
    for &col in &numeric {
        let total = Expr::sum_of(subtotals.iter().map(|&subtotal| Expr::cell(subtotal, col)));
        sheet.formula(row, col, &total, Style::TotalNumber);
    }
    sheet
}

/// A loan's columns from No through the months.
#[allow(clippy::too_many_arguments)]
fn loan_row(
    sheet: &mut Sheet,
    row: u32,
    no: usize,
    loan: &LoanDto,
    blocks: &[LoanBlock],
    months: &HashMap<(&str, &str), &LoanMonthDto>,
    last_col: u32,
) {
    for col in 1..=last_col {
        sheet.style(row, col, Style::Number);
    }
    sheet.number(row, 1, no as f64, Style::Integer);
    sheet.text(row, 2, &loan.member_name, Style::Text);
    sheet.number(row, PLAFOND, loan.plafond as f64, Style::Number);
    sheet.number(row, RATE, loan.rate / 100.0, Style::Percent);
    sheet.text(
        row,
        RATE + 1,
        &loan.interest_type.as_str().to_lowercase(),
        Style::Text,
    );
    sheet.text(row, 6, &loan.guarantee, Style::Text);
    sheet.text(
        row,
        7,
        loan.loan_type.map(|kind| kind.as_str()).unwrap_or_default(),
        Style::Text,
    );
    sheet.date(row, 8, &iso_date(&loan.realization_date));
    sheet.date(row, 9, &iso_date(&loan.due_date));
    sheet.number(row, TENOR, loan.tenor as f64, Style::Integer);

    // Kewajiban setor tiap bulan, as in the sheet (L, M, N).
    if loan.tenor > 0 {
        let principal = Expr::cell(row, PLAFOND)
            .div(Expr::cell(row, TENOR))
            .round_up(-3);
        sheet.formula(row, INSTALLMENT, &principal, Style::Number);
    }
    let interest = Expr::cell(row, PLAFOND)
        .mul(Expr::cell(row, RATE))
        .div(Expr::num(12.0))
        .round_up(0);
    sheet.formula(row, INSTALLMENT + 1, &interest, Style::Number);
    let installment = Expr::cell(row, INSTALLMENT).add(Expr::cell(row, INSTALLMENT + 1));
    sheet.formula(row, INSTALLMENT + 2, &installment, Style::Number);

    // Saldo and Tunggakan before the first month.
    if let Some(first) = blocks
        .first()
        .and_then(|block| months.get(&(loan.id.as_str(), block.period.as_str())))
    {
        sheet.amount(row, OPENING, first.opening_balance, Style::Number);
        let (principal, interest) = prior_arrears(first);
        sheet.amount(row, PRIOR_ARREARS, principal, Style::Number);
        sheet.amount(row, PRIOR_ARREARS + 1, interest, Style::Number);
    }
    let total = Expr::cell(row, PRIOR_ARREARS).add(Expr::cell(row, PRIOR_ARREARS + 1));
    sheet.formula(row, PRIOR_ARREARS + 2, &total, Style::Number);

    let (mut previous_balance, mut previous_arrears) = (OPENING, PRIOR_ARREARS);
    for block in blocks {
        if let Some(month) = months.get(&(loan.id.as_str(), block.period.as_str())) {
            loan_month(
                sheet,
                row,
                loan,
                block,
                month,
                previous_balance,
                previous_arrears,
            );
        }
        previous_balance = block.balance();
        previous_arrears = block.arrears();
    }
}

/// Tunggakan s/d bulan lalu, as the page derives it.
fn prior_arrears(month: &LoanMonthDto) -> (i64, i64) {
    (
        month.arrears_principal - month.scheduled_principal + month.principal_paid,
        month.arrears_interest - month.scheduled_interest + month.interest_paid,
    )
}

fn loan_month(
    sheet: &mut Sheet,
    row: u32,
    loan: &LoanDto,
    block: &LoanBlock,
    month: &LoanMonthDto,
    previous_balance: u32,
    previous_arrears: u32,
) {
    let (prior_principal, prior_interest) = prior_arrears(month);
    // Kewajiban setor s/d bulan ini = Tunggakan s/d bulan lalu + kewajiban bulan ini.
    let due_principal = plus(
        Expr::cell(row, previous_arrears),
        installment_term(sheet, row, INSTALLMENT, month.scheduled_principal),
    );
    sheet.checked(
        row,
        block.due(),
        &due_principal,
        prior_principal + month.scheduled_principal,
        Style::Number,
    );
    let due_interest = plus(
        Expr::cell(row, previous_arrears + 1),
        interest_term(
            sheet,
            row,
            loan.interest_type,
            previous_balance,
            month.scheduled_interest,
        ),
    );
    sheet.checked(
        row,
        block.due() + 1,
        &due_interest,
        prior_interest + month.scheduled_interest,
        Style::Number,
    );
    let due_total = Expr::cell(row, block.due()).add(Expr::cell(row, block.due() + 1));
    sheet.formula(row, block.due() + 2, &due_total, Style::Number);

    // Mutasi.
    if !month.transaction_date.is_empty() {
        sheet.date(row, block.date(), &month.transaction_date);
    }
    sheet.amount(row, block.debet(), month.disbursed, Style::Number);
    sheet.amount(row, block.kredit(), month.principal_paid, Style::Number);
    sheet.amount(row, block.interest(), month.interest_paid, Style::Number);
    sheet.amount(row, block.provision(), month.provision, Style::Number);
    if month.principal_paid + month.interest_paid + month.provision != 0 {
        let cash_in = Expr::cell(row, block.kredit())
            .add(Expr::cell(row, block.interest()))
            .add(Expr::cell(row, block.provision()));
        sheet.formula(row, block.cash_in(), &cash_in, Style::Number);
    }

    // Saldo Posisi = saldo bulan lalu + Pokok Debet - Pokok Kredit.
    let balance = Expr::cell(row, previous_balance)
        .add(Expr::cell(row, block.debet()))
        .sub(Expr::cell(row, block.kredit()));
    sheet.checked(
        row,
        block.balance(),
        &balance,
        month.closing_balance,
        Style::Number,
    );

    // Tunggakan = Kewajiban setor s/d bulan ini - yang dibayar.
    let arrears_principal = Expr::cell(row, block.due()).sub(Expr::cell(row, block.kredit()));
    sheet.checked(
        row,
        block.arrears(),
        &arrears_principal,
        month.arrears_principal,
        Style::Number,
    );
    let arrears_interest = Expr::cell(row, block.due() + 1).sub(Expr::cell(row, block.interest()));
    sheet.checked(
        row,
        block.arrears() + 1,
        &arrears_interest,
        month.arrears_interest,
        Style::Number,
    );
    let arrears_total = Expr::cell(row, block.arrears()).add(Expr::cell(row, block.arrears() + 1));
    sheet.formula(row, block.arrears() + 2, &arrears_total, Style::Number);
}

/// Kewajiban s/d bulan depan, Jumlah Pemasukan per triwulan and Insentif
/// Pinjaman, as of the last month.
#[allow(clippy::too_many_arguments)]
fn loan_tail(
    sheet: &mut Sheet,
    row: u32,
    loan: &LoanDto,
    last: &LoanMonthDto,
    blocks: &[LoanBlock],
    tail: u32,
    income: impl Fn(u32) -> u32,
    incentive: u32,
    rate_cell: (u32, u32),
) {
    let Some(block) = blocks.last() else {
        return;
    };
    let next_principal = last.next_due_principal - last.arrears_principal;
    let next_interest = last.next_due_interest - last.arrears_interest;
    let due_principal = plus(
        Expr::cell(row, block.arrears()),
        installment_term(sheet, row, INSTALLMENT, next_principal),
    );
    sheet.checked(
        row,
        tail,
        &due_principal,
        last.next_due_principal,
        Style::Number,
    );
    let due_interest = plus(
        Expr::cell(row, block.arrears() + 1),
        interest_term(
            sheet,
            row,
            loan.interest_type,
            block.balance(),
            next_interest,
        ),
    );
    sheet.checked(
        row,
        tail + 1,
        &due_interest,
        last.next_due_interest,
        Style::Number,
    );
    let due_total = Expr::cell(row, tail).add(Expr::cell(row, tail + 1));
    sheet.formula(row, tail + 2, &due_total, Style::Number);

    // Jumlah Pemasukan per triwulan: Bunga and Provisi of the quarter's months.
    for quarter in 0..4u32 {
        let col = income(quarter);
        let in_quarter: Vec<&LoanBlock> = blocks
            .iter()
            .filter(|block| quarter_of(&block.period) == quarter as usize)
            .collect();
        // Bunga, then Provisi (the column after Bunga in each month).
        let amounts = [
            last.income_interest[quarter as usize],
            last.income_provision[quarter as usize],
        ];
        for (offset, amount) in (0..).zip(amounts) {
            if in_quarter.is_empty() {
                sheet.amount(row, col + offset, amount, Style::Number);
            } else {
                let sum = Expr::sum_of(
                    in_quarter
                        .iter()
                        .map(|block| Expr::cell(row, block.interest() + offset)),
                );
                sheet.checked(row, col + offset, &sum, amount, Style::Number);
            }
        }
        let total = Expr::cell(row, col).add(Expr::cell(row, col + 1));
        sheet.formula(row, col + 2, &total, Style::Number);
    }
    let share = Expr::sum_of((0..4).map(|quarter| Expr::cell(row, income(quarter) + 2)))
        .mul(Expr::Fixed(rate_cell.0, rate_cell.1));
    sheet.checked(row, incentive, &share, last.incentive_raw, Style::Number);
    let rounded = Expr::cell(row, incentive).round_up(-3);
    sheet.checked(row, incentive + 1, &rounded, last.incentive, Style::Number);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_months_like_the_workbooks() {
        assert_eq!(month_label("2026-01"), "JANUARI 2026");
        assert_eq!(shift_period("2026-01", -1), "2025-12");
        assert_eq!(shift_period("2026-12", 1), "2027-01");
        assert_eq!(iso_date("28-Jan-2026"), "2026-01-28");
        assert_eq!(iso_date("2026-01-28"), "2026-01-28");
    }

    /// Exports the three pages from a copy of a real database:
    /// `KOPERASI_EXPORT_DB=/path/koperasi.db KOPERASI_EXPORT_OUT=/dir cargo test export_from_database -- --ignored`
    #[test]
    #[ignore]
    fn export_from_database() {
        let db = std::env::var("KOPERASI_EXPORT_DB").expect("KOPERASI_EXPORT_DB");
        let out = std::path::PathBuf::from(
            std::env::var("KOPERASI_EXPORT_OUT").expect("KOPERASI_EXPORT_OUT"),
        );
        tauri::async_runtime::block_on(async {
            let pool = sqlx::SqlitePool::connect(&format!("sqlite://{db}"))
                .await
                .unwrap();
            for kind in [ExportKind::CashBook, ExportKind::Savings, ExportKind::Loans] {
                let bytes = export_workbook(&kind, "default", 2026, Some("2026-10"), &pool)
                    .await
                    .unwrap();
                std::fs::write(out.join(kind.file_name(2026)), bytes).unwrap();
            }
        });
    }
}
