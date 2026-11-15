// Tunggakan and Riwayat angsuran of a member's loans on the Anggota page,
// from the monthly ledger rows of those loans up to the arrears month.
import {
  formatCurrency,
  formatDate,
  formatPeriod,
  formatPreviousPeriod,
  type Loan,
  type LoanMonth,
} from "./store/koperasi";

type ArrearsPart = "Principal" | "Interest";

/** The member's loan months the arrears are read from: `rows` holds only
 * those loans' months through `period`, oldest first. */
export interface ArrearsLedger {
  rows: readonly LoanMonth[];
  period: string;
}

/** The ledger months of `loanIds` through `period`, oldest first. */
export function arrearsLedger(
  ledgerRows: readonly LoanMonth[],
  loanIds: ReadonlySet<string>,
  period: string,
): ArrearsLedger {
  return {
    period,
    rows: ledgerRows
      .filter((row) => loanIds.has(row.loanId) && row.period <= period)
      .sort((a, b) => a.period.localeCompare(b.period)),
  };
}

/** Arrears run through the current month, even before its transactions are
 * recorded; for an earlier year, through that year's latest month. */
export function arrearsPeriodOf(periods: readonly string[], current: string) {
  const past = periods.filter((period) => period <= current);
  return past.length ? past.reduce((a, b) => (a > b ? a : b)) : "";
}

/** Whether a loan month's arrears of `part` count. A paid-off loan has no
 * principal arrears, as in the sheet: an early payoff reads as a large
 * "lebih bayar" that hides other loans' arrears. Interest it still owes
 * stays, so a payoff short on interest is not lost. */
function countsArrears(row: LoanMonth, part: ArrearsPart) {
  return (
    row.closingBalance > 0 || (part === "Interest" && row.arrearsInterest > 0)
  );
}

const rowsOf = (ledger: ArrearsLedger, loan: Loan) =>
  ledger.rows.filter((row) => row.loanId === loan.id);

/** Unpaid amount per month, oldest first: the arrears are matched against
 * the latest installments, walking back month by month. Arrears reaching
 * past the selected year are reported as "s/d" the month before it. */
function unpaidMonths(
  ledger: ArrearsLedger,
  part: ArrearsPart,
  loanList: Loan[],
) {
  const byPeriod = new Map<string, number>();
  let earlier = 0;
  let firstPeriod = "";

  for (const loan of loanList) {
    const rows = rowsOf(ledger, loan);
    const latest = rows.find((row) => row.period === ledger.period);

    let remaining =
      latest && countsArrears(latest, part) ? latest[`arrears${part}`] : 0;

    for (let i = rows.length - 1; i >= 0 && remaining > 0; i--) {
      const unpaid = Math.min(rows[i][`scheduled${part}`], remaining);
      if (unpaid <= 0) continue;

      byPeriod.set(
        rows[i].period,
        (byPeriod.get(rows[i].period) ?? 0) + unpaid,
      );

      remaining -= unpaid;
    }

    if (remaining > 0 && rows.length) {
      earlier += remaining;

      if (!firstPeriod || rows[0].period < firstPeriod)
        firstPeriod = rows[0].period;
    }
  }

  const months = [...byPeriod]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([period, amount]) => ({ label: formatPeriod(period), amount }));

  if (earlier > 0)
    months.unshift({
      label: `s/d ${formatPreviousPeriod(firstPeriod)}`,
      amount: earlier,
    });

  return months;
}

export const arrearsNote = (ledger: ArrearsLedger) =>
  ledger.period ? `Per ${formatPeriod(ledger.period)}` : "";

/** Tunggakan pokok and bunga of the given loans (see `countsArrears`). */
export function arrearsOf(
  ledger: ArrearsLedger,
  loanList: Loan[],
  withMonths: boolean,
) {
  const ids = new Set(loanList.map((loan) => loan.id));

  const latest = ledger.rows.filter(
    (row) => ids.has(row.loanId) && row.period === ledger.period,
  );

  const note = arrearsNote(ledger);

  const part = (label: string, key: ArrearsPart) => {
    const amount = latest
      .filter((row) => countsArrears(row, key))
      .reduce((sum, row) => sum + row[`arrears${key}`], 0);

    return {
      label,
      amount,
      note:
        amount < 0 ? [note, "Lebih bayar"].filter(Boolean).join(" · ") : note,
      months:
        withMonths && amount > 0 ? unpaidMonths(ledger, key, loanList) : [],
    };
  };

  return [
    part("Tunggakan pokok", "Principal"),
    part("Tunggakan bunga", "Interest"),
  ];
}

/** One month's installment (or the arrears carried into the year) and what
 * has paid it so far. */
interface Installment {
  period: string;
  due: number;
  paid: number;
  /** Date of each deposit that paid part of it; "" for one before the year. */
  paidOn: string[];
}

const paidKey = {
  Principal: "principalPaid",
  Interest: "interestPaid",
} as const;

/** Riwayat angsuran: the year's installments of one loan and when each was
 * paid. A deposit is not tied to an installment, so it pays the oldest unpaid
 * one first and any surplus pays the next months ahead. What stays unpaid is
 * then the latest months, as in `unpaidMonths`, and adds up to the arrears. */
function installmentsOf(ledger: ArrearsLedger, loan: Loan, part: ArrearsPart) {
  const rows = rowsOf(ledger, loan);
  const items: Installment[] = [];
  const credit: { amount: number; date: string }[] = [];
  const [first] = rows;
  if (!first) return { items, ahead: 0 };

  const prior =
    first[`arrears${part}`] - first[`scheduled${part}`] + first[paidKey[part]];

  if (prior > 0) items.push({ period: "", due: prior, paid: 0, paidOn: [] });
  else if (prior < 0) credit.push({ amount: -prior, date: "" });

  for (const row of rows) {
    const scheduled = row[`scheduled${part}`];

    if (scheduled > 0)
      items.push({ period: row.period, due: scheduled, paid: 0, paidOn: [] });

    const paid = row[paidKey[part]];
    if (paid > 0) credit.push({ amount: paid, date: row.transactionDate });

    for (const item of items) {
      while (item.paid < item.due && credit.length) {
        const deposit = credit[0];
        const amount = Math.min(item.due - item.paid, deposit.amount);
        item.paid += amount;
        item.paidOn.push(deposit.date);
        deposit.amount -= amount;
        if (!deposit.amount) credit.shift();
      }
    }
  }

  return { items, ahead: credit.reduce((sum, item) => sum + item.amount, 0) };
}

function installmentStatus(item: Installment, year: string) {
  const date = item.paidOn[item.paidOn.length - 1];

  const when =
    date === undefined
      ? ""
      : date === ""
        ? `sebelum ${year}`
        : `${item.period && date.slice(0, 7) < item.period ? "di muka " : ""}${formatDate(date)}`;

  if (item.paid >= item.due) return `Dibayar ${when}`;

  if (item.paid > 0)
    return `Kurang ${formatCurrency(item.due - item.paid)} · sebagian dibayar ${when}`;

  return "Belum dibayar";
}

/** Rows of the Riwayat angsuran table: arrears carried into the year first,
 * then each month, with its principal and interest installments. */
export function scheduleOf(ledger: ArrearsLedger, loan: Loan) {
  const principal = installmentsOf(ledger, loan, "Principal");
  const interest = installmentsOf(ledger, loan, "Interest");
  const year = ledger.period.slice(0, 4);
  const firstPeriod = rowsOf(ledger, loan)[0]?.period ?? "";

  const periods = [
    ...new Set(
      [...principal.items, ...interest.items].map((item) => item.period),
    ),
  ].sort();

  const cell = (item?: Installment) =>
    item && {
      amount: item.due,
      status: installmentStatus(item, year),
      unpaid: item.paid < item.due,
    };

  return {
    rows: periods.map((period) => ({
      label: period
        ? formatPeriod(period)
        : `Tunggakan s/d ${formatPreviousPeriod(firstPeriod)}`,
      principal: cell(principal.items.find((item) => item.period === period)),
      interest: cell(interest.items.find((item) => item.period === period)),
    })),
    // Paid ahead of the installments due so far; meaningless once paid off.
    ahead:
      loan.balance > 0
        ? { principal: principal.ahead, interest: interest.ahead }
        : { principal: 0, interest: 0 },
  };
}
