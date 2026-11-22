// Run with `bun test tests/income-statement.test.mjs` (the store imports need Bun's resolver).
import assert from "node:assert/strict";
import { test } from "node:test";

const { incomeStatement } = await import("../src/income-statement.ts");
const { incomeStatementSheet } =
  await import("../src/income-statement-export.ts");

const expense = (category, amount) => ({ category, label: category, amount });

// 31 Desember 2025: no transactions in the app, only the typed Laba.
const typedYear = {
  income: {
    interest: 0,
    provision: 0,
    expenses: [expense("BIAYA_PENGURUS", 0)],
    expenseTotal: 0,
    netIncome: 0,
    typedNetIncome: 75_913_690,
  },
};

// 30 September 2026 as in LABA RUGI of the sheet (Biaya RAT kept apart).
const expenses = [
  expense("BIAYA_PENGURUS", 30_600_000),
  expense("BIAYA_ATK", 364_700),
  expense("TRANSFORTASI", 100_000),
  expense("LAINNYA", 261_900),
  expense("BIAYA_BUNGA", 250_000),
  expense("HUMAS", 2_238_500),
  expense("PEMELIHARAAN_AT", 766_000),
  expense("PARCEL", 0),
];
const expenseTotal = expenses.reduce((total, e) => total + e.amount, 0);
const year = {
  income: {
    interest: 109_883_730,
    provision: 5_097_125,
    expenses,
    expenseTotal,
    netIncome: 109_883_730 + 5_097_125 - expenseTotal,
    typedNetIncome: undefined,
  },
};

test("Laba Rugi follows the sheet's lines", () => {
  const s = incomeStatement([typedYear, year]);

  assert.deepEqual(s.typedOnly, [true, false]);
  assert.deepEqual(s.profit, [75_913_690, 80_399_755]);
  assert.deepEqual(s.revenue[0].amounts, [null, 109_883_730]);
  assert.deepEqual(s.total, [null, 114_980_855]);
  assert.deepEqual(s.expenseTotal, [null, 34_581_100]);
  assert.equal(s.expenses.length, 10, "Parcel only when used");
  assert.deepEqual(
    s.expenses.map((line) => line.label.replace(/^\d+\. /, "")),
    [
      "Biaya Honor Pengurus",
      "Biaya ATK",
      "Biaya Transfortasi",
      "Biaya Lainnya",
      "Biaya Hutang",
      "Sewa Kantor",
      "Biaya Penghapusan Pinjaman",
      "Biaya RAT",
      "Biaya Humas dan Lainnya",
      "Pemeliharaan AT",
    ],
  );
  assert.deepEqual(s.expenses[8].amounts, [null, 2_238_500]);
  assert.deepEqual(s.typedProfit, [undefined, undefined]);
});

test("Parcel gets a line when used; a differing typed Laba is flagged", () => {
  const withParcel = {
    income: {
      ...year.income,
      expenses: [...expenses.slice(0, -1), expense("PARCEL", 500_000)],
      expenseTotal: expenseTotal + 500_000,
      netIncome: year.income.netIncome - 500_000,
      typedNetIncome: 1,
    },
  };
  const s = incomeStatement([withParcel, withParcel]);

  assert.equal(s.expenses[10].label, "11. Biaya Parcel");
  assert.deepEqual(s.expenses[10].amounts, [500_000, 500_000]);
  assert.deepEqual(s.typedProfit, [1, 1]);
});

test("the export adds up with formulas", () => {
  const spec = incomeStatementSheet({
    company: "K",
    position: "30 SEPTEMBER 2026",
    periods: ["2025-12", "2026-09"],
    statement: incomeStatement([typedYear, year]),
    fileName: "x.xlsx",
  });
  const at = (row, col) =>
    spec.cells.find((cell) => cell.row === row && cell.col === col);

  // Biaya lines in rows 8–17, TOTAL BIAYA 18, LABA 20, TOTAL 22.
  assert.equal(at(18, 4).text, "TOTAL BIAYA");
  assert.equal(at(18, 6).formula, "SUM(F8:F17)");
  assert.equal(at(20, 6).formula, "SUM(C8:C10)-F18");
  assert.equal(at(20, 6).number, 80_399_755);
  assert.equal(at(20, 5).number, 75_913_690);
  assert.equal(at(22, 3).number, 114_980_855);
  assert.equal(at(22, 6).formula, "F18+F20");
  assert.equal(at(8, 5).number, undefined, "2025 rincian left blank");
});

test("Rincian: Bunga and Provisi per month, each Biaya per Buku Kas row", async () => {
  const { incomeBreakdown } = await import("../src/income-statement.ts");
  const { balanceSheetAt } = await import("../src/balance-sheet.ts");
  const row = (businessDate, componentType, amount) => ({
    businessDate,
    description: componentType,
    balance: 0,
    status: "Terposting",
    isReversal: false,
    savingsBookIn: 0,
    savingsBookOut: 0,
    components: [{ componentType, amount }],
  });
  const loan = (period, interestPaid, provision) => ({
    loanId: "L1",
    period,
    interestPaid,
    provision,
    closingBalance: 0,
  });
  const ledger = {
    periods: ["2026-01", "2026-02", "2026-03"],
    loans: [
      loan("2026-01", 1_000, 50),
      loan("2026-02", 2_000, 0),
      loan("2026-03", 4_000, 70),
    ],
    savings: [],
  };
  const cashBook = {
    openingBalance: 0,
    savingsBookOpeningBalance: 0,
    rows: [
      row("2026-01-05", "BIAYA_PENGURUS", 300),
      row("2026-02-05", "BIAYA_PENGURUS", 300),
      row("2026-02-06", "HUMAS", 125),
      row("2026-03-07", "HUMAS", 999),
    ],
  };

  const b = incomeBreakdown(
    balanceSheetAt("2026-02", ledger, cashBook, [], {}, {}),
  );

  assert.deepEqual(
    b.revenue[0].parts.map((part) => part.amount),
    [1_000, 2_000],
  );
  assert.deepEqual(
    b.revenue[1].parts.map((part) => part.amount),
    [50],
  );
  for (const line of [...b.revenue, ...b.expenses]) {
    assert.equal(
      line.parts.reduce((total, part) => total + part.amount, 0),
      line.amount,
      line.label,
    );
  }
  const humas = b.expenses.find(
    (line) => line.label === "Biaya Humas dan Lainnya",
  );
  assert.deepEqual(humas.parts, [
    { label: "HUMAS", amount: 125, date: "2026-02-06" },
  ]);
  assert.equal(b.profit, 3_050 - 725);
});

test("Rincian: a year with only its Laba typed shows that Laba", async () => {
  const { incomeBreakdown } = await import("../src/income-statement.ts");
  const { balanceSheetAt } = await import("../src/balance-sheet.ts");
  const ledger = { periods: ["2025-12"], loans: [], savings: [] };

  const b = incomeBreakdown(
    balanceSheetAt("2025-12", ledger, undefined, [], { NET_INCOME: 75_000 }),
  );

  assert.equal(b.revenueTotal, 0);
  assert.equal(b.expenseTotal, 0);
  assert.equal(b.profit, 75_000);
});
