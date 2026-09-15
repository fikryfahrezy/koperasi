//! Pure cooperative business rules with no Tauri or database dependency.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) struct LoanCalculation {
    pub(crate) principal_installment: i64,
    pub(crate) first_interest: i64,
    pub(crate) provision: i64,
    pub(crate) first_total: i64,
    pub(crate) annual_rate: f64,
}

pub(crate) fn timestamp_id(prefix: &str) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis}-{sequence}")
}

pub(crate) fn money(value: &str) -> i64 {
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

pub(crate) fn percentage(value: &str) -> f64 {
    value
        .trim()
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or(0.0)
}

pub(crate) fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

pub(crate) fn add_months(value: &str, months: i64) -> Result<String, String> {
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

pub(crate) fn calculate_loan(
    plafond: i64,
    tenor: i64,
    annual_rate: f64,
    provision_rate: f64,
) -> LoanCalculation {
    let principal_installment = ((plafond as f64 / tenor as f64) / 1000.0).ceil() as i64 * 1000;
    let first_interest = (plafond as f64 * annual_rate / 12.0).round() as i64;
    let provision = (plafond as f64 * provision_rate).round() as i64;
    LoanCalculation {
        principal_installment,
        first_interest,
        provision,
        first_total: principal_installment + first_interest,
        annual_rate: annual_rate * 100.0,
    }
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
        let preview = calculate_loan(10_000_000, 24, 0.24, 0.01);
        assert_eq!(preview.principal_installment, 417_000);
        assert_eq!(preview.first_interest, 200_000);
        assert_eq!(preview.provision, 100_000);
        assert_eq!(preview.first_total, 617_000);
        assert_eq!(preview.annual_rate, 24.0);
    }
}
