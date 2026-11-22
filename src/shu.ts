import type { LoanMonth, SavingsMonth } from "./store/koperasi";

/** One member's row of the Daftar Penerimaan SHU (RAT workbook DAFTAR SHU
 * TAHUN BUKU): SHU from Ratas Simpanan, SHU from Jasa Pinjaman (only for
 * members who borrowed), uang duduk, and the total rounded up to the thousand. */
export interface ShuRow {
  memberId: string;
  /** Ratas Simpanan from January to the last month and its SHU. */
  savingsAverage: number;
  savingsShu: number;
  /** Jasa pinjaman (interest + provisi paid this year) of all the member's
   * loans and its SHU. */
  loanIncome: number;
  loanShu: number;
  /** savingsShu + loanShu. */
  shu: number;
  sittingAllowance: number;
  /** shu + sittingAllowance, and that rounded up to the thousand. */
  total: number;
  rounded: number;
}

export type ShuTotals = Omit<ShuRow, "memberId">;

/** Rounded up to the thousand, as ROUNDUP(x; -3) in the sheet. The small
 * margin keeps float noise (e.g. 419000.0000001) from adding a thousand. */
export const roundUpThousand = (value: number) =>
  Math.ceil(value / 1000 - 1e-9) * 1000;

/**
 * Builds the SHU rows of a book year from the monthly ledger, as of `period`
 * (the last month of the year shown). The shares are exact, as in the sheet;
 * only the total is rounded up.
 *
 * `borrowerOf` maps a loan id to its member id (empty for Non Anggota, who get
 * no SHU). `memberIds` are the members registered in the year: they are listed
 * even without savings or loans, and still get the uang duduk.
 */
export function shuRows(input: {
  periods: readonly string[];
  period: string;
  savings: readonly SavingsMonth[];
  loans: readonly LoanMonth[];
  borrowerOf: ReadonlyMap<string, string>;
  memberIds?: readonly string[];
  savingsRate: number;
  loanRate: number;
  sittingAllowance: number;
}): ShuRow[] {
  const months = input.periods.filter((period) => period <= input.period);
  const yearMonths = new Set(months);
  const savingsSum = new Map<string, number>();

  for (const row of input.savings) {
    // Penabung Non Anggota get no SHU.
    if (!yearMonths.has(row.period) || row.nonMember) continue;

    const total =
      row.principalClosing + row.mandatoryClosing + row.voluntaryClosing;

    savingsSum.set(row.memberId, (savingsSum.get(row.memberId) ?? 0) + total);
  }

  // Each loan's income from January up to its last month on or before `period`.
  const lastLoanMonth = new Map<string, LoanMonth>();

  for (const row of input.loans) {
    if (!yearMonths.has(row.period)) continue;
    const last = lastLoanMonth.get(row.loanId);
    if (!last || row.period > last.period) lastLoanMonth.set(row.loanId, row);
  }

  const loanIncome = new Map<string, number>();

  for (const row of lastLoanMonth.values()) {
    const memberId = input.borrowerOf.get(row.loanId);
    if (!memberId) continue;

    const income =
      row.incomeInterest.reduce((sum, value) => sum + value, 0) +
      row.incomeProvision.reduce((sum, value) => sum + value, 0);

    loanIncome.set(memberId, (loanIncome.get(memberId) ?? 0) + income);
  }

  const memberIds = new Set([
    ...(input.memberIds ?? []),
    ...savingsSum.keys(),
    ...loanIncome.keys(),
  ]);
  const divisor = Math.max(months.length, 1);

  return [...memberIds]
    .sort((a, b) => a.localeCompare(b))
    .map((memberId) => {
      const savingsAverage = (savingsSum.get(memberId) ?? 0) / divisor;
      const income = loanIncome.get(memberId) ?? 0;
      const savingsShu = savingsAverage * input.savingsRate;
      const loanShu = income * input.loanRate;
      const shu = savingsShu + loanShu;
      const total = shu + input.sittingAllowance;

      return {
        memberId,
        savingsAverage,
        savingsShu,
        loanIncome: income,
        loanShu,
        shu,
        sittingAllowance: input.sittingAllowance,
        total,
        rounded: roundUpThousand(total),
      };
    });
}

/** Whether a member was registered by the end of `year`. A join date that is
 * not a date (e.g. "Migrasi 2026" for imported members) counts as registered. */
export function registeredBy(joinedAt: string, year: number) {
  return !/^\d{4}-\d{2}-\d{2}/.test(joinedAt) || joinedAt <= `${year}-12-31`;
}

export function sumShuRows(rows: readonly ShuRow[]): ShuTotals {
  const sum: ShuTotals = {
    savingsAverage: 0,
    savingsShu: 0,
    loanIncome: 0,
    loanShu: 0,
    shu: 0,
    sittingAllowance: 0,
    total: 0,
    rounded: 0,
  };

  for (const row of rows) {
    for (const key of Object.keys(sum) as (keyof ShuTotals)[])
      sum[key] += row[key];
  }

  return sum;
}
