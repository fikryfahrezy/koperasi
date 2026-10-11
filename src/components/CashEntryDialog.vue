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
  InterestType,
  LoanGroup,
  LoanType,
  type LoanPreview,
  SavingsAccountType,
  SavingsMovement,
  todayIso,
  TransactionDirection,
  useKoperasiStore,
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
  wajib: 50_000,
  voluntary: 0,
}));
const savingsForm = pageState.field("savingsForm", () => ({
  memberId: "",
  accountType: SavingsAccountType.Mandatory as SavingsAccountType,
  amount: 50_000,
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
// Provisi follows the parameter rate until the user types their own amount.
const loanProvision = pageState.field("loanProvision", () => ({
  amount: 0,
  edited: false,
}));
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
const savingsAccountOptions = [
  { value: SavingsAccountType.Principal, label: "Pokok" },
  { value: SavingsAccountType.Mandatory, label: "Wajib" },
  { value: SavingsAccountType.Voluntary, label: "Manasuka" },
];
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
// Non Anggota borrowers have no member, so their payment picks the loan itself.
const nonMemberLoanOptions = computed(() =>
  loans
    .filter(
      (loan) => loan.loanGroup === LoanGroup.NonMember && loan.balance > 0,
    )
    .map((loan) => ({
      value: loan.id,
      label: `${loan.memberName} · ${formatCurrency(loan.plafond)} · ${loan.realizationDate}`,
    })),
);
// The outstanding loan balance helps fill in the installment.
const paymentLoans = computed(() =>
  loans.filter((loan) =>
    isNonMemberPayment.value
      ? loan.id === paymentForm.value.loanId
      : loan.memberId === paymentForm.value.memberId && loan.balance > 0,
  ),
);
const paymentLoanBalance = computed(() =>
  paymentLoans.value.reduce((sum, loan) => sum + loan.balance, 0),
);
watch(
  [
    () => pageState.tabId.value,
    () => paymentForm.value.memberId,
    () => paymentForm.value.loanId,
    () => paymentForm.value.loanGroup,
  ],
  ([tabId], [previousTabId]) => {
    if (tabId !== previousTabId) return;
    const loan = paymentLoans.value[0];
    paymentForm.value.principal = loan
      ? Math.min(
          Math.ceil(loan.plafond / loan.tenor / 1000) * 1000,
          paymentLoanBalance.value,
        )
      : 0;
    paymentForm.value.interest = loan
      ? Math.round(
          ((loan.interestType === InterestType.Flat
            ? loan.plafond
            : loan.balance) *
            loan.rate) /
            100 /
            12,
        )
      : 0;
  },
);
const paymentTotal = computed(
  () =>
    (paymentForm.value.principal || 0) +
    (paymentForm.value.interest || 0) +
    (isNonMemberPayment.value
      ? 0
      : (paymentForm.value.wajib || 0) + (paymentForm.value.voluntary || 0)),
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
      if (!provision.value.edited) provision.value.amount = result.provision;
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
    wajib: financialParameters.mandatorySavings || 50_000,
    voluntary: 0,
  });
  Object.assign(savingsForm.value, {
    memberId: "",
    accountType: SavingsAccountType.Mandatory,
    amount: financialParameters.mandatorySavings || 50_000,
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
  Object.assign(otherForm.value, {
    category: "",
    description: "",
    amount: 0,
  });
  modalOpen.value = true;
}
const isSavings = computed(
  () =>
    category.value === Category.SavingsDeposit ||
    category.value === Category.SavingsWithdrawal,
);
// Withdrawals usually come from manasuka, so they start from it.
function setSavingsDefaults() {
  savingsForm.value.accountType =
    category.value === Category.SavingsWithdrawal
      ? SavingsAccountType.Voluntary
      : SavingsAccountType.Mandatory;
}

const submitLabel = computed(
  () =>
    (
      ({
        [Category.Payment]: `Catat setoran ${formatCurrency(paymentTotal.value)}`,
        [Category.Loan]: "Catat realisasi",
        [Category.SavingsDeposit]: "Catat setoran",
        [Category.SavingsWithdrawal]: "Catat pengambilan",
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
      const nonMember = isNonMemberPayment.value;
      const borrowerMissing = nonMember
        ? !paymentForm.value.loanId
        : !paymentForm.value.memberId;
      if (borrowerMissing || paymentTotal.value <= 0) return;
      saved = await postPayment(
        {
          memberId: nonMember ? "" : paymentForm.value.memberId,
          loanId: nonMember ? paymentForm.value.loanId : "",
          principal: paymentForm.value.principal || 0,
          interest: paymentForm.value.interest || 0,
          wajib: nonMember ? 0 : paymentForm.value.wajib || 0,
          voluntary: nonMember ? 0 : paymentForm.value.voluntary || 0,
          reference: "",
        },
        options,
      );
    } else if (isSavings.value) {
      if (!savingsForm.value.memberId || savingsForm.value.amount <= 0) return;
      saved = await postSavingsTransaction(
        {
          ...savingsForm.value,
          movement:
            category.value === Category.SavingsWithdrawal
              ? SavingsMovement.Withdrawal
              : SavingsMovement.Deposit,
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
        !Number.isFinite(loanProvision.value.amount)
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
          <UiSelect
            v-model="paymentForm.loanId"
            :options="nonMemberLoanOptions"
            aria-label="Peminjam"
            placeholder="Pilih pinjaman"
            required
          />
        </label>
        <label v-else class="field">
          <span>Anggota</span>
          <MemberCombobox
            v-model="paymentForm.memberId"
            :options="memberOptions"
            aria-label="Anggota"
            placeholder="Cari nama atau ID anggota"
            required
          />
        </label>
        <p
          v-if="isNonMemberPayment ? paymentForm.loanId : paymentForm.memberId"
          class="field-hint"
        >
          {{
            paymentLoans.length
              ? `Sisa pokok pinjaman ${formatCurrency(paymentLoanBalance)} (${paymentLoans.length} pinjaman)`
              : "Tidak ada pinjaman berjalan."
          }}
        </p>
        <div class="field-row">
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
        <div class="field-row">
          <label class="field">
            <span>Jenis simpanan</span>
            <UiSelect
              v-model="savingsForm.accountType"
              :options="savingsAccountOptions"
              aria-label="Jenis simpanan"
            />
          </label>
          <label class="field">
            <span>Nominal</span>
            <RupiahInput
              v-model="savingsForm.amount"
              :min="1000"
              min-message="Nominal transaksi minimal Rp1.000."
              aria-label="Nominal transaksi dalam Rupiah"
              required
            />
          </label>
        </div>
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
        <label v-else class="field">
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
            <Calculator :size="19" /> Dicatat: realisasi
            {{ formatCurrency(loanForm.plafond) }} (debet) dan provisi (kredit)
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
          </dl>
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
          :disabled="submitting"
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
.field-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
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
