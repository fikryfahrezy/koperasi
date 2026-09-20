//! Pure cooperative business rules with no Tauri or database dependency.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum $name {
            $($variant),+
        }

        impl $name {
            pub(crate) const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = String;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!("Nilai {} tidak valid: {value}", stringify!($name))),
                }
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }
    };
}

string_enum!(MemberStatus {
    Active => "Aktif",
    Inactive => "Nonaktif",
});

string_enum!(SavingsAccountType {
    Principal => "POKOK",
    Mandatory => "WAJIB",
    Voluntary => "MANASUKA",
});

string_enum!(SavingsAccountStatus {
    Active => "ACTIVE",
});

string_enum!(SavingsMovement {
    Deposit => "Setoran",
    Withdrawal => "Penarikan",
});

string_enum!(LoanStatus {
    Draft => "Draf",
    Active => "Berjalan",
    NeedsReview => "Perlu review",
    PaidOff => "Lunas",
});

string_enum!(InterestType {
    Declining => "Menurun",
    Flat => "Flat",
});

string_enum!(TransactionDirection {
    In => "Masuk",
    Out => "Keluar",
});

impl TransactionDirection {
    pub(crate) const fn reversed(self) -> Self {
        match self {
            Self::In => Self::Out,
            Self::Out => Self::In,
        }
    }
}

string_enum!(TransactionStatus {
    Posted => "Terposting",
    Draft => "Draf",
    Reversed => "Dibalik",
});

string_enum!(PeriodStatus {
    Open => "OPEN",
    Locked => "LOCKED",
});

pub(crate) struct LoanCalculation {
    pub(crate) principal_installment: i64,
    pub(crate) first_interest: i64,
    pub(crate) provision: i64,
    pub(crate) first_total: i64,
    pub(crate) annual_rate: f64,
}

pub(crate) fn calculate_rate_amount(amount: i64, rate: f64) -> Result<i64, String> {
    let value = amount as f64 * rate;
    if amount < 0 || rate < 0.0 || !value.is_finite() || value >= i64::MAX as f64 {
        return Err("Hasil perhitungan pinjaman terlalu besar.".into());
    }
    Ok(value.round() as i64)
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

pub(crate) fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

fn date_parts(value: &str) -> Result<(i64, i64, i64), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return Err("Tanggal bisnis tidak valid.".into());
    }
    let year = value[0..4]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    let month = value[5..7]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    let day = value[8..10]
        .parse::<i64>()
        .map_err(|_| "Tanggal bisnis tidak valid.")?;
    if year == 0 || !(1..=12).contains(&month) {
        return Err("Tanggal bisnis tidak valid.".into());
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&day) {
        return Err("Tanggal bisnis tidak valid.".into());
    }
    Ok((year, month, day))
}

pub(crate) fn business_period(value: &str) -> Result<&str, String> {
    date_parts(value)?;
    Ok(&value[..7])
}

pub(crate) fn add_months(value: &str, months: i64) -> Result<String, String> {
    let (year, month, day) = date_parts(value)?;
    let absolute_month = year
        .checked_mul(12)
        .and_then(|value| value.checked_add(month - 1))
        .and_then(|value| value.checked_add(months))
        .ok_or("Tanggal jatuh tempo di luar rentang yang didukung.")?;
    let due_year = absolute_month.div_euclid(12);
    let due_month = absolute_month.rem_euclid(12) + 1;
    if !(1..=9999).contains(&due_year) {
        return Err("Tanggal jatuh tempo di luar rentang yang didukung.".into());
    }
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
) -> Result<LoanCalculation, String> {
    if plafond <= 0
        || tenor <= 0
        || !annual_rate.is_finite()
        || !provision_rate.is_finite()
        || annual_rate < 0.0
        || provision_rate < 0.0
    {
        return Err("Parameter perhitungan pinjaman tidak valid.".into());
    }
    let principal_installment = ((plafond as f64 / tenor as f64) / 1000.0).ceil() as i64;
    let principal_installment = principal_installment
        .checked_mul(1000)
        .ok_or("Hasil perhitungan pinjaman terlalu besar.")?;
    let first_interest = calculate_rate_amount(plafond, annual_rate / 12.0)?;
    let provision = calculate_rate_amount(plafond, provision_rate)?;
    let first_total = principal_installment
        .checked_add(first_interest)
        .ok_or("Hasil perhitungan pinjaman terlalu besar.")?;
    let annual_rate_percent = annual_rate * 100.0;
    if !annual_rate_percent.is_finite() {
        return Err("Hasil perhitungan pinjaman terlalu besar.".into());
    }
    Ok(LoanCalculation {
        principal_installment,
        first_interest,
        provision,
        first_total,
        annual_rate: annual_rate_percent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_and_advances_business_dates() {
        assert_eq!(add_months("2026-01-31", 1).unwrap(), "2026-02-28");
        assert_eq!(add_months("2024-01-31", 1).unwrap(), "2024-02-29");
        assert!(business_period("2026-02-29").is_err());
        assert!(business_period("2026-9-13").is_err());
        assert!(business_period("not-a-date").is_err());
    }

    #[test]
    fn calculates_parameterized_loan_preview() {
        let preview = calculate_loan(10_000_000, 24, 0.24, 0.01).unwrap();
        assert_eq!(preview.principal_installment, 417_000);
        assert_eq!(preview.first_interest, 200_000);
        assert_eq!(preview.provision, 100_000);
        assert_eq!(preview.first_total, 617_000);
        assert_eq!(preview.annual_rate, 24.0);
    }
}
