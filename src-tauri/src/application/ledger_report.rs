//! Year-end report columns derived from the monthly ledger, as in the RAT
//! workbooks (DATA PINJAMAN / DATA SIMPANAN POSISI DESEMBER 2025):
//! paid-off loans, Jumlah Pemasukan per triwulan, Insentif Pinjaman, Ratas
//! Simpanan and Insentif Simpanan. See notes/rules-and-formulas.md 1.10, 2.3a.

use std::collections::{HashMap, HashSet};

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
/// income moves to the running loan realized last (`realized`: loan id to
/// realization date, `YYYY-MM-DD`), so it still counts toward the SHU.
/// A borrower with only paid-off loans keeps just the one realized last, which
/// takes over the income of the others. A loan without a borrower key is its
/// own borrower.
///
/// A Pinjaman Sementara (`temporary`: loan ids) takes the income only when the
/// borrower has no running Bulanan loan. Otherwise the running Bulanan loan
/// realized last takes it, and a running Sementara loan stays listed with its
/// income moved there too, as AGUS SUGIARTO HAJI in DATA PINJAMAN POSISI
/// DESEMBER 2025 (see notes/rules-and-formulas.md 1.10).
pub(crate) fn apply_loan_report(
    loans: &mut [LoanMonthDto],
    borrowers: &HashMap<String, String>,
    realized: &HashMap<String, String>,
    temporary: &HashSet<String>,
    incentive_rate: f64,
) {
    let mut order: Vec<usize> = (0..loans.len()).collect();
    order.sort_by(|&a, &b| {
        (&loans[a].loan_id, &loans[a].period).cmp(&(&loans[b].loan_id, &loans[b].period))
    });
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
    }

    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (index, row) in loans.iter().enumerate() {
        let borrower = borrowers
            .get(&row.loan_id)
            .filter(|borrower| !borrower.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("loan:{}", row.loan_id));
        groups
            .entry((row.period.clone(), borrower))
            .or_default()
            .push(index);
    }
    for group in groups.values() {
        let (closed, open): (Vec<usize>, Vec<usize>) =
            group.iter().partition(|&&index| is_closed(&loans[index]));
        let realized_last = |candidates: &[usize]| {
            candidates.iter().copied().max_by(|&a, &b| {
                let realized = |index: usize| realized.get(&loans[index].loan_id);
                realized(a)
                    .cmp(&realized(b))
                    .then_with(|| loans[b].loan_id.cmp(&loans[a].loan_id))
            })
        };
        let is_temporary = |index: usize| temporary.contains(&loans[index].loan_id);
        let running_monthly: Vec<usize> = open
            .iter()
            .copied()
            .filter(|&index| !is_temporary(index))
            .collect();
        let Some(host) = realized_last(&running_monthly)
            .or_else(|| realized_last(&open))
            .or_else(|| realized_last(&closed))
        else {
            continue;
        };
        let running_temporary: Vec<usize> = if is_temporary(host) {
            Vec::new()
        } else {
            open.iter()
                .copied()
                .filter(|&index| is_temporary(index))
                .collect()
        };
        let carried = closed
            .iter()
            .map(|&index| (index, false))
            .chain(running_temporary.iter().map(|&index| (index, true)));
        for (index, shown) in carried.filter(|&(index, _)| index != host) {
            let (interest, provision) =
                (loans[index].income_interest, loans[index].income_provision);
            for quarter in 0..4 {
                loans[host].income_interest[quarter] += interest[quarter];
                loans[host].income_provision[quarter] += provision[quarter];
            }
            let row = &mut loans[index];
            row.shown = shown;
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
        let realized = HashMap::from([
            ("old".to_string(), "2025-12-10".to_string()),
            ("new".to_string(), "2026-01-28".to_string()),
        ]);
        apply_loan_report(&mut loans, &borrowers, &realized, &HashSet::new(), 0.101);

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
        apply_loan_report(
            &mut loans,
            &borrowers,
            &HashMap::new(),
            &HashSet::new(),
            0.101,
        );
        assert!(loans[0].shown);
    }

    #[test]
    fn keeps_only_the_last_loan_when_every_loan_is_paid_off() {
        // A borrower in September 2026 whose two loans were both paid off earlier.
        let mut old_jan = loan("old", "2026-01", 3_000_000, 0);
        old_jan.interest_paid = 60_000;
        let mut new_may = loan("new", "2026-05", 0, 0);
        new_may.disbursed = 2_000_000;
        new_may.provision = 10_000;
        let mut loans = vec![
            old_jan,
            new_may,
            loan("old", "2026-09", 0, 0),
            loan("new", "2026-09", 0, 0),
        ];
        let borrowers = HashMap::from([
            ("old".to_string(), "M109".to_string()),
            ("new".to_string(), "M109".to_string()),
        ]);
        let realized = HashMap::from([
            ("old".to_string(), "2025-05-21".to_string()),
            ("new".to_string(), "2026-04-22".to_string()),
        ]);
        apply_loan_report(&mut loans, &borrowers, &realized, &HashSet::new(), 0.101);
        assert!(!loans[2].shown);
        assert_eq!(loans[2].income_interest, [0; 4]);
        assert!(loans[3].shown);
        assert_eq!(loans[3].income_interest, [60_000, 0, 0, 0]);
        assert_eq!(loans[3].income_provision, [0, 10_000, 0, 0]);
    }

    #[test]
    fn does_not_merge_loans_without_a_borrower() {
        let mut loans = vec![loan("a", "2026-09", 0, 0), loan("b", "2026-09", 0, 0)];
        apply_loan_report(
            &mut loans,
            &HashMap::new(),
            &HashMap::new(),
            &HashSet::new(),
            0.101,
        );
        assert!(loans[0].shown && loans[1].shown);
    }

    #[test]
    fn carries_income_to_the_running_loan_realized_last() {
        // Both running loans were realized before the book year, the later
        // one under the larger id.
        let mut paid_jan = loan("c", "2026-01", 1_000_000, 0);
        paid_jan.interest_paid = 20_000;
        let mut loans = vec![
            paid_jan,
            loan("c", "2026-02", 0, 0),
            loan("a", "2026-02", 5_000_000, 5_000_000),
            loan("b", "2026-02", 5_000_000, 5_000_000),
        ];
        let borrowers = HashMap::from([
            ("a".to_string(), "M001".to_string()),
            ("b".to_string(), "M001".to_string()),
            ("c".to_string(), "M001".to_string()),
        ]);
        let realized = HashMap::from([
            ("a".to_string(), "2024-03-01".to_string()),
            ("b".to_string(), "2025-06-01".to_string()),
            ("c".to_string(), "2023-01-01".to_string()),
        ]);
        apply_loan_report(&mut loans, &borrowers, &realized, &HashSet::new(), 0.101);
        assert!(!loans[1].shown);
        assert_eq!(loans[2].income_interest, [0; 4]);
        assert_eq!(loans[3].income_interest, [20_000, 0, 0, 0]);
    }

    #[test]
    fn carries_income_to_the_last_monthly_loan_not_a_temporary_one() {
        // AGUS SUGIARTO HAJI, DATA PINJAMAN POSISI DESEMBER 2025 (row 16): the
        // paid-off loans and the running Sementara loan of 22-Okt-2025 all go
        // to the Bulanan loan of 01-Okt-2025; the Sementara row stays, empty.
        let mut paid = loan("paid", "2025-05", 5_000_000, 0);
        paid.interest_paid = 450_000;
        paid.provision = 12_500;
        let mut monthly = loan("monthly", "2025-10", 0, 5_000_000);
        monthly.disbursed = 5_000_000;
        monthly.interest_paid = 195_000;
        monthly.provision = 12_500;
        let mut temporary = loan("temporary", "2025-10", 0, 500_000);
        temporary.disbursed = 500_000;
        temporary.interest_paid = 30_000;
        let mut loans = vec![
            paid,
            monthly,
            temporary,
            loan("paid", "2025-12", 0, 0),
            loan("monthly", "2025-12", 5_000_000, 4_500_000),
            loan("temporary", "2025-12", 500_000, 500_000),
        ];
        let borrowers = HashMap::from([
            ("paid".to_string(), "M053".to_string()),
            ("monthly".to_string(), "M053".to_string()),
            ("temporary".to_string(), "M053".to_string()),
        ]);
        let realized = HashMap::from([
            ("paid".to_string(), "2024-12-02".to_string()),
            ("monthly".to_string(), "2025-10-01".to_string()),
            ("temporary".to_string(), "2025-10-22".to_string()),
        ]);
        let temporary_ids = HashSet::from(["temporary".to_string()]);
        apply_loan_report(&mut loans, &borrowers, &realized, &temporary_ids, 0.101);

        // December: the paid-off loan is hidden, the Sementara loan is listed
        // without income, and the Bulanan loan carries all of it.
        assert!(!loans[3].shown);
        assert!(loans[5].shown);
        assert_eq!(loans[5].income_interest, [0; 4]);
        assert_eq!(loans[4].income_interest, [0, 450_000, 0, 225_000]);
        assert_eq!(loans[4].income_provision, [0, 12_500, 0, 12_500]);
        assert_eq!(loans[5].incentive, 0);
    }

    #[test]
    fn a_temporary_loan_takes_the_income_without_a_monthly_loan() {
        let mut paid = loan("paid", "2026-01", 1_000_000, 0);
        paid.interest_paid = 20_000;
        let mut loans = vec![
            paid,
            loan("paid", "2026-02", 0, 0),
            loan("temporary", "2026-02", 2_000_000, 2_000_000),
        ];
        let borrowers = HashMap::from([
            ("paid".to_string(), "M001".to_string()),
            ("temporary".to_string(), "M001".to_string()),
        ]);
        let temporary_ids = HashSet::from(["temporary".to_string()]);
        apply_loan_report(
            &mut loans,
            &borrowers,
            &HashMap::new(),
            &temporary_ids,
            0.101,
        );
        assert!(!loans[1].shown);
        assert_eq!(loans[2].income_interest, [20_000, 0, 0, 0]);
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
