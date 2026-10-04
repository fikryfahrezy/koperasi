<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  BanknoteArrowDown,
  BanknoteArrowUp,
  Calculator,
  CalendarX2,
  Coins,
  HandCoins,
  Plus,
  ReceiptText,
  Search,
  Undo2,
  WalletMinimal,
} from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import DateInput from "../components/DateInput.vue";
import RupiahInput from "../components/RupiahInput.vue";
import UiModal from "../components/UiModal.vue";
import UiSelect from "../components/UiSelect.vue";
import {
  cashCategories,
  Channel,
  formatCurrency,
  formatSheetNumber,
  InterestType,
  LoanGroup,
  LoanType,
  type CashCategory,
  type CashBookRow,
  type LoanPreview,
  SavingsAccountType,
  SavingsMovement,
  todayIso,
  TransactionDirection,
  TransactionStatus,
  useKoperasiStore,
  useCashBook,
} from "../store/koperasi";

const pageState = useWorkspacePageState("/cash-ledger");

const {
  members,
  loans,
  selectedYear,
  financialParameters,
  createLoan,
  loadFinancialParameters,
  notify,
  postCashEntry,
  postPayment,
  postSavingsTransaction,
  previewLoan,
  reverseTransaction,
  refresh,
} = useKoperasiStore();

// --- Cash ledger (following the daily cash ledger spreadsheet) ----------------------------
const { cashBook, loadingBook } = useCashBook(Channel.Cash);
const selected = pageState.field<CashBookRow | null>("selected", () => null);

onMounted(() => {
  void loadFinancialParameters();
});

const rows = computed(() => cashBook.value?.rows ?? []);
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
const filteredRows = computed(() => {
  const search = debouncedQuery.value.trim().toLowerCase();
  return rows.value.filter((row) =>
    row.description.toLowerCase().includes(search),
  );
});
// As in the spreadsheet, the date appears only on the first row of each day.
const showDate = (index: number) =>
  index === 0 ||
  filteredRows.value[index - 1].date !== filteredRows.value[index].date;
const isExcluded = (item: CashBookRow) =>
  item.status === TransactionStatus.Reversed || item.isReversal;

// --- Transaction entry form ---------------------------------------------------
const Category = {
  Payment: "payment",
  Loan: "loan",
  SavingsDeposit: "savings-deposit",
  SavingsWithdrawal: "savings-withdrawal",
  CashIn: "cash-in",
  CashOut: "cash-out",
} as const;
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
  tenor: 24,
  interestType: InterestType.Declining as InterestType,
  loanType: LoanType.Monthly as LoanType,
}));
const otherForm = pageState.field("otherForm", () => ({
  category: "" as CashCategory | "",
  description: "",
  amount: 0,
}));
const loanPreview = pageState.field<LoanPreview | null>(
  "loanPreview",
  () => null,
);

const memberOptions = computed(() =>
  members.map((member) => ({ value: member.id, label: member.name })),
);
const savingsAccountOptions = [
  { value: SavingsAccountType.Principal, label: "Pokok" },
  { value: SavingsAccountType.Mandatory, label: "Wajib" },
  { value: SavingsAccountType.Voluntary, label: "Manasuka" },
];
const tenorOptions = [10, 12, 15, 20, 24, 36].map((tenor) => ({
  value: tenor,
  label: `${tenor} bulan`,
}));
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
const cashCategoryOptions = [
  { value: "", label: "Tanpa kategori" },
  ...cashCategories,
];
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
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    try {
      const result = await previewLoan({ ...loanForm.value });
      if (!cancelled) target.value = result;
    } catch {
      if (!cancelled) target.value = null;
    }
  },
  { deep: true, immediate: true },
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
    tenor: 24,
    interestType: InterestType.Declining,
    loanType: LoanType.Monthly,
  });
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
const isCash = computed(
  () =>
    category.value === Category.CashIn || category.value === Category.CashOut,
);
// Only manasuka can be withdrawn, so a withdrawal always starts from it.
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
      if (borrowerMissing || loanForm.value.plafond <= 0) return;
      saved = await createLoan(
        {
          ...loanForm.value,
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
      saved = await postCashEntry(
        {
          direction:
            category.value === Category.CashIn
              ? TransactionDirection.In
              : TransactionDirection.Out,
          category: otherForm.value.category,
          description: otherForm.value.description.trim(),
          amount: otherForm.value.amount,
          reference: "",
        },
        options,
      );
    }
    if (saved) submittedModal.value = false;
  } finally {
    submitting.value = false;
  }
}

async function reverseSelected() {
  if (!selected.value) return;
  const submittedSelection = selected.forTab(pageState.tabId.value);
  const reversed = await reverseTransaction(submittedSelection.value!.id);
  if (reversed) submittedSelection.value = null;
}
async function refreshPage() {
  await refresh();
  await loadFinancialParameters();
}
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Buku kas harian" :refresh="refreshPage">
      <template #before-actions>
        <label class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            type="search"
            aria-label="Cari uraian transaksi"
            placeholder="Cari uraian..."
          />
        </label>
      </template>
      <template #actions>
        <button class="button button--primary" type="button" @click="openModal">
          <Plus :size="18" /> Catat transaksi
        </button>
      </template>
    </PageHeader>

    <section class="panel table-panel">
      <div
        v-if="!loadingBook && !rows.length && !cashBook?.openingBalance"
        class="year-empty-state year-empty-state--inline"
      >
        <CalendarX2 :size="36" />
        <strong>Belum ada transaksi kas untuk {{ selectedYear }}</strong>
        <p>Catat transaksi baru dengan tombol Catat transaksi.</p>
      </div>
      <div v-else class="data-table-wrap sheet-wrap">
        <table v-table-navigation class="sheet-table cash-book-table">
          <thead>
            <tr>
              <th rowspan="2">Tanggal</th>
              <th rowspan="2" class="cash-book-table__desc">Uraian</th>
              <th colspan="2" class="sheet-group">Mutasi</th>
              <th rowspan="2" class="sheet-total">Jumlah Saldo Kas</th>
            </tr>
            <tr>
              <th>Debet</th>
              <th>Kredit</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td></td>
              <td class="cash-book-table__desc">
                Saldo Kas Fisik Neraca Posisi 31 Desember {{ selectedYear - 1 }}
              </td>
              <td></td>
              <td></td>
              <td class="num-cell sheet-total">
                <strong>{{
                  formatSheetNumber(cashBook?.openingBalance ?? 0)
                }}</strong>
              </td>
            </tr>
            <tr
              v-for="(item, index) in filteredRows"
              :key="item.id"
              class="cash-book-table__row"
              :class="{ 'is-excluded': isExcluded(item) }"
              @click="selected = item"
            >
              <td>{{ showDate(index) ? item.date : "" }}</td>
              <td class="cash-book-table__desc">{{ item.description }}</td>
              <td class="num-cell">
                {{
                  item.direction === TransactionDirection.Out
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td class="num-cell">
                {{
                  item.direction === TransactionDirection.In
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td class="num-cell sheet-total">
                {{ formatSheetNumber(item.balance) }}
              </td>
            </tr>
            <tr
              v-if="
                !loadingBook && debouncedQuery.trim() && !filteredRows.length
              "
            >
              <td colspan="5">
                Tidak ada transaksi yang cocok dengan pencarian.
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td colspan="2">
                {{ debouncedQuery.trim() ? "JUMLAH TAHUNAN" : "JUMLAH" }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashBook?.totalOut ?? 0) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashBook?.totalIn ?? 0) }}
              </td>
              <td class="num-cell sheet-total">
                {{ formatSheetNumber(cashBook?.closingBalance ?? 0) }}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>

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
            <label class="movement-option">
              <input
                v-model="category"
                type="radio"
                :value="Category.Payment"
              />
              <ReceiptText :size="18" />
              <span>Setoran anggota</span>
            </label>
            <label class="movement-option">
              <input v-model="category" type="radio" :value="Category.Loan" />
              <HandCoins :size="18" />
              <span>Realisasi pinjaman</span>
            </label>
            <label class="movement-option">
              <input
                v-model="category"
                type="radio"
                :value="Category.SavingsDeposit"
                @change="setSavingsDefaults"
              />
              <Coins :size="18" />
              <span>Setoran simpanan</span>
            </label>
            <label class="movement-option">
              <input
                v-model="category"
                type="radio"
                :value="Category.SavingsWithdrawal"
                @change="setSavingsDefaults"
              />
              <WalletMinimal :size="18" />
              <span>Pengambilan simpanan</span>
            </label>
            <label class="movement-option">
              <input v-model="category" type="radio" :value="Category.CashIn" />
              <BanknoteArrowDown :size="18" />
              <span>Kas masuk</span>
            </label>
            <label class="movement-option">
              <input
                v-model="category"
                type="radio"
                :value="Category.CashOut"
              />
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
            <UiSelect
              v-model="paymentForm.memberId"
              :options="memberOptions"
              aria-label="Anggota"
              placeholder="Pilih anggota"
              required
            />
          </label>
          <p
            v-if="
              isNonMemberPayment ? paymentForm.loanId : paymentForm.memberId
            "
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
            <UiSelect
              v-model="savingsForm.memberId"
              :options="memberOptions"
              aria-label="Anggota"
              placeholder="Pilih anggota"
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
                :disabled="category === Category.SavingsWithdrawal"
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
            <UiSelect
              v-model="loanForm.memberId"
              :options="memberOptions"
              aria-label="Anggota"
              placeholder="Pilih anggota"
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
              <span>Jangka waktu</span>
              <UiSelect
                v-model="loanForm.tenor"
                :options="tenorOptions"
                aria-label="Jangka waktu"
              />
            </label>
          </div>
          <label class="field">
            <span>Jenis bunga</span>
            <UiSelect
              v-model="loanForm.interestType"
              :options="interestTypeOptions"
              aria-label="Jenis bunga"
            />
          </label>
          <div class="calculation-preview">
            <div class="calculation-preview__title">
              <Calculator :size="19" /> Dicatat: realisasi
              {{ formatCurrency(loanForm.plafond) }} (debet) dan provisi
              (kredit)
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
                <dd>{{ formatCurrency(loanPreview?.provision ?? 0) }}</dd>
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

    <UiModal
      :open="Boolean(selected) && pageState.active.value"
      title="Detail transaksi"
      @close="selected = null"
      ><div v-if="selected" class="transaction-detail">
        <div class="transaction-detail__head">
          <div>
            <strong>{{ selected.description }}</strong
            ><span>{{ selected.date }}</span>
          </div>
          <strong>{{ formatCurrency(selected.amount) }}</strong>
        </div>
        <div class="component-list">
          <div v-for="(component, index) in selected.components" :key="index">
            <span>{{ component.label }}</span
            ><strong>{{ formatCurrency(component.amount) }}</strong>
          </div>
        </div>
        <div v-if="isExcluded(selected)" class="audit-note">
          {{
            selected.isReversal
              ? "Baris ini adalah reversal."
              : "Transaksi ini sudah dibalik."
          }}
        </div>
        <div class="modal-actions">
          <button
            class="button button--danger"
            :disabled="isExcluded(selected)"
            @click="reverseSelected"
          >
            <Undo2 :size="17" /> Buat reversal</button
          ><button class="button button--primary" @click="selected = null">
            Selesai
          </button>
        </div>
      </div></UiModal
    >
  </div>
</template>
