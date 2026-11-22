import { computed, watch } from "vue";

import {
  balanceSheetCategories,
  CashCategory,
  cashCategories,
  Channel,
  type PostingOptions,
  TransactionDirection,
  useKoperasiStore,
} from "../../store/koperasi";

import type { WorkspacePageState } from "../useWorkspacePageState";

// Kategori option for Kas Buku Tabungan movements that do not touch Saldo Kas;
// posted on the TABUNGAN channel without a Buku Kas category.
const SAVINGS_BOOK_ONLY = "SAVINGS_BOOK_ONLY";

/** Groups of the Kategori list, in order. Biaya lowers Laba; the funds,
 * Kas Bon and hutang move a Neraca line instead; Kas & tabungan moves money
 * between Kas and the bank. */
const CategoryGroup = {
  Expense: "Biaya",
  FundUse: "Pemakaian dana",
  BondsAndDebt: "Kas bon & hutang",
  CashAndSavings: "Kas & tabungan",
} as const;

const groupOrder: string[] = Object.values(CategoryGroup);

const fundUses = new Set<string>([
  CashCategory.SocialFund,
  CashCategory.Dekopinda,
  CashCategory.EducationFundUse,
  CashCategory.PmdkFundUse,
  CashCategory.RatReserveUse,
  CashCategory.RetainedEarningsUse,
  CashCategory.ShuPayout,
]);

const bondsAndDebt = new Set<string>([
  CashCategory.CashBond,
  CashCategory.CashBondRepayment,
  CashCategory.DebtReceived,
  CashCategory.DebtRepayment,
]);

// Expense columns of Buku Kas TAHUN 2026 (L–W), less Dansos and Dekopinda.
const expenses = new Set<string>(
  cashCategories
    .slice(3)
    .map((option) => option.value)
    .filter((value) => !fundUses.has(value)),
);

const groupOf = (value: string) =>
  expenses.has(value)
    ? CategoryGroup.Expense
    : fundUses.has(value)
      ? CategoryGroup.FundUse
      : bondsAndDebt.has(value)
        ? CategoryGroup.BondsAndDebt
        : CategoryGroup.CashAndSavings;

const cashCategoryDescriptions: string[] = cashCategories.flatMap((option) =>
  option.description ? [option.description] : [],
);

/** Kas masuk and Kas keluar: any other Buku Kas movement, or a Kas Buku
 * Tabungan movement that does not touch Saldo Kas. */
export function useCashForm(
  pageState: WorkspacePageState,
  direction: () => TransactionDirection,
) {
  const { postCashEntry } = useKoperasiStore();

  const otherForm = pageState.field("otherForm", () => ({
    category: "" as CashCategory | typeof SAVINGS_BOOK_ONLY | "",
    description: "",
    amount: 0,
  }));

  const cashDirection = computed(direction);

  const isSavingsBook = computed(
    () => otherForm.value.category === SAVINGS_BOOK_ONLY,
  );

  // Tambahan Kas is only cash in and Transfer ke Tabungan only cash out.
  // Alphabetical after "Tanpa kategori", with Lainnya last, in the groups of
  // `CategoryGroup`.
  const cashCategoryOptions = computed(() => [
    { value: "", label: "Tanpa kategori" },
    ...[
      ...[...cashCategories, ...balanceSheetCategories].filter(
        (option) =>
          !option.direction || option.direction === cashDirection.value,
      ),
      {
        value: SAVINGS_BOOK_ONLY,
        label:
          cashDirection.value === TransactionDirection.In
            ? "Tambahan Kas Tabungan"
            : "Pengeluaran Kas Tabungan",
      },
    ]
      .sort(
        (a, b) =>
          Number(a.value === CashCategory.Other) -
            Number(b.value === CashCategory.Other) ||
          a.label.localeCompare(b.label, "id-ID", { sensitivity: "base" }),
      )
      .map((option) => ({ ...option, group: groupOf(option.value) }))
      .sort(
        (a, b) => groupOrder.indexOf(a.group) - groupOrder.indexOf(b.group),
      ),
  ]);

  watch([() => otherForm.value.category, cashDirection], () => {
    const form = otherForm.value;

    if (
      !cashCategoryOptions.value.some(
        (option) => option.value === form.category,
      )
    )
      form.category = "";

    // Fill in the sheet's Uraian for the transfer categories, without
    // overwriting text the user typed.
    const selected = cashCategories.find(
      (option) => option.value === form.category,
    );

    const description = selected?.description ?? "";

    if (
      !form.description.trim() ||
      cashCategoryDescriptions.includes(form.description)
    )
      form.description = description;
  });

  function reset() {
    Object.assign(otherForm.value, {
      category: "",
      description: "",
      amount: 0,
    });
  }

  async function submit(options: PostingOptions) {
    const form = otherForm.value;
    if (form.amount <= 0 || !form.description.trim()) return false;
    const savingsBook = isSavingsBook.value;

    return postCashEntry(
      {
        direction: cashDirection.value,
        category: savingsBook ? "" : (form.category as CashCategory | ""),
        description: form.description.trim(),
        amount: form.amount,
      },
      {
        ...options,
        channel: savingsBook ? Channel.SavingsBook : Channel.Cash,
      },
    );
  }

  return { otherForm, isSavingsBook, cashCategoryOptions, reset, submit };
}
