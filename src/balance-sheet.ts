// Neraca as in NERACA of BUKU KAS HARIAN: the ledger lines come from the
// monthly ledger and the cash book, the rest are typed per book year
// (`balance_sheet_items`). See notes/rules-and-formulas.md (Neraca).
import {
  type BalanceSheetDetail,
  CashCategory,
  cashCategories,
  type CashBook,
  type Loan,
  LoanGroup,
  type MonthlyLedger,
  TransactionStatus,
} from "./store/koperasi";

/** Parts of the Neraca, as headed in the RAT report. */
export const BalanceSheetGroup = {
  CurrentAssets: "Aktiva Lancar",
  FixedAssets: "Aktiva Tetap",
  CurrentLiabilities: "Kewajiban Jangka Pendek",
  LongTermLiabilities: "Kewajiban Jangka Panjang",
  Equity: "Modal Sendiri",
} as const;
export type BalanceSheetGroup =
  (typeof BalanceSheetGroup)[keyof typeof BalanceSheetGroup];

/** Keys of the Neraca lines typed per book year (`balance_sheet_items`; the
 * same list as `BALANCE_SHEET_ITEMS` in the backend). */
export const BalanceSheetItem = {
  BankBalance: "BANK_BALANCE",
  CashAdjustment: "CASH_ADJUSTMENT",
  OtherLoans: "OTHER_LOANS",
  UnclaimedReceivables: "UNCLAIMED_RECEIVABLES",
  AccruedExpenses: "ACCRUED_EXPENSES",
  FixedAssets: "FIXED_ASSETS",
  DebtPpbri: "DEBT_PPBRI",
  NonMemberSavings: "NON_MEMBER_SAVINGS",
  CapitalReserve: "CAPITAL_RESERVE",
  EducationFund: "EDUCATION_FUND",
  SocialFund: "SOCIAL_FUND",
  PmdkFund: "PMDK_FUND",
  LongTermDebt: "LONG_TERM_DEBT",
  RatReserve: "RAT_RESERVE",
  LoanWriteOffReserve: "LOAN_WRITE_OFF_RESERVE",
  RetainedEarnings: "RETAINED_EARNINGS",
  // Pos added by hand to a group, beside its fixed lines (a list each).
  ExtraCurrentAssets: "EXTRA_CURRENT_ASSETS",
  ExtraCurrentLiabilities: "EXTRA_CURRENT_LIABILITIES",
  ExtraLongTermLiabilities: "EXTRA_LONG_TERM_LIABILITIES",
  ExtraEquity: "EXTRA_EQUITY",
} as const;
export type BalanceSheetItem =
  (typeof BalanceSheetItem)[keyof typeof BalanceSheetItem];

export const balanceSheetItems = [
  {
    key: BalanceSheetItem.BankBalance,
    label: "Bank (saldo 31 Desember)",
    hint: "Saldo buku tabungan bank per 31 Desember. Isi 0 agar dihitung dari Kas Buku Tabungan",
  },
  {
    key: BalanceSheetItem.CashAdjustment,
    label: "Penyesuaian kas",
    hint: "Ditambahkan ke saldo Buku Kas",
  },
  {
    key: BalanceSheetItem.OtherLoans,
    label: "Piutang Lainnya",
    // A list (Bon Sementara: a bon per row); the line is its total.
    detailed: {
      dateLabel: "Tanggal pinjam",
      addLabel: "Tambah bon",
      keepName: true,
    },
  },
  {
    key: BalanceSheetItem.UnclaimedReceivables,
    label: "Piutang Tidak Bertuan",
  },
  {
    key: BalanceSheetItem.AccruedExpenses,
    label: "Beban yang harus dibayar",
  },
  {
    key: BalanceSheetItem.FixedAssets,
    label: "Aktiva Tetap",
    // A list of pos per book year (Perlengkapan Kantor, Peralatan Kantor,
    // Akum. Penyusutan as a Pengurang, …); each is a Neraca line.
    detailed: {
      dateLabel: "Tanggal perolehan",
      addLabel: "Tambah pos",
      keepName: false,
    },
  },
  {
    key: BalanceSheetItem.DebtPpbri,
    label: "Hutang ke KOWABRI / PP",
  },
  {
    key: BalanceSheetItem.NonMemberSavings,
    label: "Simpanan Non Anggota",
  },
  {
    key: BalanceSheetItem.CapitalReserve,
    label: "Cadangan Modal",
  },
  {
    key: BalanceSheetItem.EducationFund,
    label: "Dana Pendidikan",
  },
  {
    key: BalanceSheetItem.SocialFund,
    label: "Dana Sosial",
  },
  {
    key: BalanceSheetItem.PmdkFund,
    label: "Dana PMDK (Pembangunan Daerah Kerja)",
  },
  {
    key: BalanceSheetItem.LongTermDebt,
    label: "Kewajiban jangka panjang",
  },
  {
    key: BalanceSheetItem.RatReserve,
    label: "Cadangan RAT",
  },
  {
    key: BalanceSheetItem.LoanWriteOffReserve,
    label: "Cadangan Penghapusan",
  },
  {
    key: BalanceSheetItem.RetainedEarnings,
    label: "Laba ditahan tahun lalu",
  },
  ...(
    [
      [BalanceSheetItem.ExtraCurrentAssets, "Aktiva Lancar lainnya"],
      [
        BalanceSheetItem.ExtraCurrentLiabilities,
        "Kewajiban Jangka Pendek lainnya",
      ],
      [
        BalanceSheetItem.ExtraLongTermLiabilities,
        "Kewajiban Jangka Panjang lainnya",
      ],
      [BalanceSheetItem.ExtraEquity, "Modal Sendiri lainnya"],
    ] as const
  ).map(([key, label]) => ({
    key,
    label,
    detailed: { dateLabel: "Tanggal", addLabel: "Tambah pos", keepName: false },
  })),
] as const;

/** The list of pos of each Neraca group: added, renamed and removed in the
 * Neraca itself ("+ Pos" on the group heading). */
export const groupListKey: Record<BalanceSheetGroup, BalanceSheetItem> = {
  [BalanceSheetGroup.CurrentAssets]: BalanceSheetItem.ExtraCurrentAssets,
  [BalanceSheetGroup.FixedAssets]: BalanceSheetItem.FixedAssets,
  [BalanceSheetGroup.CurrentLiabilities]:
    BalanceSheetItem.ExtraCurrentLiabilities,
  [BalanceSheetGroup.LongTermLiabilities]:
    BalanceSheetItem.ExtraLongTermLiabilities,
  [BalanceSheetGroup.Equity]: BalanceSheetItem.ExtraEquity,
};
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

export const LineEditKind = {
  /** A typed line of the book year (`balance_sheet_items`). */
  Item: "item",
  /** One pos of a list (`balance_sheet_details`), by its name. */
  Detail: "detail",
} as const;

/** How a Neraca cell is edited in place: a typed line of the book year, or
 * one pos of a list (Aktiva Tetap) by its name. */
export type LineEdit =
  /** Saves the typed amount less `offset` (Kas: Penyesuaian kas = the Kas
   * typed − the Buku Kas saldo). */
  | { kind: typeof LineEditKind.Item; key: BalanceSheetItem; offset?: number }
  | { kind: typeof LineEditKind.Detail; key: BalanceSheetItem; name: string };
export type BalanceSheetLine = {
  /** Matches the line across columns (labels can carry the year). */
  id: string;
  label: string;
  amount: number;
  /** Derived where the sheet types it (Bank without a typed balance). */
  estimated?: boolean;
  /** Set where the amount is typed, not derived. */
  edit?: LineEdit;
};
export type BalanceSheetSection = {
  title: string;
  /** Its list of pos added by hand. */
  listKey: BalanceSheetItem;
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
  details: Record<string, BalanceSheetDetail[]> = {},
) {
  const item = (key: BalanceSheetItemKey) => items[key] ?? 0;
  // One line per pos of a group's list (Aktiva Tetap is only its list).
  const listLines = (group: BalanceSheetGroup): BalanceSheetLine[] => {
    const key = groupListKey[group];
    return (details[key] ?? []).map((detail) => ({
      id: `${key}:${detail.name}`,
      label: detail.name,
      amount: detail.amount,
      edit: { kind: LineEditKind.Detail, key, name: detail.name },
    }));
  };
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
  // Catatan of the RAT Neraca: Piutang Non Anggota per Rekomendasi.
  const recommenderOf = new Map(
    loans.map((loan) => [loan.id, loan.recommender.trim()]),
  );
  const nonMemberByRecommender = [
    ...loanRows
      .filter((row) => groupOf.get(row.loanId) === LoanGroup.NonMember)
      .reduce((totals, row) => {
        const name = recommenderOf.get(row.loanId) ?? "";
        return totals.set(name, (totals.get(name) ?? 0) + row.closingBalance);
      }, new Map<string, number>()),
  ]
    .map(([recommender, amount]) => ({ recommender, amount }))
    .filter((entry) => entry.amount !== 0)
    // Named recommenders first, as in the sheet; the unnamed rest last.
    .sort(
      (a, b) =>
        Number(!a.recommender) - Number(!b.recommender) ||
        a.recommender.localeCompare(b.recommender),
    );
  const { cash, bank, postedRows } = cashAndBankAt(period, cashBook);
  // 0 means not typed: a bank balance of exactly 0 is not expected.
  const typedBank =
    period.endsWith("-12") && items[BalanceSheetItem.BankBalance]
      ? items[BalanceSheetItem.BankBalance]
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

  // The group's added pos go last, or before `lastLines` (Modal Sendiri:
  // before the Laba lines).
  const section = (
    title: BalanceSheetGroup,
    totalLabel: string,
    fixedLines: BalanceSheetLine[],
    lastLines: BalanceSheetLine[] = [],
  ): BalanceSheetSection => {
    const lines = [...fixedLines, ...listLines(title), ...lastLines];
    return {
      title,
      listKey: groupListKey[title],
      lines,
      totalLabel,
      total: sum(lines.map((line) => line.amount)),
    };
  };
  const typed = (
    key: BalanceSheetItem,
    label?: string,
    editable = true,
  ): BalanceSheetLine => ({
    id: key,
    label: label ?? balanceSheetItems.find((entry) => entry.key === key)!.label,
    amount: item(key),
    edit: editable ? { kind: LineEditKind.Item, key } : undefined,
  });

  // Labels and order as in the Neraca of the RAT report.
  const assets = [
    section(BalanceSheetGroup.CurrentAssets, "Jumlah Aktiva Lancar", [
      {
        id: "cash",
        label: "Kas",
        amount: cash + item(BalanceSheetItem.CashAdjustment),
        edit: {
          kind: LineEditKind.Item,
          key: BalanceSheetItem.CashAdjustment,
          offset: cash,
        },
      },
      {
        id: "bank",
        label: "Bank",
        ...(typedBank === undefined
          ? { amount: bank, estimated: true }
          : { amount: typedBank }),
        // The typed balance is the one at 31 Desember.
        edit: period.endsWith("-12")
          ? { kind: LineEditKind.Item, key: BalanceSheetItem.BankBalance }
          : undefined,
      },
      {
        id: "member-loans",
        label: "Piutang Anggota",
        amount: loanBalance(LoanGroup.Member),
      },
      {
        id: "non-member-loans",
        label: "Piutang Non Anggota",
        amount: loanBalance(LoanGroup.NonMember),
      },
      // The total of its list, edited under Catatan.
      typed(BalanceSheetItem.OtherLoans, undefined, false),
      typed(BalanceSheetItem.UnclaimedReceivables),
      typed(BalanceSheetItem.AccruedExpenses),
    ]),
    section(BalanceSheetGroup.FixedAssets, "Jumlah Aktiva Tetap", []),
  ];
  const liabilities = [
    section(
      BalanceSheetGroup.CurrentLiabilities,
      "Jumlah Kewajiban Jangka Pendek",
      [
        {
          id: "voluntary-savings",
          label: "Simpanan Manasuka",
          amount: sum(savingsRows.map((row) => row.voluntaryClosing)),
        },
        typed(BalanceSheetItem.DebtPpbri),
        typed(BalanceSheetItem.NonMemberSavings),
        typed(BalanceSheetItem.CapitalReserve),
        typed(BalanceSheetItem.EducationFund),
        typed(BalanceSheetItem.SocialFund),
        typed(BalanceSheetItem.PmdkFund),
      ],
    ),
    section(
      BalanceSheetGroup.LongTermLiabilities,
      "Jumlah Kewajiban Jangka Panjang",
      [typed(BalanceSheetItem.LongTermDebt, "")],
    ),
    section(
      BalanceSheetGroup.Equity,
      "Jumlah Modal Sendiri",
      [
        {
          id: "principal-savings",
          label: "Simpanan Pokok",
          amount: sum(savingsRows.map((row) => row.principalClosing)),
        },
        {
          id: "mandatory-savings",
          label: "Simpanan Wajib",
          amount: sum(savingsRows.map((row) => row.mandatoryClosing)),
        },
        typed(BalanceSheetItem.RatReserve),
        typed(BalanceSheetItem.LoanWriteOffReserve),
      ],
      [
        typed(
          BalanceSheetItem.RetainedEarnings,
          `Laba ditahan tahun ${year - 1}`,
        ),
        { id: "net-income", label: `Laba Tahun ${year}`, amount: netIncome },
      ],
    ),
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
    nonMemberByRecommender,
  };
}
export type BalanceSheet = ReturnType<typeof balanceSheetAt>;

type SheetCell =
  | { kind: "heading"; label: string; listKey: BalanceSheetItem }
  | {
      kind: "line";
      label: string;
      /** 1-based position in its section. */
      number: number;
      /** The label without the number. */
      name: string;
      /** Set where the line is a pos of a group's list: it can be
       * renamed and removed in place. */
      listKey?: BalanceSheetItem;
      amounts: number[];
      estimated: boolean[];
      /** Per column; undefined where the amount is derived. */
      edits: (LineEdit | undefined)[];
    }
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
      // Columns can list different lines (Aktiva Tetap is a list per book
      // year), so lines are matched by id, in order of first appearance; the
      // label is the last column's.
      const ids = [
        ...new Set(parts.flatMap((part) => part.lines.map((line) => line.id))),
      ];
      const lineOf = (part: BalanceSheetSection, id: string) =>
        part.lines.find((line) => line.id === id);
      return [
        { kind: "heading", label: first.title, listKey: first.listKey },
        ...ids.map((id, lineIndex): SheetCell => {
          // No Array.prototype.at: the Windows 7 WebView2 runtime is Chromium 88.
          const labels = parts
            .map((part) => lineOf(part, id)?.label)
            .filter((value) => value !== undefined);
          const label = labels[labels.length - 1];
          // A pos of a list missing in one year can be typed there too:
          // saving adds it to that year's list.
          const listEdit = parts
            .map((part) => lineOf(part, id)?.edit)
            .find((edit) => edit?.kind === LineEditKind.Detail);
          return {
            kind: "line",
            label: `${lineIndex + 1}. ${label}`,
            number: lineIndex + 1,
            name: label ?? "",
            listKey: listEdit?.key,
            amounts: parts.map((part) => lineOf(part, id)?.amount ?? 0),
            estimated: parts.map((part) => !!lineOf(part, id)?.estimated),
            edits: parts.map((part) => lineOf(part, id)?.edit ?? listEdit),
          };
        }),
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
