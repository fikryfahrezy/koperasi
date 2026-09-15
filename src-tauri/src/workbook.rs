//! Excel workbook loading and cell conversion for the legacy import path.

use std::path::Path;

use calamine::{open_workbook_auto, Data, DataType, Reader};

use crate::domain::{money, percentage};

pub(crate) struct WorkbookSheets {
    pub(crate) master_savings: Vec<Vec<Data>>,
    pub(crate) savings_2026: Vec<Vec<Data>>,
    pub(crate) master_loans: Vec<Vec<Data>>,
    pub(crate) loans_2026: Vec<Vec<Data>>,
    pub(crate) cash_2026: Vec<Vec<Data>>,
}

pub(crate) fn load_workbook_sheets(path: &Path) -> Result<WorkbookSheets, String> {
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

pub(crate) fn cell_text(cell: &Data) -> String {
    match cell {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) if value.fract() == 0.0 => format!("{}", *value as i64),
        Data::Float(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn cell_money(cell: &Data) -> i64 {
    match cell {
        Data::Float(value) => value.round() as i64,
        Data::Int(value) => *value,
        Data::String(value) => money(value),
        _ => 0,
    }
}

pub(crate) fn cell_percentage(cell: &Data) -> f64 {
    match cell {
        Data::Float(value) => value * 100.0,
        Data::Int(value) => *value as f64 * 100.0,
        Data::String(value) => percentage(value),
        _ => 0.0,
    }
}

pub(crate) fn cell_int(cell: &Data) -> i64 {
    match cell {
        Data::Float(value) => value.round() as i64,
        Data::Int(value) => *value,
        Data::String(value) => value.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

pub(crate) fn cell_date_iso(cell: &Data) -> String {
    cell.as_datetime()
        .map(|value| value.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "2026-01-01".to_string())
}

pub(crate) fn cell_date_display(cell: &Data) -> String {
    cell.as_datetime()
        .map(|value| value.format("%d-%b-%Y").to_string())
        .unwrap_or_default()
}
