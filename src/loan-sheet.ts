// PINJAMAN BULANAN as in the working sheet: one row per loan per month, the
// derived columns, and the section totals. See notes/rules-and-formulas.md
// (Pinjaman).
import {
  type Loan,
  LoanGroup,
  type LoanMonth,
  matchesMember,
  roundUpInterest,
} from "./store/koperasi";

/** Kewajiban setor tiap bulan (kolom L–N): the loan's fixed installment, as
 * the export writes it: ROUNDUP(plafond/jangka waktu,-3) and
 * ROUNDUP(plafond*bunga/12,0). */
export function fixedInstallment(loan: Loan) {
  const principal =
    loan.tenor > 0 ? Math.ceil(loan.plafond / loan.tenor / 1000) * 1000 : 0;

  const interest = roundUpInterest((loan.plafond * loan.rate) / 100 / 12);
  return { principal, interest };
}

/** Bunga per bulan as shown next to Bunga per tahun. */
export const monthlyRate = (loan: Loan) =>
  `${(loan.rate / 12).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;

// Jumlah pemasukan per triwulan and Insentif Pinjaman (kolom EJ–EW) come from
// the backend: as in the working sheet every loan keeps its own income, and a
// paid-off loan stays listed to the end of the year (rules 1.10). Total
// pemasukan tahun, between them, adds up the four quarters: the base of the
// insentif.
export const quarterLabels = [
  "Jan s/d Mar",
  "Apr s/d Jun",
  "Jul s/d Sep",
  "Okt s/d Des",
] as const;

export const incomeKeys = [
  ...quarterLabels.flatMap(
    (_, quarter) =>
      [
        `q${quarter + 1}Interest`,
        `q${quarter + 1}Provision`,
        `q${quarter + 1}Total`,
      ] as const,
  ),
  "incomeInterestTotal",
  "incomeProvisionTotal",
  "incomeTotal",
  "incentiveRaw",
  "incentive",
] as const;

type IncomeKey = (typeof incomeKeys)[number];

// Kewajiban s/d bulan depan (kolom EE–EG after September): arrears plus next
// month's installment.
export const nextDueKeys = [
  "nextDuePrincipal",
  "nextDueInterest",
  "nextDueTotal",
] as const;

/** The month after `period` ("2026-12" → "2027-01"). */
export function nextPeriod(period: string) {
  const [year, month] = period.split("-").map(Number);

  return month === 12
    ? `${year + 1}-01`
    : `${year}-${String(month + 1).padStart(2, "0")}`;
}

// Numeric columns after the loan term, in spreadsheet order. Saldo Posisi,
// Kewajiban setor tiap bulan and Tunggakan before the first month shown appear
// once (kolom K–Q), as in the sheet; each month then starts at Kewajiban s/d.
export const openingKeys = [
  "openingBalance",
  "installmentPrincipal",
  "installmentInterest",
  "installmentTotal",
  "priorArrearsPrincipal",
  "priorArrearsInterest",
  "priorArrearsTotal",
] as const;

export const beforeMutation = [
  "duePrincipal",
  "dueInterest",
  "dueTotal",
] as const;

export const mutation = [
  "cashIn",
  "disbursed",
  "principalPaid",
  "interestPaid",
  "provision",
] as const;

export const afterMutation = [
  "closingBalance",
  "arrearsPrincipal",
  "arrearsInterest",
  "arrearsTotal",
] as const;

/** No, Nama, Plafond, Bunga (2), PG/TN, Jenis, Realisasi, Jatuh Tempo,
 * Jangka Waktu. */
export const leadingColumnWidths = [
  56, 240, 120, 80, 80, 80, 100, 120, 120, 90,
];

// Each month: Kewajiban s/d, the mutation date, the mutation, then the closing.
export const monthColumnCount =
  beforeMutation.length + 1 + mutation.length + afterMutation.length;

export const openingColumnCount = openingKeys.length;
// Kewajiban s/d bulan depan, Rekomendasi, then the Pemasukan columns.
export const tailColumnCount = nextDueKeys.length + 1 + incomeKeys.length;

/** One loan's month of the sheet with its derived columns. Spreadsheet rows
 * follow loan numbers (L001, L002, ...). Opening arrears = closing arrears -
 * this month's payment obligation + amount paid; payment obligation through
 * this month = opening arrears + this month's payment obligation. Only the
 * rows the backend marks `shown`. */
export function loanRowsForPeriod(
  ledgerRows: readonly LoanMonth[],
  loanById: ReadonlyMap<string, Loan>,
  period: string,
) {
  return ledgerRows
    .filter((row) => row.period === period && row.shown)
    .map((row) => ({ ...row, loan: loanById.get(row.loanId) }))
    .filter((row): row is typeof row & { loan: Loan } => Boolean(row.loan))
    .sort((a, b) => a.loanId.localeCompare(b.loanId))
    .map((row) => {
      const { priorArrearsPrincipal, priorArrearsInterest } = row;
      const installment = fixedInstallment(row.loan);

      return {
        ...row,
        plafond: row.loan.plafond,
        installmentPrincipal: installment.principal,
        installmentInterest: installment.interest,
        installmentTotal: installment.principal + installment.interest,
        priorArrearsTotal: priorArrearsPrincipal + priorArrearsInterest,
        duePrincipal: priorArrearsPrincipal + row.scheduledPrincipal,
        dueInterest: priorArrearsInterest + row.scheduledInterest,
        dueTotal:
          priorArrearsPrincipal +
          row.scheduledPrincipal +
          priorArrearsInterest +
          row.scheduledInterest,
        // As in the sheet: Pokok + Bunga paid; a disbursement without a
        // payment shows its Provisi.
        cashIn: row.principalPaid + row.interestPaid || row.provision,
        arrearsTotal: row.arrearsPrincipal + row.arrearsInterest,
        ...incomeOf(row),
        nextDueTotal: row.nextDuePrincipal + row.nextDueInterest,
      };
    });
}

export type LoanSheetRow = ReturnType<typeof loanRowsForPeriod>[number];

function incomeOf(row: LoanMonth) {
  const income = {} as Record<IncomeKey, number>;

  row.incomeInterest.forEach((amount, quarter) => {
    const provision = row.incomeProvision[quarter] ?? 0;
    income[`q${quarter + 1}Interest` as IncomeKey] = amount;
    income[`q${quarter + 1}Provision` as IncomeKey] = provision;
    income[`q${quarter + 1}Total` as IncomeKey] = amount + provision;
  });

  income.incomeInterestTotal = row.incomeInterest.reduce(
    (sum, amount) => sum + amount,
    0,
  );

  income.incomeProvisionTotal = row.incomeProvision.reduce(
    (sum, amount) => sum + amount,
    0,
  );

  income.incomeTotal = income.incomeInterestTotal + income.incomeProvisionTotal;
  income.incentiveRaw = row.incentiveRaw;
  income.incentive = row.incentive;
  return income;
}

const totalKeys = [
  "plafond",
  ...openingKeys,
  ...beforeMutation,
  ...mutation,
  ...afterMutation,
  ...incomeKeys,
  ...nextDueKeys,
] as const;

type TotalKey = (typeof totalKeys)[number];

export function sumLoanRows(rows: readonly LoanSheetRow[]) {
  const sum = Object.fromEntries(totalKeys.map((key) => [key, 0])) as Record<
    TotalKey,
    number
  >;

  for (const row of rows) {
    for (const key of totalKeys) sum[key] += row[key];
  }

  return sum;
}

const sectionTitles = [
  { group: LoanGroup.Member, title: "PINJAMAN ANGGOTA PP BRI" },
  { group: LoanGroup.NonMember, title: "PINJAMAN NON ANGGOTA PP BRI" },
] as const;

/** Sections of PINJAMAN BULANAN; numbering restarts in each section, as in
 * the sheet, before the search narrows the rows. */
export function loanSections(
  monthRows: readonly LoanSheetRow[],
  filter: {
    group: LoanGroup | "";
    query: string;
    memberNumbers: ReadonlyMap<string, string>;
  },
) {
  return sectionTitles
    .filter((section) => !filter.group || section.group === filter.group)
    .map((section) => {
      const rows = monthRows
        .filter((row) => row.loan.loanGroup === section.group)
        .map((row, index) => ({ ...row, no: index + 1 }))
        .filter((row) =>
          matchesMember(
            filter.query,
            row.loan.memberName,
            filter.memberNumbers.get(row.loan.memberId),
          ),
        );

      return { ...section, rows, totals: sumLoanRows(rows) };
    });
}

export type LoanSection = ReturnType<typeof loanSections>[number];
