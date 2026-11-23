// The LABA RUGI of BUKU KAS HARIAN: Pendapatan on the left, Biaya on the
// right, a column per date (31 Desember of the year before and the month
// shown), from the Laba Bersih of each Neraca (`balanceSheetAt`).
import {
  type BalanceSheet,
  type LinePart,
  expenseCategories,
} from "./balance-sheet";
import { CashCategory } from "./store/koperasi";

/** An amount per column; null where the column has no rincian (a year
 * without transactions in the app, whose Laba is typed). */
export type IncomeAmounts = (number | null)[];

export interface IncomeLine {
  label: string;
  amounts: IncomeAmounts;
}

export interface IncomeStatement {
  revenue: IncomeLine[];
  revenueTotal: IncomeAmounts;
  expenses: IncomeLine[];
  expenseTotal: IncomeAmounts;
  /** LABA: Pendapatan less Biaya, or the typed Laba of a year without
   * rincian. */
  profit: number[];
  /** TOTAL of both sides: Pendapatan, which is Biaya + Laba. */
  total: IncomeAmounts;
  /** Per column: only the typed Laba is known (no transactions). */
  typedOnly: boolean[];
  /** Per column: the Laba typed on the Neraca where it differs from the
   * Laba counted here. */
  typedProfit: (number | undefined)[];
}

/** The Biaya lines of the sheet, in its order. A line without categories
 * (Penghapusan Pinjaman) is 0. */
const expenseLines: { label: string; categories: CashCategory[] }[] = [
  { label: "Biaya Honor Pengurus", categories: [CashCategory.BoardExpense] },
  { label: "Biaya ATK", categories: [CashCategory.StationeryExpense] },
  { label: "Biaya Transfortasi", categories: [CashCategory.Transport] },
  { label: "Biaya Lainnya", categories: [CashCategory.Other] },
  { label: "Biaya Hutang", categories: [CashCategory.InterestExpense] },
  { label: "Sewa Kantor", categories: [CashCategory.OfficeRent] },
  { label: "Biaya Penghapusan Pinjaman", categories: [] },
  { label: "Biaya RAT", categories: [CashCategory.AnnualMeetingExpense] },
  {
    label: "Biaya Humas dan Lainnya",
    categories: [CashCategory.PublicRelations],
  },
  { label: "Pemeliharaan AT", categories: [CashCategory.AssetMaintenance] },
];

const sum = (values: number[]) =>
  values.reduce((total, value) => total + value, 0);

export function incomeStatement(sheets: BalanceSheet[]): IncomeStatement {
  const incomes = sheets.map((sheet) => sheet.income);

  const typedOnly = incomes.map(
    (income) =>
      income.typedNetIncome !== undefined &&
      income.interest === 0 &&
      income.provision === 0 &&
      income.expenseTotal === 0,
  );

  const amounts = (pick: (income: BalanceSheet["income"]) => number) =>
    incomes.map((income, column) => (typedOnly[column] ? null : pick(income)));

  const categoryAmount =
    (categories: readonly CashCategory[]) => (income: BalanceSheet["income"]) =>
      sum(
        income.expenses
          .filter((expense) => categories.includes(expense.category))
          .map((expense) => expense.amount),
      );

  // Expense categories outside the sheet's lines (Parcel), where used.
  const listed = expenseLines.flatMap((line) => line.categories);
  const extra = expenseCategories
    .filter((category) => !listed.includes(category.value))
    .map((category) => ({
      label: `Biaya ${category.label}`,
      categories: [category.value],
    }))
    .filter((line) =>
      incomes.some((income) => categoryAmount(line.categories)(income) !== 0),
    );

  const expenses = [...expenseLines, ...extra].map((line, index) => ({
    label: `${index + 1}. ${line.label}`,
    amounts: amounts(categoryAmount(line.categories)),
  }));

  const profit = incomes.map((income, column) =>
    typedOnly[column] ? income.typedNetIncome! : income.netIncome,
  );

  const revenueTotal = amounts((income) => income.interest + income.provision);

  return {
    revenue: [
      {
        label: "1. Pendapatan Bunga Pinjaman",
        amounts: amounts((income) => income.interest),
      },
      {
        label: "2. Pendapatan Provisi Pinjaman",
        amounts: amounts((income) => income.provision),
      },
      { label: "3. Pendapatan lainnya", amounts: amounts(() => 0) },
    ],
    revenueTotal,
    expenses,
    expenseTotal: amounts((income) => income.expenseTotal),
    profit,
    total: revenueTotal,
    typedOnly,
    typedProfit: incomes.map((income, column) =>
      !typedOnly[column] &&
      income.typedNetIncome !== undefined &&
      income.typedNetIncome !== income.netIncome
        ? income.typedNetIncome
        : undefined,
    ),
  };
}

/** A line of tab Rincian: its amount and the parts that add up to it. */
export interface IncomeBreakdownLine {
  label: string;
  amount: number;
  parts: LinePart[];
}

/** Tab Rincian of the Laba Rugi at one date: Bunga and Provisi per month,
 * each Biaya per Buku Kas row, in the lines of `incomeStatement`. */
export function incomeBreakdown(sheet: BalanceSheet) {
  const income = sheet.income;

  const revenue: IncomeBreakdownLine[] = [
    {
      label: "Pendapatan Bunga Pinjaman",
      amount: income.interest,
      parts: income.interestParts,
    },
    {
      label: "Pendapatan Provisi Pinjaman",
      amount: income.provision,
      parts: income.provisionParts,
    },
    { label: "Pendapatan lainnya", amount: 0, parts: [] },
  ];

  const lineOf = (label: string, categories: readonly CashCategory[]) => {
    const picked = income.expenses.filter((expense) =>
      categories.includes(expense.category),
    );

    return {
      label,
      amount: sum(picked.map((expense) => expense.amount)),
      // The line names the category already.
      parts: picked.flatMap((expense) =>
        expense.parts.map(({ category: _category, ...part }) => part),
      ),
    };
  };

  const listed = expenseLines.flatMap((line) => line.categories);
  const extra = expenseCategories
    .filter((category) => !listed.includes(category.value))
    .map((category) => lineOf(`Biaya ${category.label}`, [category.value]))
    .filter((line) => line.amount !== 0);

  const expenses: IncomeBreakdownLine[] = [
    ...expenseLines.map((line) => lineOf(line.label, line.categories)),
    ...extra,
  ];

  return {
    revenue,
    revenueTotal: income.interest + income.provision,
    expenses,
    expenseTotal: income.expenseTotal,
    // As in `incomeStatement`: a year with only its Laba typed shows that Laba.
    profit:
      income.typedNetIncome !== undefined &&
      income.interest === 0 &&
      income.provision === 0 &&
      income.expenseTotal === 0
        ? income.typedNetIncome
        : income.netIncome,
  };
}

export type IncomeBreakdown = ReturnType<typeof incomeBreakdown>;
