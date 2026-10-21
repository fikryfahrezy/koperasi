//! Monthly savings and loan ledgers, arranged like an Excel workbook.
//!
//! No dedicated tables: each month is derived by summing components of
//! effective transactions (the `effective_components` view) by date. Migrated 2026 history is also
//! read from previously stored transactions.
//! Payment obligations and loan arrears are calculated from loan terms.

use std::collections::{BTreeSet, HashMap, HashSet};

use sqlx::{Row, SqlitePool};

use super::ledger_report::{apply_loan_report, apply_savings_report};
use super::parameters::incentive_rates;
use crate::contracts::{LoanMonthDto, MonthlyLedgerDto, SavingsMonthDto};
use crate::domain::{round_up_amount, ComponentType, InterestType, LoanType, SavingsAccountType};

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

/// Next month for a `YYYY-MM` period.
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

/// `YYYY-MM` of a stored date: `2026-04-05` or `05-Apr-2026` (as in the sheet).
fn date_period(value: &str) -> Option<String> {
    iso_date(value).map(|date| date[..7].to_string())
}

/// `YYYY-MM-DD` for a date stored as `YYYY-MM-DD` or `DD-Mon-YYYY`.
fn iso_date(value: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let bytes = value.as_bytes();
    if value.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        return Some(value.to_string());
    }
    let mut parts = value.split('-');
    let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
    let day: u32 = day.parse().ok()?;
    let month = MONTHS.iter().position(|name| *name == month)? + 1;
    (year.len() == 4 && year.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| format!("{year}-{month:02}-{day:02}"))
}

/// Scheduled principal and interest installments for a month with an `opening` balance,
/// following PINJAMAN BULANAN:
/// - Bulanan: principal = loan limit / term, rounded up to the nearest thousand.
/// - Interest keeps its sen, as in the sheet; the ledger rounds up only what it
///   shows (see `get_monthly_ledger`).
/// - Sementara: no principal installments; the whole balance falls due in the
///   jatuh tempo month.
/// - After the jatuh tempo month nothing new falls due; what is unpaid stays in arrears.
#[allow(clippy::too_many_arguments)]
fn scheduled_installment(
    plafond: i64,
    tenor: i64,
    annual_rate: f64,
    interest_type: InterestType,
    loan_type: Option<LoanType>,
    due_period: Option<&str>,
    period: &str,
    opening: i64,
) -> (i64, f64) {
    if opening <= 0 || plafond <= 0 || tenor <= 0 {
        return (0, 0.0);
    }
    if due_period.is_some_and(|due| period > due) {
        return (0, 0.0);
    }
    let principal = if loan_type == Some(LoanType::Temporary) {
        if due_period == Some(period) {
            opening
        } else {
            0
        }
    } else {
        ((plafond + tenor * 1_000 - 1) / (tenor * 1_000)) * 1_000
    };
    let interest_base = if interest_type == InterestType::Flat {
        plafond
    } else {
        opening
    };
    (
        principal.min(opening),
        interest_base as f64 * annual_rate / 100.0 / 12.0,
    )
}

/// Keeps only the entities (by `key`) with at least one non-empty row.
fn drop_empty<T>(rows: &mut Vec<T>, key: impl Fn(&T) -> &String, empty: impl Fn(&T) -> bool) {
    let active: HashSet<String> = rows
        .iter()
        .filter(|row| !empty(row))
        .map(|row| key(row).clone())
        .collect();
    rows.retain(|row| active.contains(key(row)));
}

fn track_first(firsts: &mut HashMap<String, String>, id: &str, period: &str) {
    let first = firsts
        .entry(id.to_string())
        .or_insert_with(|| period.to_string());
    if period < first.as_str() {
        *first = period.to_string();
    }
}

/// `through` (`YYYY-MM`) extends the months past the last recorded one, so
/// installments falling due since then count toward arrears.
pub(crate) async fn get_monthly_ledger(
    company_id: &str,
    year: i32,
    through: Option<&str>,
    pool: &SqlitePool,
) -> Result<MonthlyLedgerDto, String> {
    let year_prefix = format!("{year:04}-");
    let mut all_periods: BTreeSet<String> = BTreeSet::new();

    // --- Savings -----------------------------------------------------------
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
            other => {
                return Err(format!("Komponen simpanan tidak valid: {}", other.as_str()));
            }
        }
        if !component_type.is_opening() {
            later(&mut month.transaction_date, business_date);
        }
    }

    // --- Loans -----------------------------------------------------------
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
            other => {
                return Err(format!("Komponen pinjaman tidak valid: {}", other.as_str()));
            }
        }
        if !component_type.is_opening() {
            later(&mut month.transaction_date, business_date);
        }
    }

    let (Some(first), Some(last)) = (all_periods.first().cloned(), all_periods.last().cloned())
    else {
        let (loan_incentive_rate, savings_incentive_rate) =
            incentive_rates(company_id, year, pool).await?;
        return Ok(MonthlyLedgerDto {
            loan_incentive_rate,
            savings_incentive_rate,
            periods: Vec::new(),
            savings: Vec::new(),
            loans: Vec::new(),
        });
    };
    let last = match through {
        Some(through) if through > last.as_str() => through.to_string(),
        _ => last,
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
                    ..Default::default()
                });
            }
        }
    }

    let mut loans = Vec::new();
    let mut borrowers = HashMap::new();
    let mut realized = HashMap::new();
    for row in sqlx::query(
        "SELECT id, COALESCE(NULLIF(member_id, ''), 'name:' || member_name) borrower, plafond, rate_annual, tenor, interest_type, loan_type, realization_date, due_date FROM loans WHERE company_id = ?",
    )
    .bind(company_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    {
        let loan_id: String = col!(row, "id");
        borrowers.insert(loan_id.clone(), col!(row, "borrower"));
        let realization_date: String = col!(row, "realization_date");
        if let Some(date) = iso_date(&realization_date) {
            realized.insert(loan_id.clone(), date);
        }
        let plafond: i64 = col!(row, "plafond");
        let rate: f64 = col!(row, "rate_annual");
        let tenor: i64 = col!(row, "tenor");
        let interest_type: String = col!(row, "interest_type");
        let interest_type = InterestType::try_from(interest_type.as_str())?;
        let loan_type: Option<String> = col!(row, "loan_type");
        let loan_type = loan_type
            .map(|value| LoanType::try_from(value.as_str()))
            .transpose()?;
        let due_date: String = col!(row, "due_date");
        let due_period = date_period(&due_date);
        let Some(first_period) = loan_first_period.get(&loan_id) else {
            continue;
        };
        // Interest arrears keep their sen from month to month, as the sheet
        // does (e.g. DF = DC + DA*D/12); only the amounts shown are rounded up
        // to a whole rupiah, so Tunggakan -30.000,12 shows as -30.000 and a
        // Kewajiban of 160.146,50 as 160.147.
        let (mut balance, mut arrears_principal, mut arrears_interest) = (0_i64, 0_i64, 0.0_f64);
        for period in timeline.iter().filter(|period| *period >= first_period) {
            let month = loan_months
                .get(&(loan_id.clone(), period.clone()))
                .cloned()
                .unwrap_or_default();
            let opening = balance + month.opening;
            let (scheduled_principal, scheduled_interest) = scheduled_installment(
                plafond,
                tenor,
                rate,
                interest_type,
                loan_type,
                due_period.as_deref(),
                period,
                opening,
            );
            arrears_principal +=
                month.opening_arrears_principal + scheduled_principal - month.principal_paid;
            let prior_arrears_interest = arrears_interest + month.opening_arrears_interest as f64;
            let due_interest = prior_arrears_interest + scheduled_interest;
            arrears_interest = due_interest - month.interest_paid as f64;
            balance = opening + month.disbursed - month.principal_paid;
            // Arrears never exceed what is still owed, as in the sheet.
            arrears_principal = arrears_principal.min(balance.max(0));
            if period.starts_with(&year_prefix) {
                // Next month's installment, for "Kewajiban s/d bulan depan".
                let (next_scheduled_principal, next_scheduled_interest) = next_period(period)
                    .map(|next| {
                        scheduled_installment(
                            plafond,
                            tenor,
                            rate,
                            interest_type,
                            loan_type,
                            due_period.as_deref(),
                            &next,
                            balance,
                        )
                    })
                    .unwrap_or((0, 0.0));
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
                    // Shown so that Tunggakan bulan lalu + this = Kewajiban s/d,
                    // each rounded up, as the page and the export add them.
                    scheduled_interest: round_up_amount(due_interest)
                        - round_up_amount(prior_arrears_interest),
                    arrears_principal,
                    arrears_interest: round_up_amount(arrears_interest),
                    closing_balance: balance,
                    next_due_principal: arrears_principal + next_scheduled_principal,
                    next_due_interest: round_up_amount(arrears_interest + next_scheduled_interest),
                    ..Default::default()
                });
            }
        }
    }

    // Members and loans whose history ended before this year (every month zero,
    // e.g. loans paid off in an earlier year) are left out of the year.
    drop_empty(
        &mut savings,
        |row| &row.member_id,
        |row| {
            [
                row.principal_opening,
                row.mandatory_opening,
                row.voluntary_opening,
                row.principal_in,
                row.principal_out,
                row.mandatory_in,
                row.mandatory_out,
                row.voluntary_in,
                row.voluntary_out,
                row.shu,
                row.principal_closing,
                row.mandatory_closing,
                row.voluntary_closing,
            ]
            .iter()
            .all(|amount| *amount == 0)
        },
    );
    drop_empty(
        &mut loans,
        |row| &row.loan_id,
        |row| {
            [
                row.opening_balance,
                row.disbursed,
                row.principal_paid,
                row.interest_paid,
                row.provision,
                row.scheduled_principal,
                row.scheduled_interest,
                row.arrears_principal,
                row.arrears_interest,
                row.closing_balance,
            ]
            .iter()
            .all(|amount| *amount == 0)
        },
    );
    let (loan_incentive_rate, savings_incentive_rate) =
        incentive_rates(company_id, year, pool).await?;
    apply_loan_report(&mut loans, &borrowers, &realized, loan_incentive_rate);
    apply_savings_report(&mut savings, &periods, savings_incentive_rate);
    Ok(MonthlyLedgerDto {
        loan_incentive_rate,
        savings_incentive_rate,
        periods,
        savings,
        loans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Channel, TransactionDirection};

    #[test]
    fn schedules_installments_like_the_loan_sheet() {
        assert_eq!(date_period("05-Apr-2026").as_deref(), Some("2026-04"));
        assert_eq!(date_period("2026-04-05").as_deref(), Some("2026-04"));
        assert_eq!(date_period("-"), None);
        assert_eq!(iso_date("5-Apr-2026").as_deref(), Some("2026-04-05"));
        let monthly = |period, opening| {
            scheduled_installment(
                20_000_000,
                24,
                24.0,
                InterestType::Declining,
                Some(LoanType::Monthly),
                Some("2027-11"),
                period,
                opening,
            )
        };
        assert_eq!(monthly("2026-01", 19_166_600), (834_000, 383_332.0));
        // Past jatuh tempo nothing new falls due.
        let overdue = scheduled_installment(
            35_000_000,
            20,
            18.0,
            InterestType::Flat,
            Some(LoanType::Monthly),
            Some("2016-11"),
            "2026-01",
            20_000_000,
        );
        assert_eq!(overdue, (0, 0.0));
        // Sementara: interest monthly, the whole balance in the jatuh tempo month.
        let temporary = |period| {
            scheduled_installment(
                5_000_000,
                3,
                24.0,
                InterestType::Flat,
                Some(LoanType::Temporary),
                Some("2026-04"),
                period,
                5_000_000,
            )
        };
        assert_eq!(temporary("2026-02"), (0, 100_000.0));
        assert_eq!(temporary("2026-04"), (5_000_000, 100_000.0));
        assert_eq!(temporary("2026-05"), (0, 0.0));
    }

    #[test]
    fn reads_imported_loan_components_and_rejects_unknown_types() {
        tauri::async_runtime::block_on(async {
            let pool = crate::database::memory_pool().await;
            sqlx::query("INSERT INTO loans (id, company_id, loan_group, member_name, plafond, rate_annual, tenor, interest_type, loan_type, realization_date, due_date) VALUES ('imported', 'default', 'Anggota PP BRI', 'Imported member', 1000000, 24, 10, 'Menurun', 'Bulanan', '2026-01-01', '2026-11-01')")
                .execute(&pool).await.unwrap();

            // Literal persisted values exercise compatibility with imported data.
            for (id, date, components) in [
                (
                    "opening",
                    "2026-01-01",
                    vec![
                        (ComponentType::LoanOpening.as_str(), 1_000_000),
                        (ComponentType::LoanOpeningArrearsPrincipal.as_str(), 50_000),
                        (ComponentType::LoanOpeningPrepaidPrincipal.as_str(), 20_000),
                        (ComponentType::LoanOpeningArrearsInterest.as_str(), 5_000),
                        (ComponentType::LoanOpeningPrepaidInterest.as_str(), 1_000),
                    ],
                ),
                (
                    "payment",
                    "2026-02-10",
                    vec![
                        (ComponentType::LoanDisbursement.as_str(), 200_000),
                        (ComponentType::LoanPrincipal.as_str(), 100_000),
                        (ComponentType::LoanInterest.as_str(), 10_000),
                        (ComponentType::LoanProvision.as_str(), 2_000),
                    ],
                ),
            ] {
                let amount: i64 = components.iter().map(|(_, amount)| amount).sum();
                sqlx::query("INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_name, channel, description, reference, direction, amount, actor) VALUES (?, 'default', ?, ?, '00:00', 'Imported member', ?, 'Imported', ?, ?, ?, 'Import')")
                    .bind(id).bind(date).bind(date).bind(Channel::NonCash.as_str()).bind(id)
                    .bind(TransactionDirection::In.as_str()).bind(amount)
                    .execute(&pool).await.unwrap();
                for (component_type, amount) in components {
                    sqlx::query("INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id) VALUES ('default', ?, ?, 'Imported', ?, 'imported')")
                        .bind(id).bind(component_type).bind(amount)
                        .execute(&pool).await.unwrap();
                }
            }

            let ledger = get_monthly_ledger("default", 2026, None, &pool)
                .await
                .unwrap();
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
            let error = get_monthly_ledger("default", 2026, None, &pool)
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
        // L003: loan limit of 25 million, 36 months, declining interest at 24% per year.
        assert_eq!(
            scheduled_installment(
                25_000_000,
                36,
                24.0,
                InterestType::Declining,
                Some(LoanType::Monthly),
                None,
                "2026-01",
                24_305_000
            ),
            (695_000, 486_100.0)
        );
        // Flat interest still uses the loan limit rather than the opening balance.
        assert_eq!(
            scheduled_installment(
                25_000_000,
                36,
                24.0,
                InterestType::Flat,
                Some(LoanType::Monthly),
                None,
                "2026-01",
                24_305_000
            ),
            (695_000, 500_000.0)
        );
        // The remaining balance is smaller than the installment.
        assert_eq!(
            scheduled_installment(
                3_200_000,
                15,
                24.0,
                InterestType::Declining,
                Some(LoanType::Monthly),
                None,
                "2026-01",
                100_000
            ),
            (100_000, 2_000.0)
        );
        // Interest keeps its sen (YULIATUN, Agustus 2026: 9.507.331 * 2%).
        let (_, interest) = scheduled_installment(
            13_900_000,
            30,
            24.0,
            InterestType::Declining,
            Some(LoanType::Monthly),
            None,
            "2026-08",
            9_507_331,
        );
        assert!((interest - 190_146.62).abs() < 1e-6);
        // No loan limit provided: no repayment schedule.
        assert_eq!(
            scheduled_installment(
                0,
                10,
                24.0,
                InterestType::Declining,
                Some(LoanType::Monthly),
                None,
                "2026-01",
                5_000_000
            ),
            (0, 0.0)
        );
    }
}
