<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Calculator,
  CalendarX2,
  Coins,
  HandCoins,
  Plus,
  ReceiptText,
  Undo2,
  Wallet,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import RupiahInput from "../components/RupiahInput.vue";
import UiModal from "../components/UiModal.vue";
import UiSelect from "../components/UiSelect.vue";
import {
  Channel,
  formatCurrency,
  formatSheetNumber,
  InterestType,
  LoanStatus,
  type CashBook,
  type CashBookRow,
  type LoanPreview,
  SavingsAccountType,
  SavingsMovement,
  todayIso,
  TransactionDirection,
  TransactionStatus,
  useKoperasiStore,
} from "../store/koperasi";

const {
  members,
  loans,
  selectedYear,
  selectedCompanyId,
  snapshotVersion,
  financialParameters,
  createLoan,
  getCashBook,
  loadFinancialParameters,
  notify,
  postCashEntry,
  postPayment,
  postSavingsTransaction,
  previewLoan,
  reverseTransaction,
  refresh,
} = useKoperasiStore();

// --- Buku kas (mengikuti sheet BUKU KAS HARIAN) ----------------------------
const cashBook = ref<CashBook | null>(null);
const loadingBook = ref(false);
const selected = ref<CashBookRow | null>(null);

let bookRequest = 0;
async function loadCashBook() {
  const request = ++bookRequest;
  loadingBook.value = true;
  try {
    const book = await getCashBook(selectedYear.value, Channel.Cash);
    if (request === bookRequest) cashBook.value = book;
  } catch (error) {
    if (request === bookRequest) {
      cashBook.value = null;
      notify("Buku kas gagal dimuat", String(error), "warning");
    }
  } finally {
    if (request === bookRequest) loadingBook.value = false;
  }
}
watch([selectedYear, selectedCompanyId, snapshotVersion], loadCashBook);
onMounted(() => {
  void loadCashBook();
  void loadFinancialParameters();
});

const rows = computed(() => cashBook.value?.rows ?? []);
// Seperti di sheet, tanggal hanya ditulis pada baris pertama tiap hari.
const showDate = (index: number) =>
  index === 0 || rows.value[index - 1].date !== rows.value[index].date;
const isExcluded = (item: CashBookRow) =>
  item.status === TransactionStatus.Reversed ||
  item.transactionType === "REVERSAL";

// --- Form catat transaksi ---------------------------------------------------
type Category = "payment" | "savings" | "loan" | "other";
const modalOpen = ref(false);
const submitting = ref(false);
const category = ref<Category>("payment");
const posting = reactive({ businessDate: todayIso() });
const paymentForm = reactive({
  memberId: "",
  principal: 0,
  interest: 0,
  wajib: 50_000,
  voluntary: 0,
});
const savingsForm = reactive({
  memberId: "",
  accountType: SavingsAccountType.Mandatory as SavingsAccountType,
  movement: SavingsMovement.Deposit as SavingsMovement,
  amount: 50_000,
});
const loanForm = reactive({
  memberId: "",
  plafond: 10_000_000,
  tenor: 24,
  interestType: InterestType.Declining as InterestType,
});
const otherForm = reactive({
  direction: TransactionDirection.Out as TransactionDirection,
  description: "",
  amount: 0,
});
const loanPreview = ref<LoanPreview | null>(null);

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
  { value: InterestType.Flat, label: "Flat" },
];
// Saldo pinjaman berjalan anggota terpilih untuk membantu mengisi angsuran.
const paymentLoans = computed(() =>
  loans.filter(
    (loan) =>
      loan.memberId === paymentForm.memberId &&
      loan.status !== LoanStatus.Draft &&
      loan.balance > 0,
  ),
);
const paymentLoanBalance = computed(() =>
  paymentLoans.value.reduce((sum, loan) => sum + loan.balance, 0),
);
watch(
  () => paymentForm.memberId,
  () => {
    const loan = paymentLoans.value[0];
    paymentForm.principal = loan
      ? Math.min(
          Math.ceil(loan.plafond / loan.tenor / 1000) * 1000,
          paymentLoanBalance.value,
        )
      : 0;
    paymentForm.interest = loan
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
    (paymentForm.principal || 0) +
    (paymentForm.interest || 0) +
    (paymentForm.wajib || 0) +
    (paymentForm.voluntary || 0),
);

watch(
  loanForm,
  async () => {
    try {
      loanPreview.value = await previewLoan({ ...loanForm });
    } catch {
      loanPreview.value = null;
    }
  },
  { deep: true, immediate: true },
);

function openModal() {
  category.value = "payment";
  posting.businessDate = todayIso();
  Object.assign(paymentForm, {
    memberId: "",
    principal: 0,
    interest: 0,
    wajib: financialParameters.mandatorySavings || 50_000,
    voluntary: 0,
  });
  Object.assign(savingsForm, {
    memberId: "",
    accountType: SavingsAccountType.Mandatory,
    movement: SavingsMovement.Deposit,
    amount: financialParameters.mandatorySavings || 50_000,
  });
  Object.assign(loanForm, {
    memberId: "",
    plafond: 10_000_000,
    tenor: 24,
    interestType: InterestType.Declining,
  });
  Object.assign(otherForm, {
    direction: TransactionDirection.Out,
    description: "",
    amount: 0,
  });
  modalOpen.value = true;
}
function setSavingsMovementDefaults() {
  savingsForm.accountType =
    savingsForm.movement === SavingsMovement.Withdrawal
      ? SavingsAccountType.Voluntary
      : SavingsAccountType.Mandatory;
}

const submitLabel = computed(
  () =>
    ({
      payment: `Catat setoran ${formatCurrency(paymentTotal.value)}`,
      savings: `Catat ${savingsForm.movement.toLowerCase()}`,
      loan: "Catat pencairan",
      other: "Catat",
    })[category.value],
);

async function submit() {
  const options = { businessDate: posting.businessDate };
  submitting.value = true;
  try {
    let saved = false;
    if (category.value === "payment") {
      if (!paymentForm.memberId || paymentTotal.value <= 0) return;
      saved = await postPayment(
        {
          memberId: paymentForm.memberId,
          principal: paymentForm.principal || 0,
          interest: paymentForm.interest || 0,
          wajib: paymentForm.wajib || 0,
          voluntary: paymentForm.voluntary || 0,
          reference: "",
        },
        options,
      );
    } else if (category.value === "savings") {
      if (!savingsForm.memberId || savingsForm.amount <= 0) return;
      saved = await postSavingsTransaction(
        { ...savingsForm, reference: "" },
        options,
      );
    } else if (category.value === "loan") {
      if (!loanForm.memberId || loanForm.plafond <= 0) return;
      saved = await createLoan({ ...loanForm }, options);
    } else {
      if (otherForm.amount <= 0) return;
      if (!otherForm.description.trim()) return;
      saved = await postCashEntry(
        {
          direction: otherForm.direction,
          category: otherForm.description.trim(),
          description: otherForm.description.trim(),
          amount: otherForm.amount,
          reference: "",
        },
        options,
      );
    }
    if (saved) modalOpen.value = false;
  } finally {
    submitting.value = false;
  }
}

async function reverseSelected() {
  if (!selected.value) return;
  const reversed = await reverseTransaction(selected.value.id);
  if (reversed) selected.value = null;
}
async function refreshPage() {
  await Promise.all([refresh(), loadFinancialParameters()]);
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Buku kas harian" :refresh="refreshPage">
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
        <table class="sheet-table cash-book-table">
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
              v-for="(item, index) in rows"
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
          </tbody>
          <tfoot>
            <tr>
              <td colspan="2">JUMLAH</td>
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
      :open="modalOpen"
      size="lg"
      title="Catat transaksi"
      description="Transaksi tidak dapat diedit; koreksi dilakukan dengan reversal."
      @close="modalOpen = false"
    >
      <form class="form-stack" @submit.prevent="submit">
        <fieldset class="movement-field">
          <legend>Jenis transaksi</legend>
          <div class="movement-options movement-options--four">
            <label class="movement-option">
              <input v-model="category" type="radio" value="payment" />
              <ReceiptText :size="18" />
              <span>Setoran anggota</span>
            </label>
            <label class="movement-option">
              <input v-model="category" type="radio" value="savings" />
              <Coins :size="18" />
              <span>Simpanan</span>
            </label>
            <label class="movement-option">
              <input v-model="category" type="radio" value="loan" />
              <HandCoins :size="18" />
              <span>Pencairan pinjaman</span>
            </label>
            <label class="movement-option">
              <input v-model="category" type="radio" value="other" />
              <Wallet :size="18" />
              <span>Kas lainnya</span>
            </label>
          </div>
        </fieldset>

        <label class="field">
          <span>Tanggal</span>
          <input v-model="posting.businessDate" type="date" required />
        </label>

        <template v-if="category === 'payment'">
          <label class="field">
            <span>Anggota</span>
            <UiSelect
              v-model="paymentForm.memberId"
              :options="memberOptions"
              aria-label="Anggota"
              placeholder="Pilih anggota"
              required
            />
          </label>
          <p v-if="paymentForm.memberId" class="field-hint">
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
          <div class="field-row">
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

        <template v-else-if="category === 'savings'">
          <fieldset class="movement-field">
            <legend>Arah</legend>
            <div class="movement-options">
              <label class="movement-option">
                <input
                  v-model="savingsForm.movement"
                  type="radio"
                  :value="SavingsMovement.Deposit"
                  @change="setSavingsMovementDefaults"
                />
                <ArrowDownToLine :size="18" />
                <span>Setoran</span>
              </label>
              <label class="movement-option">
                <input
                  v-model="savingsForm.movement"
                  type="radio"
                  :value="SavingsMovement.Withdrawal"
                  @change="setSavingsMovementDefaults"
                />
                <ArrowUpFromLine :size="18" />
                <span>Pengambilan</span>
              </label>
            </div>
          </fieldset>
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
                :disabled="savingsForm.movement === SavingsMovement.Withdrawal"
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

        <template v-else-if="category === 'loan'">
          <label class="field">
            <span>Anggota</span>
            <UiSelect
              v-model="loanForm.memberId"
              :options="memberOptions"
              aria-label="Anggota"
              placeholder="Pilih anggota"
              required
            />
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

        <template v-else>
          <fieldset class="movement-field">
            <legend>Mutasi</legend>
            <div class="movement-options">
              <label class="movement-option">
                <input
                  v-model="otherForm.direction"
                  type="radio"
                  :value="TransactionDirection.Out"
                />
                <ArrowUpFromLine :size="18" />
                <span>Debet (keluar)</span>
              </label>
              <label class="movement-option">
                <input
                  v-model="otherForm.direction"
                  type="radio"
                  :value="TransactionDirection.In"
                />
                <ArrowDownToLine :size="18" />
                <span>Kredit (masuk)</span>
              </label>
            </div>
          </fieldset>
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
      :open="Boolean(selected)"
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
            selected.transactionType === "REVERSAL"
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
