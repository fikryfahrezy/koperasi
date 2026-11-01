// Neraca as in NERACA of BUKU KAS HARIAN: the ledger lines come from the
// monthly ledger and the cash book, the rest are typed per book year
// (`balance_sheet_items`). See notes/rules-and-formulas.md (Neraca).
import {
  CashCategory,
  cashCategories,
  type CashBook,
  type Loan,
  LoanGroup,
  type MonthlyLedger,
  TransactionStatus,
} from "./store/koperasi";

export const balanceSheetItems = [
  {
    key: "BANK_BALANCE",
    label: "Bank (saldo 31 Desember)",
    hint: "Saldo buku tabungan bank per 31 Desember. Isi 0 agar dihitung dari Kas Buku Tabungan",
  },
  {
    key: "CASH_ADJUSTMENT",
    label: "Penyesuaian kas",
    hint: "Ditambahkan ke saldo Buku Kas",
  },
  { key: "OTHER_LOANS", label: "Pinjaman Lainnya" },
  { key: "FIXED_ASSETS_DESK", label: "Meja" },
  { key: "FIXED_ASSETS_CHAIR", label: "Kursi" },
  { key: "FIXED_ASSETS", label: "Lap-Top" },
  { key: "DEBT_PPBRI", label: "Hutang ke PPBRI" },
  { key: "NON_MEMBER_SAVINGS", label: "Simpanan Non Anggota" },
  { key: "CAPITAL_RESERVE", label: "Cadangan Modal" },
  { key: "EDUCATION_FUND", label: "Dana Pendidikan" },
  { key: "SOCIAL_FUND", label: "Dana Sosial" },
  { key: "PMDK_FUND", label: "Dana PMDK (Pembangunan Daerah Kerja)" },
  { key: "LONG_TERM_DEBT", label: "Hutang jangka panjang" },
  { key: "RAT_RESERVE", label: "Cadangan RAT" },
  { key: "LOAN_WRITE_OFF_RESERVE", label: "Cadangan Penghapusan Pinjaman" },
  { key: "RETAINED_EARNINGS", label: "Laba ditahan tahun lalu" },
] as const;
export type BalanceSheetItemKey = (typeof balanceSheetItems)[number]["key"];

/** Biaya columns of Buku Kas (L–W), subtracted from income for Laba Bersih. */
export const expenseCategories = cashCategories.filter(
  (category) =>
    ![
      CashCategory.CashTopUp,
      CashCategory.CashTopUpFromSavingsBook,
      CashCategory.TransferToSavingsBook,
    ].includes(category.value as never),
);

export type BalanceSheetLine = {
  label: string;
  amount: number;
  /** Derived where the sheet types it (Bank without a typed balance). */
  estimated?: boolean;
};
export type BalanceSheetSection = {
  title: string;
  lines: BalanceSheetLine[];
  totalLabel: string;
  total: number;
};

const sum = (values: number[]) =>
  values.reduce((total, value) => total + value, 0);

/** Kas (Buku Kas saldo) and Bank (Kas Buku Tabungan) at the end of `period`,
 * before the typed adjustments, and the Buku Kas rows counted. */
export function cashAndBankAt(period: string, cashBook: CashBook | undefined) {
  // Rows are in date order.
  const cashRows = (cashBook?.rows ?? []).filter(
    (row) => row.businessDate.slice(0, 7) <= period,
  );
  const lastCashRow = cashRows[cashRows.length - 1];
  const cash = lastCashRow?.balance ?? cashBook?.openingBalance ?? 0;
  // Deleted rows and their reversals are left out, as on Buku Kas.
  const postedRows = cashRows.filter(
    (row) => row.status !== TransactionStatus.Reversed && !row.isReversal,
  );
  const bank =
    (cashBook?.savingsBookOpeningBalance ?? 0) +
    sum(postedRows.map((row) => row.savingsBookIn - row.savingsBookOut));
  return { cash, bank, postedRows };
}

/** Neraca at the end of `period` ("YYYY-MM"). `items` are the typed lines of
 * that book year. Bank is the typed 31 December balance in December, and
 * otherwise Kas Buku Tabungan, which misses money moved only at the bank. */
export function balanceSheetAt(
  period: string,
  ledger: MonthlyLedger,
  cashBook: CashBook | undefined,
  loans: Loan[],
  items: Record<string, number>,
) {
  const item = (key: BalanceSheetItemKey) => items[key] ?? 0;
  const year = Number(period.slice(0, 4));
  const groupOf = new Map(loans.map((loan) => [loan.id, loan.loanGroup]));
  const loanRows = ledger.loans.filter((row) => row.period === period);
  const savingsRows = ledger.savings.filter((row) => row.period === period);
  const loanBalance = (group: LoanGroup) =>
    sum(
      loanRows
        .filter((row) => groupOf.get(row.loanId) === group)
        .map((row) => row.closingBalance),
    );
  const { cash, bank, postedRows } = cashAndBankAt(period, cashBook);
  // 0 means not typed: a bank balance of exactly 0 is not expected.
  const typedBank =
    period.endsWith("-12") && items.BANK_BALANCE
      ? items.BANK_BALANCE
      : undefined;

  // Laba Bersih: Bunga + Provisi from January, less the Buku Kas expenses.
  const monthsSoFar = ledger.loans.filter((row) => row.period <= period);
  const interest = sum(monthsSoFar.map((row) => row.interestPaid));
  const provision = sum(monthsSoFar.map((row) => row.provision));
  const expenses = expenseCategories.map((category) => ({
    label: category.label,
    amount: sum(
      postedRows.flatMap((row) =>
        row.components
          .filter((part) => part.componentType === category.value)
          .map((part) => part.amount),
      ),
    ),
  }));
  const expenseTotal = sum(expenses.map((expense) => expense.amount));
  const netIncome = interest + provision - expenseTotal;

  const section = (
    title: string,
    totalLabel: string,
    lines: BalanceSheetLine[],
  ): BalanceSheetSection => ({
    title,
    lines,
    totalLabel,
    total: sum(lines.map((line) => line.amount)),
  });
  const typed = (key: BalanceSheetItemKey, label?: string) => ({
    label: label ?? balanceSheetItems.find((entry) => entry.key === key)!.label,
    amount: item(key),
  });

  // Labels and order as in the sheet.
  const assets = [
    section("Aktiva Lancar", "Jumlah Aktiva Lancar", [
      { label: "Kas", amount: cash + item("CASH_ADJUSTMENT") },
      typedBank === undefined
        ? { label: "Bank", amount: bank, estimated: true }
        : { label: "Bank", amount: typedBank },
      { label: "Pinjaman Anggota", amount: loanBalance(LoanGroup.Member) },
      {
        label: "Pinjaman Non Anggota",
        amount: loanBalance(LoanGroup.NonMember),
      },
      typed("OTHER_LOANS"),
    ]),
    section("Aktiva Tetap", "Jumlah Aktiva Tetap", [
      typed("FIXED_ASSETS_DESK"),
      typed("FIXED_ASSETS_CHAIR"),
      typed("FIXED_ASSETS"),
    ]),
  ];
  const liabilities = [
    section("Hutang Lancar", "Jumlah Hutang Lancar", [
      {
        label: "Simpanan Manasuka",
        amount: sum(savingsRows.map((row) => row.voluntaryClosing)),
      },
      typed("DEBT_PPBRI"),
      typed("NON_MEMBER_SAVINGS"),
      typed("CAPITAL_RESERVE"),
      typed("EDUCATION_FUND"),
      typed("SOCIAL_FUND"),
      typed("PMDK_FUND"),
    ]),
    section("Hutang Jangka Panjang", "Jumlah Hutang Jangka Panjang", [
      typed("LONG_TERM_DEBT", ""),
    ]),
    section("Modal Sendiri", "Total Modal", [
      {
        label: "Simpanan Pokok",
        amount: sum(savingsRows.map((row) => row.principalClosing)),
      },
      {
        label: "Simpanan Wajib",
        amount: sum(savingsRows.map((row) => row.mandatoryClosing)),
      },
      typed("RAT_RESERVE"),
      typed("LOAN_WRITE_OFF_RESERVE"),
      typed("RETAINED_EARNINGS", `Laba ditahan tahun ${year - 1}`),
      { label: "Laba Bersih", amount: netIncome },
    ]),
  ];
  const totalAssets = sum(assets.map((part) => part.total));
  const totalLiabilities = sum(liabilities.map((part) => part.total));
  return {
    assets,
    liabilities,
    totalAssets,
    totalLiabilities,
    difference: totalAssets - totalLiabilities,
    income: { interest, provision, expenses, expenseTotal, netIncome },
  };
}
export type BalanceSheet = ReturnType<typeof balanceSheetAt>;

type SheetCell =
  | { kind: "heading"; label: string }
  | { kind: "line"; label: string; amounts: number[]; estimated: boolean[] }
  | { kind: "total"; label: string; amounts: number[] }
  | { kind: "blank" };
export type SheetRow = { left: SheetCell; right: SheetCell };

/** The NERACA layout: Aktiva and Pasiva side by side, one column per sheet
 * compared (e.g. 31 Desember last year, then the month shown). Aktiva Lancar
 * faces Hutang Lancar; Aktiva Tetap faces Hutang Jangka Panjang and Modal. */
export function sheetRows(sheets: BalanceSheet[]): SheetRow[] {
  const cells = (
    pick: (sheet: BalanceSheet) => BalanceSheetSection[],
    indexes: number[],
  ) =>
    indexes.flatMap((index): SheetCell[] => {
      const parts = sheets.map((sheet) => pick(sheet)[index]);
      const [first] = parts;
      return [
        { kind: "heading", label: first.title },
        ...first.lines.map(
          (line, lineIndex): SheetCell => ({
            kind: "line",
            label: `${lineIndex + 1}. ${line.label}`,
            amounts: parts.map((part) => part.lines[lineIndex].amount),
            estimated: parts.map((part) => !!part.lines[lineIndex].estimated),
          }),
        ),
        {
          kind: "total",
          label: first.totalLabel,
          amounts: parts.map((part) => part.total),
        },
        { kind: "blank" },
      ];
    });
  const zip = (left: SheetCell[], right: SheetCell[]) =>
    Array.from({ length: Math.max(left.length, right.length) }, (_, index) => ({
      left: left[index] ?? { kind: "blank" as const },
      right: right[index] ?? { kind: "blank" as const },
    }));
  return [
    ...zip(
      cells((sheet) => sheet.assets, [0]),
      cells((sheet) => sheet.liabilities, [0]),
    ),
    ...zip(
      cells((sheet) => sheet.assets, [1]),
      cells((sheet) => sheet.liabilities, [1, 2]),
    ),
    {
      left: {
        kind: "total",
        label: "TOTAL AKTIVA",
        amounts: sheets.map((sheet) => sheet.totalAssets),
      },
      right: {
        kind: "total",
        label: "TOTAL PASIVA",
        amounts: sheets.map((sheet) => sheet.totalLiabilities),
      },
    },
  ];
}
