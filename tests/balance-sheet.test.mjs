// Run with `bun test tests/balance-sheet.test.mjs` (the store imports need Bun's resolver).
import assert from "node:assert/strict";
import { test } from "node:test";

const { balanceSheetAt, openingLedger } =
  await import("../src/balance-sheet.ts");

const row = (businessDate, componentType, amount) => ({
  businessDate,
  balance: 0,
  status: "Terposting",
  isReversal: false,
  savingsBookIn: 0,
  savingsBookOut: 0,
  components: [{ componentType, amount }],
});

const ledger = {
  periods: ["2026-01", "2026-02"],
  loans: [],
  savings: [
    {
      memberId: "A",
      period: "2026-02",
      principalClosing: 50_000,
      mandatoryClosing: 100_000,
      voluntaryClosing: 900_000,
    },
    // A penabung Non Anggota: Simpanan Non Anggota, not Manasuka.
    {
      memberId: "SA-NA-1",
      nonMember: true,
      period: "2026-02",
      principalClosing: 0,
      mandatoryClosing: 0,
      voluntaryClosing: 100_000,
    },
  ],
};

const cashBook = {
  openingBalance: 0,
  savingsBookOpeningBalance: 0,
  rows: [
    row("2026-01-19", "PAKAI_CADANGAN_RAT", 18_730_000),
    row("2026-01-26", "DANSOS", 120_000),
    row("2026-01-26", "DEKOPINDA", 850_000),
    row("2026-02-04", "PAKAI_DANA_PMDK", 1_100_000),
    row("2026-02-04", "PAKAI_DANA_PENDIDIKAN", 845_450),
    row("2026-02-05", "PAKAI_LABA_DITAHAN", 1_000_000),
    row("2026-02-06", "BAYAR_KAS_BON", 700_000),
    row("2026-02-06", "KAS_BON", 2_000_000),
    row("2026-02-07", "TERIMA_HUTANG", 5_000_000),
    row("2026-02-08", "CICIL_HUTANG", 15_000_000),
    row("2026-02-09", "BIAYA_PENGURUS", 3_400_000),
    row("2026-02-09", "BAGI_SHU", 40_583_458),
    // March is after the period shown.
    row("2026-03-02", "PAKAI_DANA_PENDIDIKAN", 1_000),
  ],
};

const items = {
  DEBT_PPBRI: 10_000_000,
  EDUCATION_FUND: 6_460_622,
  SOCIAL_FUND: 4_232_995,
  PMDK_FUND: 6_494_387,
  RAT_RESERVE: 20_000_000,
  RETAINED_EARNINGS: 7_591_369,
};

const details = {
  OTHER_LOANS: [{ name: "X", amount: 28_109_000 }],
};

const lineOf = (sheet, id) =>
  [...sheet.assets, ...sheet.liabilities]
    .flatMap((part) => part.lines)
    .find((line) => line.id === id);

test("Neraca lines move with the Buku Kas Neraca categories", () => {
  const sheet = balanceSheetAt("2026-02", ledger, cashBook, [], items, details);
  const amount = (id) => lineOf(sheet, id).amount;

  assert.equal(amount("DEBT_PPBRI"), 0);
  assert.equal(amount("EDUCATION_FUND"), 6_460_622 - 845_450);
  assert.equal(amount("SOCIAL_FUND"), 4_232_995 - 120_000);
  assert.equal(amount("PMDK_FUND"), 6_494_387 - 850_000 - 1_100_000);
  assert.equal(amount("RAT_RESERVE"), 1_270_000);
  assert.equal(amount("RETAINED_EARNINGS"), 6_591_369);
  assert.equal(amount("OTHER_LOANS"), 28_109_000 + 2_000_000 - 700_000);
  assert.equal(amount("voluntary-savings"), 900_000);
  assert.equal(amount("NON_MEMBER_SAVINGS"), 100_000);
  assert.equal(sheet.cashBonds.lent, 2_000_000);
  assert.equal(sheet.cashBonds.repaid, 700_000);
});

test("Only expense categories count for Laba Bersih", () => {
  const sheet = balanceSheetAt("2026-02", ledger, cashBook, [], items, details);

  assert.equal(sheet.income.expenseTotal, 3_400_000);
  assert.equal(sheet.income.netIncome, -3_400_000);
});

test("Typing a moved line saves its 1 January balance", () => {
  const sheet = balanceSheetAt("2026-02", ledger, cashBook, [], items, details);

  assert.deepEqual(lineOf(sheet, "EDUCATION_FUND").edit, {
    kind: "item",
    key: "EDUCATION_FUND",
    offset: -845_450,
  });
  assert.deepEqual(lineOf(sheet, "CAPITAL_RESERVE").edit, {
    kind: "item",
    key: "CAPITAL_RESERVE",
  });
});

test("Last year without transactions takes this year's openings, plus its adjustments", () => {
  const thisYear = {
    periods: ["2026-01"],
    loans: [
      {
        loanId: "L1",
        period: "2026-01",
        openingBalance: 762_512_637,
        disbursed: 0,
        principalPaid: 1_000_000,
        interestPaid: 200_000,
        provision: 0,
        closingBalance: 761_512_637,
      },
    ],
    savings: [
      {
        memberId: "A",
        period: "2026-01",
        principalOpening: 50_000,
        mandatoryOpening: 100_000,
        voluntaryOpening: 2_000_000,
        principalClosing: 50_000,
        mandatoryClosing: 150_000,
        voluntaryClosing: 2_500_000,
      },
      {
        memberId: "SA-NA-1",
        nonMember: true,
        period: "2026-01",
        principalOpening: 0,
        mandatoryOpening: 0,
        voluntaryOpening: 876_000,
        principalClosing: 0,
        mandatoryClosing: 0,
        voluntaryClosing: 900_000,
      },
    ],
  };
  const loans = [{ id: "L1", loanGroup: "Anggota PP BRI", recommender: "" }];
  const previous = openingLedger(thisYear, "2025-12");
  const sheet = balanceSheetAt(
    "2025-12",
    previous,
    { openingBalance: 0, savingsBookOpeningBalance: 0, rows: [] },
    loans,
    {
      MEMBER_LOANS_ADJUSTMENT: 600,
      VOLUNTARY_SAVINGS_ADJUSTMENT: -1_000,
      NET_INCOME: 75_913_690,
    },
  );

  assert.equal(lineOf(sheet, "member-loans").amount, 762_513_237);
  assert.deepEqual(lineOf(sheet, "member-loans").edit, {
    kind: "item",
    key: "MEMBER_LOANS_ADJUSTMENT",
    offset: 762_512_637,
  });
  assert.equal(lineOf(sheet, "voluntary-savings").amount, 1_999_000);
  assert.equal(lineOf(sheet, "NON_MEMBER_SAVINGS").amount, 876_000);
  assert.equal(lineOf(sheet, "NON_MEMBER_SAVINGS").edit, undefined);
  assert.equal(lineOf(sheet, "mandatory-savings").amount, 100_000);
  assert.equal(lineOf(sheet, "net-income").amount, 75_913_690);
  assert.equal(sheet.income.interest, 0);
  // Without an adjustment the derived line is not typed.
  assert.equal(lineOf(sheet, "non-member-loans").edit, undefined);
});

test("A year without its income in the app takes the typed LABA RUGI", () => {
  const empty = { periods: ["2025-12"], loans: [], savings: [] };
  const book = { openingBalance: 0, savingsBookOpeningBalance: 0, rows: [] };
  const sheet = balanceSheetAt("2025-12", empty, book, [], {
    NET_INCOME: 75_913_690,
    INCOME_INTEREST: 147_325_870,
    INCOME_PROVISION: 7_390_960,
    EXPENSE_BIAYA_PENGURUS: 44_960_000,
    EXPENSE_BIAYA_RAT: 33_843_140,
  });

  assert.equal(sheet.income.interest, 147_325_870);
  assert.equal(sheet.income.provision, 7_390_960);
  assert.equal(sheet.income.expenseTotal, 78_803_140);
  assert.equal(sheet.income.netIncome, 75_913_690);
  assert.equal(
    sheet.income.expenses.find((e) => e.category === "BIAYA_RAT").amount,
    33_843_140,
  );
  assert.equal(lineOf(sheet, "net-income").amount, 75_913_690);

  // Without the typed rincian, only the Laba is known.
  const bare = balanceSheetAt("2025-12", empty, book, [], {
    NET_INCOME: 75_913_690,
  });
  assert.equal(bare.income.interest, 0);
});

test("Carried lines open from the Neraca of last year, plus what the RAT adds", () => {
  // 2025 typed as balances, 2026 typed as additions (data script 046).
  const prior = {
    items: { ...items, RETAINED_EARNINGS: 0, CAPITAL_RESERVE: 42_991_150 },
    categories: { DANSOS: 32_995 },
  };
  const additions = { RETAINED_EARNINGS: 7_591_369, CAPITAL_RESERVE: 500_000 };
  const sheet = balanceSheetAt(
    "2026-02",
    ledger,
    cashBook,
    [],
    additions,
    details,
    prior,
  );

  assert.equal(
    lineOf(sheet, "SOCIAL_FUND").amount,
    4_232_995 - 32_995 - 120_000,
  );
  assert.equal(lineOf(sheet, "RETAINED_EARNINGS").amount, 6_591_369);
  assert.equal(lineOf(sheet, "CAPITAL_RESERVE").amount, 43_491_150);
  // Typing the line saves what the RAT adds this year.
  assert.deepEqual(lineOf(sheet, "CAPITAL_RESERVE").edit, {
    kind: "item",
    key: "CAPITAL_RESERVE",
    offset: 42_991_150,
  });
  // Not carried: typed per year.
  assert.equal(lineOf(sheet, "LONG_TERM_DEBT").amount, 0);
});

test("Bank takes the saldo typed for the month end, else Kas Buku Tabungan", () => {
  const bankBook = { ...cashBook, savingsBookOpeningBalance: 11_698_551 };
  const bank = (period, typed) =>
    lineOf(
      balanceSheetAt(period, ledger, bankBook, [], { ...items, ...typed }),
      "bank",
    );

  const { parts, ...estimated } = bank("2026-02", {});
  assert.deepEqual(
    parts.map((part) => part.amount),
    [11_698_551, 0, 0],
  );
  assert.deepEqual(estimated, {
    id: "bank",
    label: "Bank",
    amount: 11_698_551,
    estimated: true,
    edit: { kind: "item", key: "BANK_BALANCE_02" },
  });
  assert.equal(
    bank("2026-02", { BANK_BALANCE_02: 20_587_928 }).amount,
    20_587_928,
  );
  assert.equal(
    bank("2026-02", { BANK_BALANCE_02: 20_587_928 }).estimated,
    undefined,
  );
  // Another month's saldo is not this month's.
  assert.equal(
    bank("2026-01", { BANK_BALANCE_02: 20_587_928 }).estimated,
    true,
  );
  // December is BANK_BALANCE, also the opening of the next year.
  assert.equal(
    bank("2026-12", { BANK_BALANCE: 30_000_000 }).amount,
    30_000_000,
  );
  assert.deepEqual(bank("2026-12", {}).edit, {
    kind: "item",
    key: "BANK_BALANCE",
  });
});

test("Rincian: the parts of every line add up to the line", () => {
  const prior = {
    items: { RAT_RESERVE: 20_000_000, CAPITAL_RESERVE: 36_227_240 },
    categories: { PAKAI_CADANGAN_RAT: 18_730_000 },
  };
  const typedItems = { ...items, CASH_ADJUSTMENT: 5_000, BANK_BALANCE_02: 0 };

  for (const sheet of [
    balanceSheetAt("2026-02", ledger, cashBook, [], typedItems, details, prior),
    balanceSheetAt(
      "2026-02",
      ledger,
      cashBook,
      [],
      { ...items, BANK_BALANCE_02: 1_500 },
      details,
    ),
  ]) {
    for (const line of [...sheet.assets, ...sheet.liabilities].flatMap(
      (part) => part.lines,
    )) {
      assert.ok(line.parts, `${line.id} has no parts`);
      assert.equal(
        line.parts.reduce((total, part) => total + part.amount, 0),
        line.amount,
        line.id,
      );
    }
  }

  const sheet = balanceSheetAt("2026-02", ledger, cashBook, [], items, details);
  assert.deepEqual(
    lineOf(sheet, "PMDK_FUND").parts.map((part) => [part.date, part.amount]),
    [
      [undefined, 6_494_387],
      ["2026-01-26", -850_000],
      ["2026-02-04", -1_100_000],
    ],
  );
});

test("A bon is Lunas once its payments reach its amount, and leaves the Catatan", async () => {
  const { bonStatuses, openOtherLoans, otherLoansPerPerson } =
    await import("../src/balance-sheet.ts");

  const bon = (name, amount, bonKey, loanDate) => ({
    name,
    amount,
    loanDate,
    note: "",
    bonKey,
  });
  const details = [
    bon("A", 3_900_000, "BON-1", "2024-01-30"),
    bon("A", 1_000_000, "BON-2", "2024-07-02"),
    bon("B", 1_000_000, "BON-3", "2023-07-06"),
    // A Pengurang names no bon: it only lowers B.
    bon("B", -391_000, null, null),
    bon("C", 300_000, "BON-4", "2025-01-13"),
  ];
  const pay = (businessDate, amount, bonPayments) => ({
    ...row(businessDate, "BAYAR_KAS_BON", amount),
    description: "Bayar bon",
    bonPayments,
  });
  const rows = [
    // One payment for two bons of A: all of one, part of the other.
    pay("2026-03-29", 4_000_000, [
      { bonKey: "BON-1", amount: 3_900_000 },
      { bonKey: "BON-2", amount: 100_000 },
    ]),
    // Paid in two parts.
    pay("2026-04-01", 100_000, [{ bonKey: "BON-4", amount: 100_000 }]),
    pay("2026-05-01", 200_000, [{ bonKey: "BON-4", amount: 200_000 }]),
    // No bon named.
    pay("2026-06-01", 50_000, []),
  ];

  const { statuses, unlinked, unapplied } = bonStatuses(details, rows);
  assert.equal(statuses.get("BON-1").left, 0);
  assert.equal(statuses.get("BON-1").paidOffOn, "2026-03-29");
  assert.equal(statuses.get("BON-2").left, 900_000);
  assert.equal(statuses.get("BON-3").left, 1_000_000);
  assert.equal(statuses.get("BON-3").payments.length, 0);
  assert.equal(statuses.get("BON-4").left, 0);
  assert.equal(statuses.get("BON-4").payments.length, 2);
  assert.equal(statuses.get("BON-4").paidOffOn, "2026-05-01");
  assert.deepEqual([...unapplied.values()], [-391_000]);
  assert.deepEqual(
    unlinked.map((part) => part.amount),
    [-50_000],
  );

  // C is Lunas and drops out; the totals still add up to Piutang Lainnya.
  const perPerson = otherLoansPerPerson(details, rows);
  assert.deepEqual(perPerson.byName, [
    ["A", 900_000],
    ["B", 609_000],
  ]);
  const total = (amounts) => amounts.reduce((sum, amount) => sum + amount, 0);
  assert.equal(
    total(perPerson.byName.map(([, amount]) => amount)) +
      total(unlinked.map((part) => part.amount)),
    total(details.map((detail) => detail.amount)) -
      total(rows.map((entry) => entry.components[0].amount)),
  );

  assert.deepEqual(
    openOtherLoans(details, statuses, unapplied).map((detail) => [
      detail.name,
      detail.amount,
    ]),
    [
      ["A", 900_000],
      ["B", 1_000_000],
      ["B", -391_000],
    ],
  );
});
