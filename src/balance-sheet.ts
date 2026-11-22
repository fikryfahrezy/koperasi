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
 * same list as `BALANCE_SHEET_ITEMS` in the backend, except the typed LABA
 * RUGI keys, which only data scripts write). */
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
  /** Laba of a book year whose transactions are not all in the app (2025:
   * pinjaman migrated as one lump in December), typed from the sheet. */
  NetIncome: "NET_INCOME",
  /** LABA RUGI of such a year, typed from the sheet: Pendapatan Bunga and
   * Provisi, and each Biaya as `EXPENSE_<Buku Kas category>`
   * (`typedExpenseKey`). Used only where the year has none counted. */
  IncomeInterest: "INCOME_INTEREST",
  IncomeProvision: "INCOME_PROVISION",
  /** Added to the derived Piutang and Simpanan Manasuka lines, for a book
   * year whose transactions are not in the app (2025): its Neraca as in the
   * sheet. */
  MemberLoansAdjustment: "MEMBER_LOANS_ADJUSTMENT",
  NonMemberLoansAdjustment: "NON_MEMBER_LOANS_ADJUSTMENT",
  VoluntarySavingsAdjustment: "VOLUNTARY_SAVINGS_ADJUSTMENT",
  // Pos added by hand to a group, beside its fixed lines (a list each).
  ExtraCurrentAssets: "EXTRA_CURRENT_ASSETS",
  ExtraCurrentLiabilities: "EXTRA_CURRENT_LIABILITIES",
  ExtraLongTermLiabilities: "EXTRA_LONG_TERM_LIABILITIES",
  ExtraEquity: "EXTRA_EQUITY",
} as const;

export type BalanceSheetItem =
  (typeof BalanceSheetItem)[keyof typeof BalanceSheetItem];

/** The typed Biaya of a book year whose transactions are not in the app. */
export const typedExpenseKey = (category: CashCategory) =>
  `EXPENSE_${category}`;

/** Key of the bank balance typed for the end of `period` ("YYYY-MM"):
 * `BANK_BALANCE` at 31 Desember (also the opening of the next year), and
 * `BANK_BALANCE_MM` at the end of the other months. */
export const bankBalanceKey = (period: string) => {
  const month = period.slice(5, 7);

  return (
    month === "12"
      ? BalanceSheetItem.BankBalance
      : `${BalanceSheetItem.BankBalance}_${month}`
  ) as BalanceSheetItem;
};

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
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = hutang baru di luar Buku Kas. Bertambah dengan kategori Kas Pinjaman Modal KOWABRI / PP, berkurang dengan Cicil Hutang",
  },
  {
    key: BalanceSheetItem.NonMemberSavings,
    label: "Simpanan Non Anggota",
    hint: "Manasuka penabung non anggota (bagian bawah halaman Simpanan), dihitung otomatis",
  },
  {
    key: BalanceSheetItem.CapitalReserve,
    label: "Cadangan Modal",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU)",
  },
  {
    key: BalanceSheetItem.EducationFund,
    label: "Dana Pendidikan",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU). Berkurang dengan kategori Kas Pakai Dana Pendidikan",
  },
  {
    key: BalanceSheetItem.SocialFund,
    label: "Dana Sosial",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU). Berkurang dengan kategori Kas Dansos",
  },
  {
    key: BalanceSheetItem.PmdkFund,
    label: "Dana PMDK (Pembangunan Daerah Kerja)",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU). Berkurang dengan kategori Kas Pakai Dana PMDK dan Dekopinda",
  },
  {
    key: BalanceSheetItem.LongTermDebt,
    label: "Kewajiban jangka panjang",
  },
  {
    key: BalanceSheetItem.RatReserve,
    label: "Cadangan RAT",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU). Berkurang dengan kategori Kas Pakai Cadangan RAT",
  },
  {
    key: BalanceSheetItem.LoanWriteOffReserve,
    label: "Cadangan Penghapusan",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (bagian SHU)",
  },
  {
    key: BalanceSheetItem.RetainedEarnings,
    label: "Laba ditahan tahun lalu",
    hint: "Saldo 1 Januari dari Neraca 31 Desember tahun lalu. Isian = tambahan saat RAT (laba tahun lalu yang ditahan). Berkurang dengan kategori Kas Pakai Laba Ditahan",
  },
  ...(
    [
      [BalanceSheetItem.MemberLoansAdjustment, "Penyesuaian Piutang Anggota"],
      [
        BalanceSheetItem.NonMemberLoansAdjustment,
        "Penyesuaian Piutang Non Anggota",
      ],
      [
        BalanceSheetItem.VoluntarySavingsAdjustment,
        "Penyesuaian Simpanan Manasuka",
      ],
    ] as const
  ).map(([key, label]) => ({
    key,
    label,
    hint: "Ditambahkan ke angka dari pinjaman/simpanan; hanya untuk tahun yang transaksinya tidak ada di aplikasi (2025)",
  })),
  {
    key: BalanceSheetItem.NetIncome,
    label: "Laba tahun (isian)",
    hint: "Hanya untuk tahun yang transaksinya tidak lengkap di aplikasi (2025). Kosong: Bunga + Provisi − Biaya",
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

/** Biaya columns of Buku Kas (L–W), subtracted from income for Laba Bersih.
 * Dansos and Dekopinda are not: as in Neraca Penyesuaian, they are paid from
 * Dana Sosial and Dana PMDK (see `fundUses`). */
export const expenseCategories = cashCategories.filter(
  (category) =>
    ![
      CashCategory.CashTopUp,
      CashCategory.CashTopUpFromSavingsBook,
      CashCategory.TransferToSavingsBook,
      CashCategory.SocialFund,
      CashCategory.Dekopinda,
    ].includes(category.value as never),
);

/** Neraca lines carried from the Neraca of last year: their 1 January
 * balance is every earlier book year's typed amount and Buku Kas movement
 * (`prior`), and the typed amount of the year is what the RAT adds (bagian
 * SHU). The first book year's typed amount is its whole balance. */
export const carriedItems: readonly BalanceSheetItem[] = [
  BalanceSheetItem.DebtPpbri,
  BalanceSheetItem.CapitalReserve,
  BalanceSheetItem.EducationFund,
  BalanceSheetItem.SocialFund,
  BalanceSheetItem.PmdkFund,
  BalanceSheetItem.RatReserve,
  BalanceSheetItem.LoanWriteOffReserve,
  BalanceSheetItem.RetainedEarnings,
];

/** Earlier book years added up (`get_balance_sheet`): typed amounts by key
 * and Buku Kas components by type. */
export type BalanceSheetPrior = {
  items: Record<string, number>;
  categories: Record<string, number>;
};

/** Neraca lines typed as their balance on 1 January (`balance_sheet_items`)
 * that the Buku Kas categories move during the year: the typed balance plus
 * `adds` less `uses` (Neraca Penyesuaian: saldo, then each pemakaian). */
export const fundUses: Partial<
  Record<BalanceSheetItem, { adds?: CashCategory[]; uses: CashCategory[] }>
> = {
  [BalanceSheetItem.DebtPpbri]: {
    adds: [CashCategory.DebtReceived],
    uses: [CashCategory.DebtRepayment],
  },
  [BalanceSheetItem.EducationFund]: { uses: [CashCategory.EducationFundUse] },
  [BalanceSheetItem.SocialFund]: { uses: [CashCategory.SocialFund] },
  [BalanceSheetItem.PmdkFund]: {
    uses: [CashCategory.PmdkFundUse, CashCategory.Dekopinda],
  },
  [BalanceSheetItem.RatReserve]: { uses: [CashCategory.RatReserveUse] },
  [BalanceSheetItem.RetainedEarnings]: {
    uses: [CashCategory.RetainedEarningsUse],
  },
};

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
 * that book year. Bank is the balance typed for the month end, and otherwise
 * Kas Buku Tabungan, which misses money moved only at the bank. */
export function balanceSheetAt(
  period: string,
  ledger: MonthlyLedger,
  cashBook: CashBook | undefined,
  loans: Loan[],
  items: Record<string, number>,
  details: Record<string, BalanceSheetDetail[]> = {},
  prior: BalanceSheetPrior = { items: {}, categories: {} },
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
  const monthRows = ledger.savings.filter((row) => row.period === period);
  // Simpanan Pokok, Wajib, Manasuka are the members'; penabung Non Anggota
  // are Simpanan Non Anggota.
  const savingsRows = monthRows.filter((row) => !row.nonMember);
  const nonMemberSavings = sum(
    monthRows.filter((row) => row.nonMember).map((row) => row.voluntaryClosing),
  );

  const loanBalance = (group: LoanGroup) =>
    sum(
      loanRows
        .filter((row) => groupOf.get(row.loanId) === group)
        .map((row) => row.closingBalance),
    );

  // A derived line plus its typed adjustment (only typed for 2025), which is
  // edited in place like Kas once it is set.
  const adjusted = (
    id: string,
    label: string,
    amount: number,
    key: BalanceSheetItem,
  ): BalanceSheetLine => ({
    id,
    label,
    amount: amount + item(key),
    edit: items[key]
      ? { kind: LineEditKind.Item, key, offset: amount }
      : undefined,
  });

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

  // The bank balance typed for the end of this month. 0 means not typed: a
  // bank balance of exactly 0 is not expected.
  const bankKey = bankBalanceKey(period);
  const typedBank = items[bankKey] || undefined;

  // Laba Bersih: Bunga + Provisi from January, less the Buku Kas expenses.
  const monthsSoFar = ledger.loans.filter((row) => row.period <= period);
  const countedInterest = sum(monthsSoFar.map((row) => row.interestPaid));
  const countedProvision = sum(monthsSoFar.map((row) => row.provision));

  const countedExpenses = expenseCategories.map((category) => ({
    category: category.value,
    label: category.label,
    amount: sum(
      postedRows.flatMap((row) =>
        row.components
          .filter((part) => part.componentType === category.value)
          .map((part) => part.amount),
      ),
    ),
  }));

  // A year without its income in the app (2025) takes the LABA RUGI typed
  // from the sheet, where there is one.
  const typedIncome =
    countedInterest === 0 &&
    countedProvision === 0 &&
    countedExpenses.every((expense) => expense.amount === 0) &&
    Object.keys(items).some(
      (key) => key.startsWith("INCOME_") || key.startsWith("EXPENSE_"),
    );

  const interest = typedIncome
    ? (items[BalanceSheetItem.IncomeInterest] ?? 0)
    : countedInterest;
  const provision = typedIncome
    ? (items[BalanceSheetItem.IncomeProvision] ?? 0)
    : countedProvision;
  const expenses = typedIncome
    ? countedExpenses.map((expense) => ({
        ...expense,
        amount: items[typedExpenseKey(expense.category)] ?? 0,
      }))
    : countedExpenses;

  const expenseTotal = sum(expenses.map((expense) => expense.amount));

  // Buku Kas total of categories from January to `period`.
  const categoryTotal = (categories: readonly CashCategory[]) =>
    sum(
      postedRows.flatMap((row) =>
        row.components
          .filter((part) =>
            categories.includes(part.componentType as CashCategory),
          )
          .map((part) => part.amount),
      ),
    );

  const cashBondLent = categoryTotal([CashCategory.CashBond]);
  const cashBondRepaid = categoryTotal([CashCategory.CashBondRepayment]);
  const netIncome = interest + provision - expenseTotal;
  // 0 means not typed, as for Bank.
  const typedNetIncome = items[BalanceSheetItem.NetIncome] || undefined;

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

  // A typed line; one in `carriedItems` starts from the Neraca of last year,
  // one in `fundUses` moves with the Buku Kas categories, and typing it saves
  // the amount that gives it.
  const typed = (
    key: BalanceSheetItem,
    label?: string,
    editable = true,
  ): BalanceSheetLine => {
    const rule = fundUses[key];
    const priorTotal = (categories: readonly CashCategory[]) =>
      sum(categories.map((category) => prior.categories[category] ?? 0));
    const opening = carriedItems.includes(key)
      ? (prior.items[key] ?? 0) +
        (rule ? priorTotal(rule.adds ?? []) - priorTotal(rule.uses) : 0)
      : 0;
    const movement =
      opening +
      (rule ? categoryTotal(rule.adds ?? []) - categoryTotal(rule.uses) : 0);

    return {
      id: key,
      label:
        label ?? balanceSheetItems.find((entry) => entry.key === key)!.label,
      amount: item(key) + movement,
      edit: editable
        ? movement
          ? { kind: LineEditKind.Item, key, offset: movement }
          : { kind: LineEditKind.Item, key }
        : undefined,
    };
  };

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
        // Typed per month: the saldo of the buku tabungan at the month end.
        edit: { kind: LineEditKind.Item, key: bankKey },
      },
      adjusted(
        "member-loans",
        "Piutang Anggota",
        loanBalance(LoanGroup.Member),
        BalanceSheetItem.MemberLoansAdjustment,
      ),
      adjusted(
        "non-member-loans",
        "Piutang Non Anggota",
        loanBalance(LoanGroup.NonMember),
        BalanceSheetItem.NonMemberLoansAdjustment,
      ),
      // The total of its list (Bon Sementara on 1 January), edited under
      // Catatan, with this year's Kas Bon and Bayar Kas Bon of Buku Kas.
      {
        id: BalanceSheetItem.OtherLoans,
        label: balanceSheetItems.find(
          (entry) => entry.key === BalanceSheetItem.OtherLoans,
        )!.label,
        amount:
          sum(
            (details[BalanceSheetItem.OtherLoans] ?? []).map(
              (detail) => detail.amount,
            ),
          ) +
          cashBondLent -
          cashBondRepaid,
      },
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
        // Members only: Simpanan Non Anggota is its own line below.
        adjusted(
          "voluntary-savings",
          "Simpanan Manasuka",
          sum(savingsRows.map((row) => row.voluntaryClosing)),
          BalanceSheetItem.VoluntarySavingsAdjustment,
        ),
        typed(BalanceSheetItem.DebtPpbri),
        {
          id: BalanceSheetItem.NonMemberSavings,
          label: "Simpanan Non Anggota",
          amount: nonMemberSavings,
        },
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
        typedNetIncome === undefined
          ? { id: "net-income", label: `Laba Tahun ${year}`, amount: netIncome }
          : {
              id: "net-income",
              label: `Laba Tahun ${year}`,
              amount: typedNetIncome,
              edit: {
                kind: LineEditKind.Item,
                key: BalanceSheetItem.NetIncome,
              },
            },
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
    income: {
      interest,
      provision,
      expenses,
      expenseTotal,
      netIncome,
      /** The typed Laba of the year (NET_INCOME), shown on the Neraca. */
      typedNetIncome,
    },
    /** Kas Bon and Bayar Kas Bon of Buku Kas this year (Piutang Lainnya). */
    nonMemberByRecommender,
    cashBonds: { lent: cashBondLent, repaid: cashBondRepaid },
  };
}

export type BalanceSheet = ReturnType<typeof balanceSheetAt>;

/** The ledger of `period` (31 Desember of last year) when last year has no
 * transactions in the app (2025 was removed, its balances kept as the
 * openings of 1 January): each loan and member at the opening of `ledger`'s
 * first month, with no income. */
export function openingLedger(
  ledger: MonthlyLedger,
  period: string,
): MonthlyLedger {
  const first = [...ledger.periods].sort()[0];

  return {
    ...ledger,
    periods: [period],
    loans: ledger.loans
      .filter((row) => row.period === first)
      .map((row) => ({
        ...row,
        period,
        disbursed: 0,
        principalPaid: 0,
        interestPaid: 0,
        provision: 0,
        closingBalance: row.openingBalance,
      })),
    savings: ledger.savings
      .filter((row) => row.period === first)
      .map((row) => ({
        ...row,
        period,
        principalClosing: row.principalOpening,
        mandatoryClosing: row.mandatoryOpening,
        voluntaryClosing: row.voluntaryOpening,
      })),
  };
}

/** The ledger of 31 Desember of last year (`period`): last year's own, or,
 * when last year has no transactions in the app (2025 was removed), this
 * year's openings (`openingLedger`). */
export function previousYearLedger(
  previous: MonthlyLedger,
  ledger: MonthlyLedger,
  period: string,
): MonthlyLedger {
  const hasRows =
    previous.loans.some((row) => row.period === period) ||
    previous.savings.some((row) => row.period === period);

  return hasRows ? previous : openingLedger(ledger, period);
}

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
