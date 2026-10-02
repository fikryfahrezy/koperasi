//! Buku besar bulanan simpanan dan pinjaman, disusun seperti workbook Excel.
//!
//! Tidak ada tabel khusus: setiap bulan diturunkan dengan menjumlahkan komponen
//! transaksi yang berlaku (view `effective_components`) per tanggal. Riwayat 2026 hasil migrasi juga
//! tetap dibaca dari transaksi yang sudah tersimpan.
//! Kewajiban setor dan tunggakan pinjaman dihitung dari syarat pinjaman.

use std::collections::{BTreeSet, HashMap};

use sqlx::{Row, SqlitePool};

use crate::contracts::{LoanMonthDto, MonthlyLedgerDto, SavingsMonthDto};
use crate::domain::{ComponentType, InterestType, SavingsAccountType};

macro_rules! col {
    ($row:expr, $column:literal) => {
        $row.try_get($column).map_err(|error| error.to_string())?
    };
}

#[derive(Clone, Default)]
struct SavingsMonth {
    transaction_date: String,
    opening: [i64; 3],
    incoming: [i64; 3],
    outgoing: [i64; 3],
    shu: i64,
}

#[derive(Clone, Default)]
struct LoanMonth {
    transaction_date: String,
    opening: i64,
    opening_arrears_principal: i64,
    opening_arrears_interest: i64,
    disbursed: i64,
    principal_paid: i64,
    interest_paid: i64,
    provision: i64,
}

fn later(current: &mut String, candidate: String) {
    if candidate > *current {
        *current = candidate;
    }
}

fn period_of(business_date: &str) -> String {
    business_date.chars().take(7).collect()
}

/// Bulan berikutnya untuk periode `YYYY-MM`.
fn next_period(period: &str) -> Option<String> {
    let year: i32 = period.get(0..4)?.parse().ok()?;
    let month: u32 = period.get(5..7)?.parse().ok()?;
    Some(if month >= 12 {
        format!("{:04}-01", year + 1)
    } else {
        format!("{year:04}-{:02}", month + 1)
    })
}

fn period_range(first: &str, last: &str) -> Vec<String> {
    let mut periods = Vec::new();
    let mut current = first.to_string();
    while current.as_str() <= last {
        periods.push(current.clone());
        match next_period(&current) {
            Some(next) => current = next,
            None => break,
        }
    }
    periods
}

/// Angsuran pokok dan bunga terjadwal untuk bulan dengan saldo awal `opening`,
/// mengikuti rumus sheet: pokok = plafond / tenor dibulatkan ke atas per seribu.
fn scheduled_installment(
    plafond: i64,
    tenor: i64,
    annual_rate: f64,
    interest_type: InterestType,
    opening: i64,
) -> (i64, i64) {
    if opening <= 0 || plafond <= 0 || tenor <= 0 {
        return (0, 0);
    }
    let principal = ((plafond + tenor * 1_000 - 1) / (tenor * 1_000)) * 1_000;
    let interest_base = if interest_type == InterestType::Flat {
        plafond
    } else {
        opening
    };
    (
        principal.min(opening),
        (interest_base as f64 * annual_rate / 100.0 / 12.0).round() as i64,
    )
}

fn track_first(firsts: &mut HashMap<String, String>, id: &str, period: &str) {
    let first = firsts
        .entry(id.to_string())
        .or_insert_with(|| period.to_string());
    if period < first.as_str() {
        *first = period.to_string();
    }
}

pub(crate) async fn get_monthly_ledger(
    company_id: &str,
    year: i32,
    pool: &SqlitePool,
) -> Result<MonthlyLedgerDto, String> {
    let year_prefix = format!("{year:04}-");
    let mut all_periods: BTreeSet<String> = BTreeSet::new();

    // --- Simpanan -----------------------------------------------------------
    let mut savings_months: HashMap<(String, String), SavingsMonth> = HashMap::new();
    let mut member_first_period: HashMap<String, String> = HashMap::new();
    for row in sqlx::query(
        "SELECT sa.member_id, sa.account_type, c.component_type, c.amount, c.business_date FROM effective_components c JOIN savings_accounts sa ON sa.company_id = c.company_id AND sa.id = c.savings_account_id WHERE c.company_id = ?",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    {
        let member_id: String = col!(row, "member_id");
        let account_type: String = col!(row, "account_type");
        let account_type = SavingsAccountType::try_from(account_type.as_str())?;
        let component_type: String = col!(row, "component_type");
        let component_type = ComponentType::try_from(component_type.as_str())?;
        let amount: i64 = col!(row, "amount");
        let business_date: String = col!(row, "business_date");
        let period = period_of(&business_date);
        all_periods.insert(period.clone());
        track_first(&mut member_first_period, &member_id, &period);
        let slot = match account_type {
            SavingsAccountType::Principal => 0,
            SavingsAccountType::Mandatory => 1,
            SavingsAccountType::Voluntary => 2,
        };
        let month = savings_months.entry((member_id, period)).or_default();
        match component_type {
            ComponentType::SavingsOpening => month.opening[slot] += amount,
            ComponentType::SavingsWithdrawal => month.outgoing[slot] += amount,
            ComponentType::SavingsShu => month.shu += amount,
            ComponentType::SavingsDeposit => month.incoming[slot] += amount,
            other @ (ComponentType::LoanOpening
            | ComponentType::LoanOpeningArrearsPrincipal
            | ComponentType::LoanOpeningPrepaidPrincipal
            | ComponentType::LoanOpeningArrearsInterest
            | ComponentType::LoanOpeningPrepaidInterest
            | ComponentType::LoanDisbursement
            | ComponentType::LoanPrincipal
            | ComponentType::LoanInterest
            | ComponentType::LoanProvision
            | ComponentType::CashOther
            | ComponentType::Reversal) => {
                return Err(format!("Komponen simpanan tidak valid: {}", other.as_str()));
            }
        }
        if !component_type.is_opening() {
            later(&mut month.transaction_date, business_date);
        }
    }

    // --- Pinjaman -----------------------------------------------------------
    let mut loan_months: HashMap<(String, String), LoanMonth> = HashMap::new();
    let mut loan_first_period: HashMap<String, String> = HashMap::new();
    for row in sqlx::query(
        "SELECT c.loan_id, c.component_type, c.amount, c.business_date FROM effective_components c WHERE c.company_id = ? AND c.loan_id IS NOT NULL",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    {
        let loan_id: Option<String> = col!(row, "loan_id");
        let Some(loan_id) = loan_id else { continue };
        let component_type: String = col!(row, "component_type");
        let component_type = ComponentType::try_from(component_type.as_str())?;
        let amount: i64 = col!(row, "amount");
        let business_date: String = col!(row, "business_date");
        let period = period_of(&business_date);
        all_periods.insert(period.clone());
        track_first(&mut loan_first_period, &loan_id, &period);
        let month = loan_months.entry((loan_id, period)).or_default();
        match component_type {
            ComponentType::LoanOpening => month.opening += amount,
            ComponentType::LoanOpeningArrearsPrincipal => month.opening_arrears_principal += amount,
            ComponentType::LoanOpeningPrepaidPrincipal => month.opening_arrears_principal -= amount,
            ComponentType::LoanOpeningArrearsInterest => month.opening_arrears_interest += amount,
            ComponentType::LoanOpeningPrepaidInterest => month.opening_arrears_interest -= amount,
            ComponentType::LoanDisbursement => month.disbursed += amount,
            ComponentType::LoanPrincipal => month.principal_paid += amount,
            ComponentType::LoanInterest => month.interest_paid += amount,
            ComponentType::LoanProvision => month.provision += amount,
            other @ (ComponentType::SavingsOpening
            | ComponentType::SavingsDeposit
            | ComponentType::SavingsWithdrawal
            | ComponentType::SavingsShu
            | ComponentType::CashOther
            | ComponentType::Reversal) => {
                return Err(format!("Komponen pinjaman tidak valid: {}", other.as_str()));
            }
        }
        if !component_type.is_opening() {
            later(&mut month.transaction_date, business_date);
        }
    }

    let (Some(first), Some(last)) = (all_periods.first().cloned(), all_periods.last().cloned())
    else {
        return Ok(MonthlyLedgerDto {
            periods: Vec::new(),
            savings: Vec::new(),
            loans: Vec::new(),
        });
    };
    let timeline = period_range(&first, &last);
    let periods: Vec<String> = timeline
        .iter()
        .filter(|period| period.starts_with(&year_prefix))
        .cloned()
        .collect();

    let mut savings = Vec::new();
    let member_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM members WHERE company_id = ?")
        .bind(company_id)
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    for member_id in member_ids {
        let Some(first_period) = member_first_period.get(&member_id) else {
            continue;
        };
        let mut balance = [0_i64; 3];
        for period in timeline.iter().filter(|period| *period >= first_period) {
            let month = savings_months
                .get(&(member_id.clone(), period.clone()))
                .cloned()
                .unwrap_or_default();
            let opening = [
                balance[0] + month.opening[0],
                balance[1] + month.opening[1],
                balance[2] + month.opening[2],
            ];
            balance = [
                opening[0] + month.incoming[0] - month.outgoing[0],
                opening[1] + month.incoming[1] - month.outgoing[1],
                opening[2] + month.incoming[2] - month.outgoing[2] + month.shu,
            ];
            if period.starts_with(&year_prefix) {
                savings.push(SavingsMonthDto {
                    member_id: member_id.clone(),
                    period: period.clone(),
                    transaction_date: month.transaction_date,
                    principal_opening: opening[0],
                    mandatory_opening: opening[1],
                    voluntary_opening: opening[2],
                    principal_in: month.incoming[0],
                    principal_out: month.outgoing[0],
                    mandatory_in: month.incoming[1],
                    mandatory_out: month.outgoing[1],
                    voluntary_in: month.incoming[2],
                    voluntary_out: month.outgoing[2],
                    shu: month.shu,
                    principal_closing: balance[0],
                    mandatory_closing: balance[1],
                    voluntary_closing: balance[2],
                });
            }
        }
    }

    let mut loans = Vec::new();
    for row in sqlx::query(
        "SELECT id, plafond, rate_annual, tenor, interest_type FROM loans WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    {
        let loan_id: String = col!(row, "id");
        let plafond: i64 = col!(row, "plafond");
        let rate: f64 = col!(row, "rate_annual");
        let tenor: i64 = col!(row, "tenor");
        let interest_type: String = col!(row, "interest_type");
        let interest_type = InterestType::try_from(interest_type.as_str())?;
        let Some(first_period) = loan_first_period.get(&loan_id) else {
            continue;
        };
        let (mut balance, mut arrears_principal, mut arrears_interest) = (0_i64, 0_i64, 0_i64);
        for period in timeline.iter().filter(|period| *period >= first_period) {
            let month = loan_months
                .get(&(loan_id.clone(), period.clone()))
                .cloned()
                .unwrap_or_default();
            let opening = balance + month.opening;
            let (scheduled_principal, scheduled_interest) =
                scheduled_installment(plafond, tenor, rate, interest_type, opening);
            arrears_principal +=
                month.opening_arrears_principal + scheduled_principal - month.principal_paid;
            arrears_interest +=
                month.opening_arrears_interest + scheduled_interest - month.interest_paid;
            balance = opening + month.disbursed - month.principal_paid;
            if period.starts_with(&year_prefix) {
                loans.push(LoanMonthDto {
                    loan_id: loan_id.clone(),
                    period: period.clone(),
                    transaction_date: month.transaction_date,
                    opening_balance: opening,
                    disbursed: month.disbursed,
                    principal_paid: month.principal_paid,
                    interest_paid: month.interest_paid,
                    provision: month.provision,
                    scheduled_principal,
                    scheduled_interest,
                    arrears_principal,
                    arrears_interest,
                    closing_balance: balance,
                });
            }
        }
    }

    Ok(MonthlyLedgerDto {
        periods,
        savings,
        loans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_imported_loan_components_and_rejects_unknown_types() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            sqlx::query("INSERT INTO loans (id, company_id, member_name, plafond, rate_annual, tenor, interest_type, realization_date, due_date, status) VALUES ('imported', 'default', 'Imported member', 1000000, 24, 10, 'Menurun', '2026-01-01', '2026-11-01', 'Berjalan')")
                .execute(&pool).await.unwrap();

            // Literal persisted values exercise compatibility with imported data.
            for (id, date, transaction_type, components) in [
                (
                    "opening",
                    "2026-01-01",
                    "OPENING_LOAN",
                    vec![
                        ("LOAN_OPENING", 1_000_000),
                        ("LOAN_OPENING_ARREARS_PRINCIPAL", 50_000),
                        ("LOAN_OPENING_PREPAID_PRINCIPAL", 20_000),
                        ("LOAN_OPENING_ARREARS_INTEREST", 5_000),
                        ("LOAN_OPENING_PREPAID_INTEREST", 1_000),
                    ],
                ),
                (
                    "payment",
                    "2026-02-10",
                    "MEMBER_PAYMENT",
                    vec![
                        ("LOAN_DISBURSEMENT", 200_000),
                        ("LOAN_PRINCIPAL", 100_000),
                        ("LOAN_INTEREST", 10_000),
                        ("LOAN_PROVISION", 2_000),
                    ],
                ),
            ] {
                let amount: i64 = components.iter().map(|(_, amount)| amount).sum();
                sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_name, transaction_type, channel, description, reference, direction, amount, actor) VALUES (?, 'default', ?, ?, '00:00', 'Imported member', ?, 'NON_KAS', 'Imported', ?, 'Masuk', ?, 'Import')")
                    .bind(id).bind(date).bind(date).bind(transaction_type).bind(id).bind(amount)
                    .execute(&pool).await.unwrap();
                for (component_type, amount) in components {
                    sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES ('default', ?, ?, 'Imported', ?, 'imported')")
                        .bind(id).bind(component_type).bind(amount)
                        .execute(&pool).await.unwrap();
                }
            }

            let ledger = get_monthly_ledger("default", 2026, &pool).await.unwrap();
            let opening = &ledger.loans[0];
            assert_eq!(opening.opening_balance, 1_000_000);
            assert_eq!(opening.transaction_date, "");
            assert_eq!(
                (opening.arrears_principal, opening.arrears_interest),
                (130_000, 24_000)
            );
            let payment = &ledger.loans[1];
            assert_eq!(payment.transaction_date, "2026-02-10");
            assert_eq!(
                (
                    payment.disbursed,
                    payment.principal_paid,
                    payment.interest_paid,
                    payment.provision
                ),
                (200_000, 100_000, 10_000, 2_000)
            );
            assert_eq!(payment.closing_balance, 1_100_000);
            assert_eq!(
                (payment.arrears_principal, payment.arrears_interest),
                (130_000, 34_000)
            );

            // Unknown types must fail instead of silently disappearing from the ledger.
            sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES ('default', 'payment', 'UNKNOWN_COMPONENT', 'Invalid import', 1, 'imported')")
                .execute(&pool).await.unwrap();
            let error = get_monthly_ledger("default", 2026, &pool)
                .await
                .err()
                .unwrap();
            assert!(error.contains("ComponentType"));
            assert!(error.contains("UNKNOWN_COMPONENT"));
        });
    }

    #[test]
    fn walks_periods_across_year_end() {
        assert_eq!(
            period_range("2025-11", "2026-02"),
            vec!["2025-11", "2025-12", "2026-01", "2026-02"]
        );
        assert_eq!(next_period("2026-12").as_deref(), Some("2027-01"));
    }

    #[test]
    fn schedules_installments_like_the_sheet() {
        // L003: plafond 25 jt, 36 bulan, menurun 24%/tahun.
        assert_eq!(
            scheduled_installment(25_000_000, 36, 24.0, InterestType::Declining, 24_305_000),
            (695_000, 486_100)
        );
        // Bunga flat tetap menggunakan plafond, bukan saldo awal.
        assert_eq!(
            scheduled_installment(25_000_000, 36, 24.0, InterestType::Flat, 24_305_000),
            (695_000, 500_000)
        );
        // Sisa saldo lebih kecil dari angsuran.
        assert_eq!(
            scheduled_installment(3_200_000, 15, 24.0, InterestType::Declining, 100_000),
            (100_000, 2_000)
        );
        // Plafond belum diisi: tidak ada jadwal.
        assert_eq!(
            scheduled_installment(0, 10, 24.0, InterestType::Declining, 5_000_000),
            (0, 0)
        );
    }
}
