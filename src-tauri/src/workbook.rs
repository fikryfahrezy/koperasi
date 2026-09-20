//! Excel workbook loading and cell conversion for the legacy import path.

use std::path::Path;

use calamine::{open_workbook_auto, Data, DataType, Reader};

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

fn rounded_i64(value: f64) -> Result<i64, String> {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < i64::MIN as f64 || rounded >= i64::MAX as f64 {
        return Err("Nilai angka di workbook berada di luar rentang yang didukung.".into());
    }
    Ok(rounded as i64)
}

pub(crate) fn cell_money(cell: &Data) -> Result<i64, String> {
    match cell {
        Data::Float(value) => rounded_i64(*value),
        Data::Int(value) => Ok(*value),
        Data::String(value) => {
            let value = value.trim();
            if value.is_empty() || value == "-" || value == "`" {
                return Ok(0);
            }
            let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
            if digits.is_empty() {
                return Err(format!("Nilai uang tidak valid: {value}"));
            }
            let parsed = digits
                .parse::<i64>()
                .map_err(|_| format!("Nilai uang terlalu besar: {value}"))?;
            if value.contains('(') || value.starts_with('-') {
                parsed
                    .checked_neg()
                    .ok_or_else(|| format!("Nilai uang terlalu besar: {value}"))
            } else {
                Ok(parsed)
            }
        }
        Data::Empty => Ok(0),
        _ => Err("Jenis sel uang tidak didukung.".into()),
    }
}

pub(crate) fn cell_percentage(cell: &Data) -> Result<f64, String> {
    let value = match cell {
        Data::Float(value) => value * 100.0,
        Data::Int(value) => *value as f64 * 100.0,
        Data::String(value) => value
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .map_err(|_| format!("Persentase tidak valid: {value}"))?,
        Data::Empty => 0.0,
        _ => return Err("Jenis sel persentase tidak didukung.".into()),
    };
    if !value.is_finite() {
        return Err("Persentase harus berupa angka terbatas.".into());
    }
    Ok(value)
}

pub(crate) fn cell_int(cell: &Data) -> Result<i64, String> {
    match cell {
        Data::Float(value) => rounded_i64(*value),
        Data::Int(value) => Ok(*value),
        Data::String(value) => value
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("Bilangan bulat tidak valid: {value}")),
        Data::Empty => Ok(0),
        _ => Err("Jenis sel bilangan bulat tidak didukung.".into()),
    }
}

pub(crate) fn cell_date_iso(cell: &Data) -> Result<String, String> {
    cell.as_datetime()
        .map(|value| value.format("%Y-%m-%d").to_string())
        .ok_or_else(|| "Tanggal workbook tidak valid.".to_string())
}

pub(crate) fn cell_date_display(cell: &Data) -> Result<String, String> {
    cell.as_datetime()
        .map(|value| value.format("%d-%b-%Y").to_string())
        .ok_or_else(|| "Tanggal workbook tidak valid.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_financial_cells_instead_of_defaulting_to_zero() {
        assert_eq!(
            cell_money(&Data::String("(12,950,000)".into())).unwrap(),
            -12_950_000
        );
        assert_eq!(cell_money(&Data::String("-1,250".into())).unwrap(), -1_250);
        assert!(cell_money(&Data::String("not money".into())).is_err());
        assert!(cell_percentage(&Data::String("not a rate".into())).is_err());
        assert!(cell_date_iso(&Data::String("not a date".into())).is_err());
    }
}
