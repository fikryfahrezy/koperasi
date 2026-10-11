//! Year-end report columns derived from the monthly ledger, as in the RAT
//! workbooks (DATA PINJAMAN / DATA SIMPANAN POSISI DESEMBER 2025):
//! paid-off loans, Jumlah Pemasukan per triwulan, Insentif Pinjaman, Ratas
//! Simpanan and Insentif Simpanan. See docs/rules-and-formulas.md 1.10, 2.3a.

use std::collections::HashMap;

use crate::contracts::{LoanMonthDto, SavingsMonthDto};

/// Insentif as in the sheet: the exact share (rounded to rupiah for display)
/// and the share rounded up to the next thousand (Pembulatan).
fn incentive(base: f64, rate: f64) -> (i64, i64) {
    let share = base * rate;
    (
        share.round() as i64,
        ((share / 1000.0).ceil() * 1000.0) as i64,
    )
}

fn quarter_of(period: &str) -> usize {
    let month: usize = period.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    (month.clamp(1, 12) - 1) / 3
}

/// Paid off in an earlier month; the payoff month itself is still shown.
fn is_closed(row: &LoanMonthDto) -> bool {
    row.opening_balance <= 0 && row.closing_balance <= 0 && row.disbursed == 0
}

/// Fills `shown`, the quarterly income and Insentif Pinjaman (`incentive_rate`
/// of the income, decided at the RAT for the year) of every loan month.
///
/// Income is interest and provisi paid from January up to that month. A loan
/// paid off before the month is hidden once its borrower (`borrowers`: loan id
/// to member id, or name for Non Anggota) has another loan still running; its
/// income moves to the running loan disbursed last, so it still counts toward
/// the SHU. Borrowers with only paid-off loans keep those rows.
pub(crate) fn apply_loan_report(
    loans: &mut [LoanMonthDto],
    borrowers: &HashMap<String, String>,
    incentive_rate: f64,
) {
    let mut order: Vec<usize> = (0..loans.len()).collect();
    order.sort_by(|&a, &b| {
        (&loans[a].loan_id, &loans[a].period).cmp(&(&loans[b].loan_id, &loans[b].period))
    });
    let mut latest_disbursement: HashMap<String, String> = HashMap::new();
    let mut running: Option<(String, [i64; 4], [i64; 4])> = None;
    for &index in &order {
        let row = &mut loans[index];
        let (interest, provision) = match &mut running {
            Some((loan_id, interest, provision)) if *loan_id == row.loan_id => {
                (interest, provision)
            }
            _ => {
                running = Some((row.loan_id.clone(), [0; 4], [0; 4]));
                let (_, interest, provision) = running.as_mut().unwrap();
                (interest, provision)
            }
        };
        interest[quarter_of(&row.period)] += row.interest_paid;
        provision[quarter_of(&row.period)] += row.provision;
        row.income_interest = *interest;
        row.income_provision = *provision;
        row.shown = true;
        if row.disbursed > 0 {
            let latest = latest_disbursement.entry(row.loan_id.clone()).or_default();
            if row.period > *latest {
                *latest = row.period.clone();
            }
        }
    }

    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (index, row) in loans.iter().enumerate() {
        let borrower = borrowers.get(&row.loan_id).cloned().unwrap_or_default();
        groups
            .entry((row.period.clone(), borrower))
            .or_default()
            .push(index);
    }
    for group in groups.values() {
        let (closed, open): (Vec<usize>, Vec<usize>) =
            group.iter().partition(|&&index| is_closed(&loans[index]));
        let Some(&host) = open.iter().max_by(|&&a, &&b| {
            let latest = |index: usize| latest_disbursement.get(&loans[index].loan_id);
            latest(a)
                .cmp(&latest(b))
                .then_with(|| loans[b].loan_id.cmp(&loans[a].loan_id))
        }) else {
            continue;
        };
        for index in closed {
            let (interest, provision) =
                (loans[index].income_interest, loans[index].income_provision);
            for quarter in 0..4 {
                loans[host].income_interest[quarter] += interest[quarter];
                loans[host].income_provision[quarter] += provision[quarter];
            }
            let row = &mut loans[index];
            row.shown = false;
            row.income_interest = [0; 4];
            row.income_provision = [0; 4];
        }
    }

    for row in loans.iter_mut() {
        let income: i64 =
            row.income_interest.iter().sum::<i64>() + row.income_provision.iter().sum::<i64>();
        (row.incentive_raw, row.incentive) = incentive(income as f64, incentive_rate);
    }
}

/// Fills Ratas Simpanan and Insentif Simpanan of every savings month: the
/// average Total Simpanan from January up to that month (months before a member
/// joined count as 0; at year end this divides by 12), and Insentif Simpanan
/// (`incentive_rate` of it, e.g. 3,65% for 2025).
pub(crate) fn apply_savings_report(
    savings: &mut [SavingsMonthDto],
    periods: &[String],
    incentive_rate: f64,
) {
    let mut order: Vec<usize> = (0..savings.len()).collect();
    order.sort_by(|&a, &b| {
        (&savings[a].member_id, &savings[a].period)
            .cmp(&(&savings[b].member_id, &savings[b].period))
    });
    let mut running: Option<(String, i64)> = None;
    for index in order {
        let row = &mut savings[index];
        let total = row.principal_closing + row.mandatory_closing + row.voluntary_closing;
        let sum = match &mut running {
            Some((member_id, sum)) if *member_id == row.member_id => {
                *sum += total;
                *sum
            }
            _ => {
                running = Some((row.member_id.clone(), total));
                total
            }
        };
        let months = periods
            .iter()
            .filter(|period| **period <= row.period)
            .count()
            .max(1);
        let average = sum as f64 / months as f64;
        row.savings_average = average.round() as i64;
        (row.savings_incentive_raw, row.savings_incentive) = incentive(average, incentive_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loan(loan_id: &str, period: &str, opening: i64, closing: i64) -> LoanMonthDto {
        LoanMonthDto {
            loan_id: loan_id.into(),
            period: period.into(),
            opening_balance: opening,
            closing_balance: closing,
            ..Default::default()
        }
    }

    #[test]
    fn hides_a_replaced_loan_and_carries_its_income() {
        // EMAN SULAEMAN HAJI: the December 2025 loan is paid off on 28 Jan
        // 2026 with 500.000 interest; the new loan is disbursed the same day
        // with 12.500 provisi.
        let mut old_jan = loan("old", "2026-01", 25_000_000, 0);
        old_jan.interest_paid = 500_000;
        let mut new_jan = loan("new", "2026-01", 0, 25_000_000);
        new_jan.disbursed = 25_000_000;
        new_jan.provision = 12_500;
        let mut loans = vec![
            old_jan,
            new_jan,
            loan("old", "2026-09", 0, 0),
            loan("new", "2026-09", 25_000_000, 25_000_000),
        ];
        let borrowers = HashMap::from([
            ("old".to_string(), "M043".to_string()),
            ("new".to_string(), "M043".to_string()),
        ]);
        apply_loan_report(&mut loans, &borrowers, 0.101);

        // January: both rows shown, each with its own income.
        assert!(loans[0].shown && loans[1].shown);
        assert_eq!(loans[0].income_interest, [500_000, 0, 0, 0]);
        // September: only the new loan, carrying the old loan's interest.
        assert!(!loans[2].shown);
        assert_eq!(loans[3].income_interest, [500_000, 0, 0, 0]);
        assert_eq!(loans[3].income_provision, [12_500, 0, 0, 0]);
        // 10,1% × 512.500 = 51.762,5 → 52.000
        assert_eq!(
            (loans[3].incentive_raw, loans[3].incentive),
            (51_763, 52_000)
        );
    }

    #[test]
    fn keeps_paid_off_loans_of_a_borrower_without_another_loan() {
        let mut paid = loan("paid", "2026-03", 0, 0);
        paid.interest_paid = 0;
        let mut loans = vec![paid];
        let borrowers = HashMap::from([("paid".to_string(), "M001".to_string())]);
        apply_loan_report(&mut loans, &borrowers, 0.101);
        assert!(loans[0].shown);
    }

    #[test]
    fn averages_savings_like_the_2025_sheet() {
        // ANAH ROHANAH, DATA SIMPANAN POSISI DESEMBER 2025, row 10.
        let totals = [
            6_997_630, 6_997_630, 7_072_630, 7_072_630, 7_147_630, 7_147_630, 7_347_630, 7_397_630,
            7_447_630, 7_497_630, 7_547_630, 7_597_630,
        ];
        let periods: Vec<String> = (1..=12).map(|month| format!("2025-{month:02}")).collect();
        let mut savings: Vec<SavingsMonthDto> = periods
            .iter()
            .zip(totals)
            .map(|(period, total)| SavingsMonthDto {
                member_id: "M001".into(),
                period: period.clone(),
                mandatory_closing: total,
                ..Default::default()
            })
            .collect();
        apply_savings_report(&mut savings, &periods, 0.0365);
        let december = &savings[11];
        assert_eq!(december.savings_average, 7_272_630);
        // 3,65% × 7.272.630 = 265.450,995 → 266.000 (kolom DN, DO)
        assert_eq!(
            (december.savings_incentive_raw, december.savings_incentive),
            (265_451, 266_000)
        );
    }
}
