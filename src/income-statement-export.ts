// The Laba Rugi page as LABA RUGI of BUKU KAS HARIAN: Pendapatan in A–C and
// Biaya in D–F, a column per date. TOTAL BIAYA sums its lines, LABA is
// Pendapatan less Biaya, and both TOTAL rows add up, as in the sheet.
import { lastDate } from "./balance-sheet-export";
import type { IncomeAmounts, IncomeStatement } from "./income-statement";
import { SheetBuilder, cellRef, type SheetSpec } from "./xlsx-sheet";

const FIRST_LINE = 8;
const LAST_COL = 6;
const left = { label: 1, amounts: [2, 3] } as const;
const right = { label: 4, amounts: [5, 6] } as const;

export function incomeStatementSheet(input: {
  company: string;
  /** "30 SEPTEMBER 2026". */
  position: string;
  /** `YYYY-MM` of each amount column. */
  periods: [string, string];
  statement: IncomeStatement;
  fileName: string;
}): SheetSpec {
  const { statement } = input;
  const sheet = new SheetBuilder();

  [input.company, "LABA - RUGI", `POSISI ${input.position}`].forEach(
    (text, i) => {
      sheet.text(i + 1, 1, text, "title");
      sheet.merge(i + 1, 1, i + 1, LAST_COL);
    },
  );

  for (const side of [left, right]) {
    sheet.header(5, side.label, 6, side.label, "URAIAN");
    sheet.header(5, side.amounts[0], 5, side.amounts[1], "TAHUN");
    side.amounts.forEach((col, index) =>
      sheet.date(6, col, lastDate(input.periods[index])),
    );
  }

  sheet.text(7, left.label, "PENDAPATAN", "totalText");
  sheet.text(7, right.label, "BIAYA", "totalText");

  const amount = (
    row: number,
    col: number,
    value: number | null,
    style: "number" | "totalNumber" = "number",
  ) =>
    value === null
      ? sheet.blank(row, col, style)
      : sheet.number(row, col, value, style);

  const lines = (
    side: typeof left | typeof right,
    entries: { label: string; amounts: IncomeAmounts }[],
  ) =>
    entries.forEach((line, index) => {
      sheet.text(FIRST_LINE + index, side.label, line.label);
      side.amounts.forEach((col, column) =>
        amount(FIRST_LINE + index, col, line.amounts[column]),
      );
    });

  lines(left, statement.revenue);
  lines(right, statement.expenses);

  const lastRevenue = FIRST_LINE + statement.revenue.length - 1;
  const lastExpense = FIRST_LINE + statement.expenses.length - 1;
  const expenseTotalRow = lastExpense + 1;
  const profitRow = expenseTotalRow + 2;
  const totalRow = profitRow + 2;

  sheet.text(expenseTotalRow, right.label, "TOTAL BIAYA", "totalText");
  sheet.text(profitRow, right.label, "LABA", "totalText");
  sheet.text(totalRow, left.label, "TOTAL", "totalText");
  sheet.text(totalRow, right.label, "TOTAL", "totalText");

  input.periods.forEach((_, column) => {
    const revenueCol = left.amounts[column];
    const expenseCol = right.amounts[column];
    const expenseTotal = statement.expenseTotal[column];
    const revenueTotal = statement.revenueTotal[column];

    // A year with only its Laba typed: no rincian to add up.
    if (expenseTotal === null || revenueTotal === null) {
      sheet.blank(expenseTotalRow, expenseCol, "totalNumber");
      sheet.number(
        profitRow,
        expenseCol,
        statement.profit[column],
        "totalNumber",
      );
      sheet.blank(totalRow, revenueCol, "totalNumber");
      sheet.blank(totalRow, expenseCol, "totalNumber");
      return;
    }

    const revenueSum = `SUM(${cellRef(FIRST_LINE, revenueCol)}:${cellRef(lastRevenue, revenueCol)})`;

    sheet.formula(
      expenseTotalRow,
      expenseCol,
      `SUM(${cellRef(FIRST_LINE, expenseCol)}:${cellRef(lastExpense, expenseCol)})`,
      expenseTotal,
      "totalNumber",
    );
    sheet.formula(
      profitRow,
      expenseCol,
      `${revenueSum}-${cellRef(expenseTotalRow, expenseCol)}`,
      statement.profit[column],
      "totalNumber",
    );
    sheet.formula(
      totalRow,
      revenueCol,
      revenueSum,
      revenueTotal,
      "totalNumber",
    );
    sheet.formula(
      totalRow,
      expenseCol,
      `${cellRef(expenseTotalRow, expenseCol)}+${cellRef(profitRow, expenseCol)}`,
      revenueTotal,
      "totalNumber",
    );
  });

  [36, 15, 15, 36, 15, 15].forEach((width, index) =>
    sheet.width(index + 1, width),
  );

  return sheet.build(input.fileName, "LABA RUGI");
}
