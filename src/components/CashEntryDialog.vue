<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  BanknoteArrowDown,
  BanknoteArrowUp,
  Calculator,
  Coins,
  HandCoins,
  ReceiptText,
  WalletMinimal,
} from "lucide-vue-next";
import type { useWorkspacePageState } from "../composables/useWorkspacePageState";
import DateInput from "./DateInput.vue";
import MemberCombobox from "./MemberCombobox.vue";
import RupiahInput from "./RupiahInput.vue";
import UiModal from "./UiModal.vue";
import UiSelect from "./UiSelect.vue";
import {
  CashCategory,
  cashCategories,
  Channel,
  formatCurrency,
  formatLoanLabel,
  formatPeriod,
  InterestType,
  LoanGroup,
  LoanType,
  type LoanPreview,
  SavingsAccountType,
  SavingsMovement,
  todayIso,
  TransactionDirection,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

/** Catat transaksi: the single entry point for every Buku Kas movement. */
const props = defineProps<{
  /** The page's per-tab state, so each workspace tab keeps its own draft. */
  pageState: ReturnType<typeof useWorkspacePageState>;
}>();
const pageState = props.pageState;

const {
  members,
  loans,
  savingsBalances,
  financialParameters,
  createLoan,
  postCashEntry,
  postPayment,
  postSavingsTransaction,
  previewLoan,
} = useKoperasiStore();

const Category = {
  Payment: "payment",
  Loan: "loan",
  SavingsDeposit: "savings-deposit",
  SavingsWithdrawal: "savings-withdrawal",
  CashIn: "cash-in",
  CashOut: "cash-out",
} as const;
// Kategori option for Kas Buku Tabungan movements that do not touch Saldo Kas;
// posted on the TABUNGAN channel without a Buku Kas category.
const SAVINGS_BOOK_ONLY = "SAVINGS_BOOK_ONLY";
type Category = (typeof Category)[keyof typeof Category];
const modalOpen = pageState.field("modalOpen", () => false);
const submitting = ref(false);
const category = pageState.field<Category>("category", () => Category.Payment);
const posting = pageState.field("posting", () => ({
  businessDate: todayIso(),
}));
const paymentForm = pageState.field("paymentForm", () => ({
  loanGroup: LoanGroup.Member as LoanGroup,
  memberId: "",
  loanId: "",
  principal: 0,
  interest: 0,
  // Anggota: principal and interest per running loan of the member.
  lines: {} as Record<string, { principal: number; interest: number }>,
  wajib: 0,
  voluntary: 0,
}));
// Pokok, Wajib, and Manasuka in one transaction, as one Buku Kas row.
const savingsForm = pageState.field("savingsForm", () => ({
  memberId: "",
  principal: 0,
  mandatory: 0,
  voluntary: 0,
}));
const loanForm = pageState.field("loanForm", () => ({
  loanGroup: LoanGroup.Member as LoanGroup,
  memberId: "",
  borrowerName: "",
  plafond: 10_000_000,
  tenor: null as number | null,
  interestType: InterestType.Declining as InterestType,
  loanType: LoanType.Monthly as LoanType,
}));
const otherForm = pageState.field("otherForm", () => ({
  category: "" as CashCategory | typeof SAVINGS_BOOK_ONLY | "",
  description: "",
  amount: 0,
}));
// Uraian of payments, loans, and savings follows the generated text (as the
// backend writes it) until the user types their own; empty falls back to it.
const typedDescription = pageState.field("typedDescription", () => ({
  text: "",
  edited: false,
}));
// Provisi follows the parameter rate until the user types their own amount.
const loanProvision = pageState.field("loanProvision", () => ({
  amount: 0,
  edited: false,
}));
// Suplesi: the member's running loans this new loan pays off, with the
// interest still due on each.
const loanPayoffs = pageState.field(
  "loanPayoffs",
  () => ({}) as Record<string, { selected: boolean; interest: number }>,
);
const loanPreview = pageState.field<LoanPreview | null>(
  "loanPreview",
  () => null,
);

const memberOptions = computed(() =>
  members.map((member) => ({
    value: member.id,
    label: member.name,
    code: member.memberNumber,
  })),
);
const interestTypeOptions = [
  { value: InterestType.Declining, label: "Menurun" },
  { value: InterestType.Flat, label: "Plat" },
];
const loanGroupOptions = [
  { value: LoanGroup.Member, label: "Anggota PP BRI" },
  { value: LoanGroup.NonMember, label: "Non Anggota PP BRI" },
];
const loanTypeOptions = [
  { value: LoanType.Monthly, label: "Bulanan" },
  { value: LoanType.Temporary, label: "Sementara" },
];
const isCash = computed(
  () =>
    category.value === Category.CashIn || category.value === Category.CashOut,
);
const isSavingsBook = computed(
  () => isCash.value && otherForm.value.category === SAVINGS_BOOK_ONLY,
);
const cashDirection = computed(() =>
  category.value === Category.CashIn
    ? TransactionDirection.In
    : TransactionDirection.Out,
);
// Tambahan Kas is only cash in and Transfer ke Tabungan only cash out.
// Alphabetical after "Tanpa kategori", with Lainnya last.
const cashCategoryOptions = computed(() => [
  { value: "", label: "Tanpa kategori" },
  ...[
    ...cashCategories.filter(
      (option) => !option.direction || option.direction === cashDirection.value,
    ),
    {
      value: SAVINGS_BOOK_ONLY,
      label:
        cashDirection.value === TransactionDirection.In
          ? "Tambahan Kas Tabungan"
          : "Pengeluaran Kas Tabungan",
    },
  ].sort(
    (a, b) =>
      Number(a.value === CashCategory.Other) -
        Number(b.value === CashCategory.Other) ||
      a.label.localeCompare(b.label, "id-ID", { sensitivity: "base" }),
  ),
]);
const cashCategoryDescriptions: string[] = cashCategories.flatMap((option) =>
  option.description ? [option.description] : [],
);
watch([() => otherForm.value.category, cashDirection], () => {
  const form = otherForm.value;
  if (
    !cashCategoryOptions.value.some((option) => option.value === form.category)
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
const isNonMemberPayment = computed(
  () => paymentForm.value.loanGroup === LoanGroup.NonMember,
);
// Non Anggota pays one loan, picked by borrower. Anggota picks the member,
// and every running loan of that member gets its own pokok and bunga, so one
// deposit can pay several loans in one Buku Kas row, as in the sheet.
const memberNumbers = computed(
  () => new Map(members.map((member) => [member.id, member.memberNumber])),
);
const runningLoans = computed(() =>
  loans.filter(
    (loan) =>
      loan.loanGroup === paymentForm.value.loanGroup &&
      loan.balance > 0 &&
      (isNonMemberPayment.value || loan.memberId),
  ),
);
const paymentLoanOptions = computed(() =>
  runningLoans.value
    .map((loan) => ({
      value: loan.id,
      label: `${loan.memberName} · ${formatLoanLabel(loan)}`,
    }))
    .sort((a, b) => a.label.localeCompare(b.label, "id-ID")),
);
const paymentMemberOptions = computed(() =>
  [...new Set(runningLoans.value.map((loan) => loan.memberId))]
    .map((memberId) => ({
      value: memberId,
      label:
        members.find((member) => member.id === memberId)?.name ??
        runningLoans.value.find((loan) => loan.memberId === memberId)
          ?.memberName ??
        "",
      code: memberNumbers.value.get(memberId) ?? "",
    }))
    .sort((a, b) => a.label.localeCompare(b.label, "id-ID")),
);
watch(
  () => paymentForm.value.loanGroup,
  () => {
    if (
      !paymentLoanOptions.value.some(
        (option) => option.value === paymentForm.value.loanId,
      )
    )
      paymentForm.value.loanId = "";
    if (
      !paymentMemberOptions.value.some(
        (option) => option.value === paymentForm.value.memberId,
      )
    )
      paymentForm.value.memberId = "";
  },
);
// The loans this payment goes to: the picked loan for Non Anggota, every
// running loan of the picked member for Anggota.
const paymentLoans = computed(() =>
  isNonMemberPayment.value
    ? loans.filter((loan) => loan.id === paymentForm.value.loanId)
    : runningLoans.value.filter(
        (loan) =>
          paymentForm.value.memberId &&
          loan.memberId === paymentForm.value.memberId,
      ),
);
const paymentLoanBalance = computed(() =>
  paymentLoans.value.reduce((sum, loan) => sum + loan.balance, 0),
);
// Interest of the transaction month: this month's installment ("Kewajiban
// setor tiap bulan") and what is still owed through it ("Kewajiban setor s/d"
// on the Pinjaman page, less what was already paid that month).
const { monthlyLedger } = useMonthlyLedger();
const postingPeriod = computed(() => posting.value.businessDate.slice(0, 7));
const interestOfMonth = computed(
  () =>
    new Map(
      monthlyLedger.value.loans
        .filter((row) => row.period === postingPeriod.value)
        .map((row) => [
          row.loanId,
          { scheduled: row.scheduledInterest, due: row.arrearsInterest },
        ]),
    ),
);
function interestDueLines(loanIds: string[]) {
  let scheduled = 0;
  let due = 0;
  for (const loanId of loanIds) {
    const interest = interestOfMonth.value.get(loanId);
    if (!interest) return [];
    scheduled += interest.scheduled;
    due += interest.due;
  }
  if (!loanIds.length) return [];
  const dueText =
    due < 0
      ? `Lebih bayar bunga ${formatCurrency(-due)}`
      : `Kewajiban bunga s/d ${formatPeriod(postingPeriod.value)} ${formatCurrency(due)}`;
  return [`Bunga per bulan ${formatCurrency(scheduled)}`, dueText];
}
/** A payment that pays off the loan's principal but not exactly its interest
 * due leaves interest arrears on a paid-off loan; warn, but still allow it
 * (the interest may be waived). */
function payoffInterestWarning(
  loan: (typeof loans)[number],
  principal: number,
  interest: number,
) {
  const due = interestOfMonth.value.get(loan.id)?.due;
  if (due === undefined || loan.balance <= 0 || principal !== loan.balance)
    return "";
  const gap = due - (interest || 0);
  if (!gap) return "";
  return gap > 0
    ? `Pinjaman lunas, tapi bunga masih kurang ${formatCurrency(gap)}.`
    : `Pinjaman lunas, tapi bunga lebih ${formatCurrency(-gap)}.`;
}
/** One month's installment, to start from: the outstanding balance helps
 * fill in the principal. */
function installmentOf(loan: (typeof loans)[number]) {
  return {
    principal: Math.min(
      Math.ceil(loan.plafond / loan.tenor / 1000) * 1000,
      loan.balance,
    ),
    interest: Math.round(
      ((loan.interestType === InterestType.Flat ? loan.plafond : loan.balance) *
        loan.rate) /
        100 /
        12,
    ),
  };
}
function paymentLine(loanId: string) {
  paymentForm.value.lines ??= {};
  return (paymentForm.value.lines[loanId] ??= { principal: 0, interest: 0 });
}
watch(
  [
    () => pageState.tabId.value,
    () => paymentForm.value.memberId,
    () => paymentForm.value.loanId,
    () => paymentForm.value.loanGroup,
  ],
  ([tabId], [previousTabId]) => {
    if (tabId !== previousTabId) return;
    const installments = paymentLoans.value.map(
      (loan) => [loan.id, installmentOf(loan)] as const,
    );
    paymentForm.value.lines = Object.fromEntries(installments);
    const [first] = installments;
    paymentForm.value.principal = first?.[1].principal ?? 0;
    paymentForm.value.interest = first?.[1].interest ?? 0;
  },
);
const paymentTotal = computed(() =>
  isNonMemberPayment.value
    ? (paymentForm.value.principal || 0) + (paymentForm.value.interest || 0)
    : paymentLoans.value.reduce(
        (sum, loan) =>
          sum +
          (paymentLine(loan.id).principal || 0) +
          (paymentLine(loan.id).interest || 0),
        0,
      ) +
      (paymentForm.value.wajib || 0) +
      (paymentForm.value.voluntary || 0),
);

const topUpLoans = computed(() =>
  loanForm.value.loanGroup === LoanGroup.Member && loanForm.value.memberId
    ? loans.filter(
        (loan) =>
          loan.memberId === loanForm.value.memberId &&
          loan.loanGroup === LoanGroup.Member &&
          loan.balance > 0,
      )
    : [],
);
function loanPayoff(loanId: string) {
  return (loanPayoffs.value[loanId] ??= { selected: false, interest: 0 });
}
watch(
  [() => pageState.tabId.value, () => loanForm.value.memberId],
  ([tabId], [previousTabId]) => {
    if (tabId !== previousTabId) return;
    loanPayoffs.value = Object.fromEntries(
      topUpLoans.value.map((loan) => [
        loan.id,
        { selected: false, interest: installmentOf(loan).interest },
      ]),
    );
  },
);
const selectedPayoffs = computed(() =>
  topUpLoans.value
    .filter((loan) => loanPayoff(loan.id).selected)
    .map((loan) => ({
      loanId: loan.id,
      principal: loan.balance,
      interest: loanPayoff(loan.id).interest || 0,
    })),
);
const payoffTotal = computed(() =>
  selectedPayoffs.value.reduce(
    (sum, payoff) => sum + payoff.principal + payoff.interest,
    0,
  ),
);
const exceedsTopUpPlafond = computed(
  () =>
    selectedPayoffs.value.length > 0 &&
    payoffTotal.value + loanProvision.value.amount > loanForm.value.plafond,
);
/** Provisi at the parameter rate. A suplesi charges it on the new money
 * only, as in the sheet: plafond minus what pays off the old loans. */
function rateProvision(preview: LoanPreview | null) {
  if (!payoffTotal.value) return preview?.provision ?? 0;
  const base = Math.max(loanForm.value.plafond - payoffTotal.value, 0);
  return Math.round((base * financialParameters.provisionRate) / 100);
}
watch(
  [() => pageState.tabId.value, payoffTotal],
  ([tabId], [previousTabId]) => {
    if (tabId !== previousTabId) return;
    loanProvision.value.edited = false;
    loanProvision.value.amount = rateProvision(loanPreview.value);
  },
);

watch(
  loanForm,
  async (_, __, onCleanup) => {
    const target = loanPreview.forTab(pageState.tabId.value);
    const provision = loanProvision.forTab(pageState.tabId.value);
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    const { tenor } = loanForm.value;
    if (!tenor || tenor <= 0) {
      target.value = null;
      return;
    }
    try {
      const result = await previewLoan({ ...loanForm.value, tenor });
      if (cancelled) return;
      target.value = result;
      if (!provision.value.edited)
        provision.value.amount = rateProvision(result);
    } catch {
      if (!cancelled) target.value = null;
    }
  },
  { deep: true, immediate: true },
);

watch(
  () => loanForm.value.plafond,
  () => {
    loanProvision.value.edited = false;
  },
);

function openModal() {
  category.value = Category.Payment;
  posting.value.businessDate = todayIso();
  Object.assign(paymentForm.value, {
    loanGroup: LoanGroup.Member,
    memberId: "",
    loanId: "",
    principal: 0,
    interest: 0,
    wajib: 0,
    voluntary: 0,
  });
  Object.assign(savingsForm.value, {
    memberId: "",
    principal: 0,
    mandatory: 0,
    voluntary: 0,
  });
  Object.assign(loanForm.value, {
    loanGroup: LoanGroup.Member,
    memberId: "",
    borrowerName: "",
    plafond: 10_000_000,
    tenor: null,
    interestType: InterestType.Declining,
    loanType: LoanType.Monthly,
  });
  Object.assign(loanProvision.value, { amount: 0, edited: false });
  loanPayoffs.value = {};
  Object.assign(otherForm.value, {
    category: "",
    description: "",
    amount: 0,
  });
  Object.assign(typedDescription.value, { text: "", edited: false });
  modalOpen.value = true;
}
const isSavings = computed(
  () =>
    category.value === Category.SavingsDeposit ||
    category.value === Category.SavingsWithdrawal,
);
// Every amount starts empty: wajib is typed as paid, not prefilled.
function setSavingsDefaults() {
  Object.assign(savingsForm.value, {
    principal: 0,
    mandatory: 0,
    voluntary: 0,
  });
}
const savingsAmounts = computed(() =>
  (
    [
      [SavingsAccountType.Principal, savingsForm.value.principal],
      [SavingsAccountType.Mandatory, savingsForm.value.mandatory],
      [SavingsAccountType.Voluntary, savingsForm.value.voluntary],
    ] as const
  )
    .map(([accountType, amount]) => ({ accountType, amount: amount || 0 }))
    .filter((item) => item.amount > 0),
);
const savingsTotal = computed(() =>
  savingsAmounts.value.reduce((sum, item) => sum + item.amount, 0),
);
// Show what each account holds; "Ambil semua saldo" fills them all in.
// It only withdraws: the member stays a member.
const savingsMemberBalances = computed(() => {
  const member = members.find((item) => item.id === savingsForm.value.memberId);
  const balanceOf = (accountType: SavingsAccountType) =>
    savingsBalances.find(
      (item) =>
        item.memberId === savingsForm.value.memberId &&
        item.accountType === accountType,
    )?.balance ?? 0;
  return {
    principal: member?.principalSavings ?? 0,
    mandatory: balanceOf(SavingsAccountType.Mandatory),
    voluntary: balanceOf(SavingsAccountType.Voluntary),
  };
});
function withdrawAllSavings() {
  Object.assign(savingsForm.value, savingsMemberBalances.value);
}

const memberName = (id: string) =>
  members.find((member) => member.id === id)?.name ?? "";
const savingsAccountWords = {
  [SavingsAccountType.Principal]: "Pokok",
  [SavingsAccountType.Mandatory]: "Wajib",
  [SavingsAccountType.Voluntary]: "Manasuka",
} satisfies Record<SavingsAccountType, string>;
/** The Uraian the backend writes when none is typed; "" until it is known. */
const generatedDescription = computed(() => {
  if (category.value === Category.Payment) {
    const loan = paymentLoans.value[0];
    if (!loan) return "";
    return `Setoran ${isNonMemberPayment.value ? loan.memberName : memberName(loan.memberId)}`;
  }
  if (isSavings.value) {
    const name = memberName(savingsForm.value.memberId);
    const words = savingsAmounts.value.map(
      (item) => savingsAccountWords[item.accountType],
    );
    if (!name || !words.length) return "";
    const accounts =
      words.length === 1
        ? `Simpanan ${words[0]}`
        : `Simpanan ${words.slice(0, -1).join(", ")} & ${words.at(-1)}`;
    return `${category.value === Category.SavingsWithdrawal ? "Pengambilan" : "Setoran"} ${accounts} ${name}`;
  }
  if (category.value === Category.Loan) {
    const name =
      loanForm.value.loanGroup === LoanGroup.Member
        ? memberName(loanForm.value.memberId)
        : loanForm.value.borrowerName.trim();
    if (!name) return "";
    return selectedPayoffs.value.length
      ? `Suplesi ${name} Jk ${loanForm.value.tenor ?? ""} bulan`
      : `Realisasi Pinjaman ${name}`;
  }
  return "";
});
const descriptionInput = computed({
  get: () =>
    typedDescription.value.edited
      ? typedDescription.value.text
      : generatedDescription.value,
  set: (text: string) => {
    typedDescription.value = { text, edited: true };
  },
});
// Sent only when typed, so the backend's own text stays the source otherwise.
const descriptionOverride = computed(() =>
  typedDescription.value.edited ? typedDescription.value.text.trim() : "",
);
watch(category, () => {
  typedDescription.value = { text: "", edited: false };
});

const submitLabel = computed(
  () =>
    (
      ({
        [Category.Payment]: `Catat setoran ${formatCurrency(paymentTotal.value)}`,
        [Category.Loan]: "Catat realisasi",
        [Category.SavingsDeposit]: `Catat setoran ${formatCurrency(savingsTotal.value)}`,
        [Category.SavingsWithdrawal]: `Catat pengambilan ${formatCurrency(savingsTotal.value)}`,
        [Category.CashIn]: "Catat",
        [Category.CashOut]: "Catat",
      }) satisfies Record<Category, string>
    )[category.value],
);

async function submit() {
  const submittedModal = modalOpen.forTab(pageState.tabId.value);
  const options = { businessDate: posting.value.businessDate };
  submitting.value = true;
  try {
    let saved = false;
    if (category.value === Category.Payment) {
      const loan = paymentLoans.value[0];
      if (!loan || paymentTotal.value <= 0) return;
      saved = await postPayment(
        isNonMemberPayment.value
          ? {
              memberId: "",
              loanId: loan.id,
              principal: paymentForm.value.principal || 0,
              interest: paymentForm.value.interest || 0,
              wajib: 0,
              voluntary: 0,
              description: descriptionOverride.value,
              reference: "",
            }
          : {
              memberId: loan.memberId,
              principal: 0,
              interest: 0,
              loans: paymentLoans.value.map((item) => ({
                loanId: item.id,
                principal: paymentLine(item.id).principal || 0,
                interest: paymentLine(item.id).interest || 0,
              })),
              wajib: paymentForm.value.wajib || 0,
              voluntary: paymentForm.value.voluntary || 0,
              description: descriptionOverride.value,
              reference: "",
            },
        options,
      );
    } else if (isSavings.value) {
      if (!savingsForm.value.memberId || savingsTotal.value <= 0) return;
      saved = await postSavingsTransaction(
        {
          memberId: savingsForm.value.memberId,
          amounts: savingsAmounts.value,
          movement:
            category.value === Category.SavingsWithdrawal
              ? SavingsMovement.Withdrawal
              : SavingsMovement.Deposit,
          description: descriptionOverride.value,
          reference: "",
        },
        options,
      );
    } else if (category.value === Category.Loan) {
      const borrowerMissing =
        loanForm.value.loanGroup === LoanGroup.Member
          ? !loanForm.value.memberId
          : !loanForm.value.borrowerName.trim();
      const { tenor } = loanForm.value;
      if (
        borrowerMissing ||
        loanForm.value.plafond <= 0 ||
        !tenor ||
        tenor <= 0 ||
        !Number.isFinite(loanProvision.value.amount) ||
        exceedsTopUpPlafond.value
      )
        return;
      saved = await createLoan(
        {
          ...loanForm.value,
          tenor,
          provision: loanProvision.value.amount,
          memberId:
            loanForm.value.loanGroup === LoanGroup.Member
              ? loanForm.value.memberId
              : "",
          borrowerName: loanForm.value.borrowerName.trim(),
          payoffs: selectedPayoffs.value,
          description: descriptionOverride.value,
        },
        options,
      );
    } else {
      if (otherForm.value.amount <= 0) return;
      if (!otherForm.value.description.trim()) return;
      const savingsBook = isSavingsBook.value;
      saved = await postCashEntry(
        {
          direction: cashDirection.value,
          category: savingsBook
            ? ""
            : (otherForm.value.category as CashCategory | ""),
          description: otherForm.value.description.trim(),
          amount: otherForm.value.amount,
          reference: "",
        },
        {
          ...options,
          channel: savingsBook ? Channel.SavingsBook : Channel.Cash,
        },
      );
    }
    if (saved) submittedModal.value = false;
  } finally {
    submitting.value = false;
  }
}

defineExpose({ open: openModal });
</script>

<template>
  <UiModal
    :open="modalOpen && pageState.active.value"
    size="lg"
    title="Catat transaksi"
    @close="modalOpen = false"
  >
    <form class="form-stack" @submit.prevent="submit">
      <fieldset class="movement-field">
        <legend>Jenis transaksi</legend>
        <div class="movement-options">
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.Payment }"
          >
            <input v-model="category" type="radio" :value="Category.Payment" />
            <ReceiptText :size="18" />
            <span>Setoran anggota</span>
          </label>
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.Loan }"
          >
            <input v-model="category" type="radio" :value="Category.Loan" />
            <HandCoins :size="18" />
            <span>Realisasi pinjaman</span>
          </label>
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.SavingsDeposit }"
          >
            <input
              v-model="category"
              type="radio"
              :value="Category.SavingsDeposit"
              @change="setSavingsDefaults"
            />
            <Coins :size="18" />
            <span>Setoran simpanan</span>
          </label>
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.SavingsWithdrawal }"
          >
            <input
              v-model="category"
              type="radio"
              :value="Category.SavingsWithdrawal"
              @change="setSavingsDefaults"
            />
            <WalletMinimal :size="18" />
            <span>Pengambilan simpanan</span>
          </label>
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.CashIn }"
          >
            <input v-model="category" type="radio" :value="Category.CashIn" />
            <BanknoteArrowDown :size="18" />
            <span>Kas masuk</span>
          </label>
          <label
            class="movement-option"
            :class="{ 'is-selected': category === Category.CashOut }"
          >
            <input v-model="category" type="radio" :value="Category.CashOut" />
            <BanknoteArrowUp :size="18" />
            <span>Kas keluar</span>
          </label>
        </div>
      </fieldset>

      <div class="field">
        <span>Tanggal</span>
        <DateInput v-model="posting.businessDate" required />
      </div>

      <template v-if="category === Category.Payment">
        <label class="field">
          <span>Kelompok</span>
          <UiSelect
            v-model="paymentForm.loanGroup"
            :options="loanGroupOptions"
            aria-label="Kelompok"
          />
        </label>
        <label v-if="isNonMemberPayment" class="field">
          <span>Peminjam</span>
          <MemberCombobox
            v-model="paymentForm.loanId"
            :options="paymentLoanOptions"
            aria-label="Peminjam"
            placeholder="Cari nama peminjam"
            required
          />
        </label>
        <label v-else class="field">
          <span>Anggota</span>
          <MemberCombobox
            v-model="paymentForm.memberId"
            :options="paymentMemberOptions"
            aria-label="Anggota"
            placeholder="Cari nama atau ID anggota"
            empty-text="Tidak ada anggota dengan pinjaman berjalan yang cocok."
            required
          />
        </label>
        <template v-if="isNonMemberPayment">
          <p v-if="paymentLoans.length" class="field-hint">
            Sisa pokok pinjaman {{ formatCurrency(paymentLoanBalance) }}
            <template
              v-for="line in interestDueLines(
                paymentLoans.map((loan) => loan.id),
              )"
              :key="line"
            >
              <br />{{ line }}
            </template>
          </p>
        </template>
        <div v-if="!isNonMemberPayment" class="field-row">
          <label class="field">
            <span>Simpanan wajib</span>
            <RupiahInput
              v-model="paymentForm.wajib"
              :min="0"
              aria-label="Simpanan wajib"
            />
          </label>
          <label class="field">
            <span>Manasuka</span>
            <RupiahInput
              v-model="paymentForm.voluntary"
              :min="0"
              aria-label="Manasuka"
            />
          </label>
        </div>
        <section
          v-if="!isNonMemberPayment && paymentLoans.length"
          class="payment-loans"
        >
          <p class="payment-loans__title">Angsuran pinjaman</p>
          <div v-for="loan in paymentLoans" :key="loan.id" class="payment-loan">
            <p v-if="paymentLoans.length > 1" class="payment-loan__title">
              {{ formatLoanLabel(loan) }}
            </p>
            <p class="field-hint">
              Sisa pokok pinjaman {{ formatCurrency(loan.balance) }}
              <template v-for="line in interestDueLines([loan.id])" :key="line">
                <br />{{ line }}
              </template>
            </p>
            <div class="field-row">
              <label class="field">
                <span>Angsuran pokok</span>
                <RupiahInput
                  v-model="paymentLine(loan.id).principal"
                  :min="0"
                  :aria-label="
                    paymentLoans.length > 1
                      ? `Angsuran pokok ${formatLoanLabel(loan)}`
                      : 'Angsuran pokok'
                  "
                />
              </label>
              <label class="field">
                <span>Bunga</span>
                <RupiahInput
                  v-model="paymentLine(loan.id).interest"
                  :min="0"
                  :aria-label="
                    paymentLoans.length > 1
                      ? `Bunga ${formatLoanLabel(loan)}`
                      : 'Bunga'
                  "
                />
              </label>
            </div>
            <p
              v-if="
                payoffInterestWarning(
                  loan,
                  paymentLine(loan.id).principal,
                  paymentLine(loan.id).interest,
                )
              "
              class="field-hint field-hint--warning"
              role="status"
            >
              {{
                payoffInterestWarning(
                  loan,
                  paymentLine(loan.id).principal,
                  paymentLine(loan.id).interest,
                )
              }}
            </p>
          </div>
        </section>
        <div v-if="isNonMemberPayment" class="field-row">
          <label class="field">
            <span>Angsuran pokok</span>
            <RupiahInput
              v-model="paymentForm.principal"
              :min="0"
              aria-label="Angsuran pokok"
            />
          </label>
          <label class="field">
            <span>Bunga</span>
            <RupiahInput
              v-model="paymentForm.interest"
              :min="0"
              aria-label="Bunga"
            />
          </label>
        </div>
        <p
          v-if="
            isNonMemberPayment &&
            paymentLoans[0] &&
            payoffInterestWarning(
              paymentLoans[0],
              paymentForm.principal,
              paymentForm.interest,
            )
          "
          class="field-hint field-hint--warning"
          role="status"
        >
          {{
            payoffInterestWarning(
              paymentLoans[0],
              paymentForm.principal,
              paymentForm.interest,
            )
          }}
        </p>
      </template>

      <template v-else-if="isSavings">
        <label class="field">
          <span>Anggota</span>
          <MemberCombobox
            v-model="savingsForm.memberId"
            :options="memberOptions"
            aria-label="Anggota"
            placeholder="Cari nama atau ID anggota"
            required
          />
        </label>
        <div class="field-row field-row--three">
          <label
            v-for="account in [
              { key: 'principal', label: 'Pokok' },
              { key: 'mandatory', label: 'Wajib' },
              { key: 'voluntary', label: 'Manasuka' },
            ] as const"
            :key="account.key"
            class="field"
          >
            <span>{{ account.label }}</span>
            <RupiahInput
              v-model="savingsForm[account.key]"
              :min="0"
              :aria-label="`Simpanan ${account.label.toLowerCase()}`"
            />
            <small
              v-if="
                category === Category.SavingsWithdrawal && savingsForm.memberId
              "
              class="field-hint"
            >
              Saldo {{ formatCurrency(savingsMemberBalances[account.key]) }}
            </small>
          </label>
        </div>
        <button
          v-if="category === Category.SavingsWithdrawal && savingsForm.memberId"
          class="button button--secondary savings-all"
          type="button"
          @click="withdrawAllSavings"
        >
          Ambil semua saldo
        </button>
      </template>

      <template v-else-if="category === Category.Loan">
        <div class="field-row">
          <label class="field">
            <span>Kelompok</span>
            <UiSelect
              v-model="loanForm.loanGroup"
              :options="loanGroupOptions"
              aria-label="Kelompok pinjaman"
            />
          </label>
          <label class="field">
            <span>Jenis pinjaman</span>
            <UiSelect
              v-model="loanForm.loanType"
              :options="loanTypeOptions"
              aria-label="Jenis pinjaman"
            />
          </label>
        </div>
        <label v-if="loanForm.loanGroup === LoanGroup.Member" class="field">
          <span>Anggota</span>
          <MemberCombobox
            v-model="loanForm.memberId"
            :options="memberOptions"
            aria-label="Anggota"
            placeholder="Cari nama atau ID anggota"
            required
          />
        </label>
        <fieldset v-if="topUpLoans.length" class="payoff-field">
          <legend>Lunasi pinjaman lama (suplesi)</legend>
          <div v-for="loan in topUpLoans" :key="loan.id" class="payoff-loan">
            <label class="payoff-loan__check">
              <input v-model="loanPayoff(loan.id).selected" type="checkbox" />
              <span>
                {{ formatLoanLabel(loan) }}
                <small>
                  Sisa pokok {{ formatCurrency(loan.balance) }}
                  <template
                    v-for="line in interestDueLines([loan.id])"
                    :key="line"
                  >
                    <br />{{ line }}
                  </template>
                </small>
              </span>
            </label>
            <label v-if="loanPayoff(loan.id).selected" class="field">
              <span>Bunga</span>
              <RupiahInput
                v-model="loanPayoff(loan.id).interest"
                :min="0"
                :aria-label="`Bunga pelunasan ${formatLoanLabel(loan)}`"
              />
            </label>
            <p
              v-if="
                loanPayoff(loan.id).selected &&
                payoffInterestWarning(
                  loan,
                  loan.balance,
                  loanPayoff(loan.id).interest,
                )
              "
              class="field-hint field-hint--warning"
              role="status"
            >
              {{
                payoffInterestWarning(
                  loan,
                  loan.balance,
                  loanPayoff(loan.id).interest,
                )
              }}
            </p>
          </div>
        </fieldset>
        <label v-if="loanForm.loanGroup !== LoanGroup.Member" class="field">
          <span>Nama peminjam</span>
          <input v-model="loanForm.borrowerName" required />
        </label>
        <div class="field-row">
          <label class="field">
            <span>Plafond pinjaman</span>
            <RupiahInput
              v-model="loanForm.plafond"
              :min="100000"
              min-message="Plafond pinjaman minimal Rp100.000."
              aria-label="Plafond pinjaman dalam Rupiah"
              required
            />
          </label>
          <label class="field">
            <span>Jangka waktu (bulan)</span>
            <input
              v-model.number="loanForm.tenor"
              type="number"
              inputmode="numeric"
              min="1"
              max="360"
              step="1"
              aria-label="Jangka waktu dalam bulan"
              placeholder="Jumlah bulan"
              required
            />
          </label>
        </div>
        <div class="field-row">
          <label class="field">
            <span>Jenis bunga</span>
            <UiSelect
              v-model="loanForm.interestType"
              :options="interestTypeOptions"
              aria-label="Jenis bunga"
            />
          </label>
          <label class="field">
            <span>Provisi</span>
            <RupiahInput
              v-model="loanProvision.amount"
              :min="0"
              aria-label="Provisi dalam Rupiah"
              required
              @update:model-value="loanProvision.edited = true"
            />
          </label>
        </div>
        <div class="calculation-preview">
          <div class="calculation-preview__title">
            <Calculator :size="19" />
            <template v-if="payoffTotal">
              Dicatat: suplesi {{ formatCurrency(loanForm.plafond) }} (debet),
              provisi dan pelunasan pinjaman lama (kredit)
            </template>
            <template v-else>
              Dicatat: realisasi
              {{ formatCurrency(loanForm.plafond) }} (debet) dan provisi
              (kredit)
            </template>
          </div>
          <dl>
            <div>
              <dt>Angsuran pokok / bulan</dt>
              <dd>
                {{ formatCurrency(loanPreview?.principalInstallment ?? 0) }}
              </dd>
            </div>
            <div>
              <dt>Bunga bulan pertama</dt>
              <dd>{{ formatCurrency(loanPreview?.firstInterest ?? 0) }}</dd>
            </div>
            <div>
              <dt>Provisi</dt>
              <dd>{{ formatCurrency(loanProvision.amount) }}</dd>
            </div>
            <template v-if="payoffTotal">
              <div>
                <dt>Pelunasan pinjaman lama</dt>
                <dd>{{ formatCurrency(payoffTotal) }}</dd>
              </div>
              <div>
                <dt>Diterima anggota</dt>
                <dd>
                  {{
                    formatCurrency(
                      loanForm.plafond - loanProvision.amount - payoffTotal,
                    )
                  }}
                </dd>
              </div>
            </template>
          </dl>
          <p v-if="exceedsTopUpPlafond" class="field-hint" role="alert">
            Pelunasan pinjaman lama dan provisi tidak boleh melebihi plafond
            pinjaman.
          </p>
        </div>
      </template>

      <template v-else-if="isCash">
        <label class="field">
          <span>Kategori</span>
          <UiSelect
            v-model="otherForm.category"
            :options="cashCategoryOptions"
            aria-label="Kategori Buku Kas"
          />
        </label>
        <p v-if="isSavingsBook" class="field-hint">
          Hanya mengubah Kas Buku Tabungan; Saldo Kas tidak berubah. Untuk
          memindahkan uang antara kas dan tabungan, pakai kategori Transfer ke
          Tabungan atau Tambahan Kas dari Tabungan.
        </p>
        <label class="field">
          <span>Uraian</span>
          <input
            v-model="otherForm.description"
            placeholder="mis. Biaya Pulsa Karyawan Koperasi 3 orang"
            required
          />
        </label>
        <label class="field">
          <span>Jumlah</span>
          <RupiahInput
            v-model="otherForm.amount"
            :min="1"
            aria-label="Jumlah"
            required
          />
        </label>
      </template>

      <label v-if="!isCash" class="field">
        <span>Uraian</span>
        <input v-model="descriptionInput" placeholder="Keterangan transaksi" />
      </label>

      <div class="modal-actions">
        <button
          class="button button--secondary"
          type="button"
          @click="modalOpen = false"
        >
          Batal
        </button>
        <button
          class="button button--primary"
          type="submit"
          :disabled="
            submitting || (category === Category.Loan && exceedsTopUpPlafond)
          "
        >
          {{ submitting ? "Menyimpan…" : submitLabel }}
        </button>
      </div>
    </form>
  </UiModal>
</template>

<style scoped>
.legend {
  display: flex;
  gap: 14px;
  color: var(--muted);
  font-size: 10px;
}
.legend span {
  display: flex;
  align-items: center;
  gap: 6px;
}
.field-hint {
  margin: -6px 0 0;
  color: var(--soft);
  font-size: 10px;
}
.field-hint--warning {
  color: var(--amber);
}
.payoff-field {
  display: grid;
  gap: 10px;
  min-width: 0;
  margin: 0;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
}
.payoff-field legend {
  padding: 0 4px;
  color: #42504c;
  font-size: 10.5px;
  font-weight: 700;
}
.payoff-loan {
  display: grid;
  gap: 8px;
}
.payoff-loan__check {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.payoff-loan__check input {
  width: 15px;
  min-height: auto;
  margin: 2px 0 0;
}
.payoff-loan__check small {
  display: block;
  color: var(--soft);
  font-size: 10px;
  font-weight: 400;
}
.payment-loans {
  display: grid;
  gap: 12px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
}
.payment-loans__title {
  margin: 0;
  color: #42504c;
  font-size: 10.5px;
  font-weight: 700;
}
.payment-loan {
  display: grid;
  gap: 8px;
}
.payment-loan .field-hint {
  margin: 0;
}
.payment-loan + .payment-loan {
  padding-top: 12px;
  border-top: 1px solid var(--line);
}
.payment-loan__title {
  margin: 0;
  color: #42504c;
  font-size: 11px;
  font-weight: 700;
}
.field-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.field-row--three {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
.field-row--three .field-hint {
  margin: 0;
}
.savings-all {
  align-self: flex-start;
  justify-self: start;
}
.movement-field {
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}
.movement-field legend {
  margin-bottom: 7px;
  color: #42504c;
  font-size: 10.5px;
  font-weight: 700;
}
.movement-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 9px;
}
.movement-option {
  display: flex;
  align-items: center;
  min-height: 43px;
  gap: 9px;
  padding: 0 11px;
  border: 1px solid var(--line);
  border-radius: 9px;
  color: var(--muted);
  background: #fff;
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}
.movement-option.is-selected {
  border-color: #91acbd;
  color: var(--primary);
  background: rgba(1, 82, 156, 0.05);
}
.movement-option input {
  width: 15px;
  min-height: auto;
  margin: 0;
}
.movement-option svg {
  flex: 0 0 auto;
}
.calculation-preview {
  padding: 16px;
  border: 1px solid #beced9;
  border-radius: 11px;
  background: #f5fbf8;
}
.calculation-preview__title {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--green);
  font-size: 11px;
  font-weight: 700;
}
.calculation-preview dd {
  margin: 0;
  font-weight: 700;
}
@media (max-width: 540px) {
  .field-row {
    grid-template-columns: 1fr;
  }
}
</style>
