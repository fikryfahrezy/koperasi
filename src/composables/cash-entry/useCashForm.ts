import { computed, watch } from "vue";

import {
  BalanceSheetItem,
  bonStatuses,
  cashAndBankAt,
} from "../../balance-sheet";

import {
  balanceSheetCategories,
  CashCategory,
  cashCategories,
  Channel,
  formatDate,
  type PostingOptions,
  TransactionDirection,
  useBalanceSheetOf,
  useCashBookOf,
  useKoperasiStore,
} from "../../store/koperasi";

import type { WorkspacePageState } from "../useWorkspacePageState";

// Kategori option for Kas Buku Tabungan movements that do not touch Saldo Kas;
// posted on the TABUNGAN channel without a Buku Kas category.
const SAVINGS_BOOK_ONLY = "SAVINGS_BOOK_ONLY";

// Bayar Kas Bon option for a payment that names none of the bons.
const NO_BON = "NO_BON";
const BON_DESCRIPTION = "Bayar Kas Bon ";

/** Groups of the Kategori list, in order. Biaya lowers Laba; the funds,
 * Bayar Kas Bon and hutang move a Neraca line instead; Kas & tabungan moves money
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
  businessDate: () => string,
) {
  const { postCashEntry, selectedYear } = useKoperasiStore();

  const otherForm = pageState.field("otherForm", () => ({
    category: "" as CashCategory | typeof SAVINGS_BOOK_ONLY | "",
    description: "",
    amount: 0,
    /** Bayar Kas Bon: whose bons it pays (`NO_BON`: no bon named). */
    bonPerson: "",
    /** Bayar Kas Bon: what it pays of each bon of `bonPerson`, by key. */
    bonAmounts: {} as Record<string, number>,
  }));

  // Bayar Kas Bon pays bons of one person on Piutang Lainnya of the year of
  // the date: the bons not yet Lunas, each with what is left of it. The
  // Jumlah is what it pays of them together.
  const cashDirection = computed(direction);

  const isSavingsBook = computed(
    () => otherForm.value.category === SAVINGS_BOOK_ONLY,
  );

  // A draft saved before these fields existed lacks them.
  otherForm.value.bonPerson ??= "";
  otherForm.value.bonAmounts ??= {};

  const bonYear = computed(
    () =>
      Number(String(businessDate?.() ?? "").slice(0, 4)) || selectedYear.value,
  );
  const bonSheet = useBalanceSheetOf(() => bonYear.value);
  const { cashBook: bonCashBook } = useCashBookOf(
    Channel.Cash,
    () => bonYear.value,
  );

  const isBonPayment = computed(
    () => otherForm.value.category === CashCategory.CashBondRepayment,
  );

  const openBons = computed(() => {
    const details =
      bonSheet.data.value?.details[BalanceSheetItem.OtherLoans] ?? [];
    const { statuses } = bonStatuses(
      details,
      cashAndBankAt(`${bonYear.value}-12`, bonCashBook.value).postedRows,
    );

    return details.flatMap((detail) => {
      const status = detail.bonKey ? statuses.get(detail.bonKey) : undefined;
      if (!detail.bonKey || !status || status.left <= 0) return [];

      return [
        {
          bonKey: detail.bonKey,
          name: detail.name,
          label: [
            detail.note || detail.name,
            detail.loanDate ? formatDate(detail.loanDate) : "",
          ]
            .filter(Boolean)
            .join(" · "),
          left: status.left,
        },
      ];
    });
  });

  const bonPersonOptions = computed(() => [
    ...[...new Set(openBons.value.map((bon) => bon.name))].map((name) => ({
      value: name,
      label: name,
    })),
    { value: NO_BON, label: "Tanpa rincian bon" },
  ]);

  /** The open bons of the person picked. */
  const personBons = computed(() =>
    isBonPayment.value
      ? openBons.value.filter((bon) => bon.name === otherForm.value.bonPerson)
      : [],
  );

  const bonParts = computed(() =>
    personBons.value.flatMap((bon) => {
      const amount = otherForm.value.bonAmounts[bon.bonKey] ?? 0;
      return amount > 0 ? [{ bonKey: bon.bonKey, amount }] : [];
    }),
  );

  /** A bon is paid more than what is left of it. */
  const bonOverpaid = computed(() =>
    personBons.value.some(
      (bon) => (otherForm.value.bonAmounts[bon.bonKey] ?? 0) > bon.left,
    ),
  );

  /** A Bayar Kas Bon still has to say whose bons it pays. */
  const bonMissing = computed(
    () =>
      isBonPayment.value &&
      !!openBons.value.length &&
      !bonPersonOptions.value.some(
        (option) => option.value === otherForm.value.bonPerson,
      ),
  );

  /** Every bon of the person paid in full. */
  function payAllBons() {
    otherForm.value.bonAmounts = Object.fromEntries(
      personBons.value.map((bon) => [bon.bonKey, bon.left]),
    );
  }

  // The Jumlah is what is paid of the bons; a person no longer listed (other
  // year, all Lunas) is cleared, and the only one is picked.
  watch([isBonPayment, bonPersonOptions], () => {
    const form = otherForm.value;

    if (!isBonPayment.value) form.bonPerson = "";
    else if (
      !bonPersonOptions.value.some((option) => option.value === form.bonPerson)
    )
      form.bonPerson =
        bonPersonOptions.value.length === 2
          ? bonPersonOptions.value[0].value
          : "";
  });

  watch(
    () => otherForm.value.bonPerson,
    (person, before) => {
      const form = otherForm.value;
      if (person === before) return;
      form.bonAmounts = {};

      // Fill in the Uraian, without overwriting text the user typed.
      const typed = form.description.trim();
      if (
        person &&
        person !== NO_BON &&
        (!typed || typed.startsWith(BON_DESCRIPTION))
      )
        form.description = `${BON_DESCRIPTION}${person}`;
    },
  );

  watch(bonParts, (parts) => {
    if (personBons.value.length)
      otherForm.value.amount = parts.reduce(
        (total, part) => total + part.amount,
        0,
      );
  });

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
      bonPerson: "",
      bonAmounts: {},
    });
  }

  async function submit(options: PostingOptions) {
    const form = otherForm.value;
    if (form.amount <= 0 || !form.description.trim()) return false;
    if (bonMissing.value || bonOverpaid.value) return false;
    const savingsBook = isSavingsBook.value;

    return postCashEntry(
      {
        direction: cashDirection.value,
        category: savingsBook ? "" : (form.category as CashCategory | ""),
        description: form.description.trim(),
        amount: form.amount,
        ...(bonParts.value.length ? { bonPayments: bonParts.value } : {}),
      },
      {
        ...options,
        channel: savingsBook ? Channel.SavingsBook : Channel.Cash,
      },
    );
  }

  return {
    otherForm,
    isSavingsBook,
    cashCategoryOptions,
    isBonPayment,
    openBons,
    bonPersonOptions,
    personBons,
    bonOverpaid,
    bonMissing,
    payAllBons,
    reset,
    submit,
  };
}
