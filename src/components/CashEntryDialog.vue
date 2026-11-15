<script setup lang="ts">
import { computed, ref, watch } from "vue";

import {
  BanknoteArrowDown,
  BanknoteArrowUp,
  Calculator,
  HandCoins,
  ReceiptText,
  TriangleAlert,
  WalletMinimal,
} from "lucide-vue-next";

import { useCashForm } from "../composables/cash-entry/useCashForm";

import {
  guaranteeOptions,
  interestTypeOptions,
  loanGroupOptions,
  loanTypeOptions,
  useLoanForm,
} from "../composables/cash-entry/useLoanForm";

import { usePaymentForm } from "../composables/cash-entry/usePaymentForm";
import { useSavingsWithdrawalForm } from "../composables/cash-entry/useSavingsWithdrawalForm";
import type { WorkspacePageState } from "../composables/useWorkspacePageState";
import DateInput from "./DateInput.vue";
import MemberCombobox from "./MemberCombobox.vue";
import RupiahInput from "./RupiahInput.vue";
import UiModal from "./UiModal.vue";
import UiSelect from "./UiSelect.vue";

import {
  formatCurrency,
  formatLoanLabel,
  LoanGroup,
  todayIso,
  TransactionDirection,
  useKoperasiStore,
} from "../store/koperasi";

/** Catat transaksi: the single entry point for every Buku Kas movement. Each
 * kind of transaction keeps its own form (see composables/cash-entry); this
 * dialog picks one, holds the date and the Uraian, and submits it. */
const props = defineProps<{
  /** The page's per-tab state, so each workspace tab keeps its own draft. */
  pageState: WorkspacePageState;
}>();

const pageState = props.pageState;

const { financialParameters } = useKoperasiStore();

// Setoran simpanan is part of Setoran anggota: a member without a running loan
// deposits savings there too.
const Category = {
  Payment: "payment",
  Loan: "loan",
  SavingsWithdrawal: "savings-withdrawal",
  CashIn: "cash-in",
  CashOut: "cash-out",
} as const;

type Category = (typeof Category)[keyof typeof Category];
const modalOpen = pageState.field("modalOpen", () => false);
const submitting = ref(false);
const category = pageState.field<Category>("category", () => Category.Payment);

// A tab saved before the merge may still hold the old Setoran simpanan.
if (!Object.values(Category).includes(category.value))
  category.value = Category.Payment;

const posting = pageState.field("posting", () => ({
  businessDate: todayIso(),
}));

const isCash = computed(
  () =>
    category.value === Category.CashIn || category.value === Category.CashOut,
);

const isSavings = computed(() => category.value === Category.SavingsWithdrawal);

const payment = usePaymentForm(pageState, () => posting.value.businessDate);

const {
  paymentForm,
  isNonMemberPayment,
  paymentBorrowerOptions,
  paymentMemberOptions,
  paymentLoans,
  paymentLine,
  paymentTotal,
  interestDueLines,
  payoffInterestWarning,
} = payment;

// Every form picks from all members.
const memberOptions = paymentMemberOptions;

const loan = useLoanForm(pageState);

const {
  loanForm,
  loanProvision,
  loanPreview,
  topUpLoans,
  loanPayoff,
  payoffTotal,
  exceedsTopUpPlafond,
  borrowerOutstanding,
  outstandingAfterLoan,
  exceedsOutstandingLimit,
  loanAnnualRate,
} = loan;

const savings = useSavingsWithdrawalForm(pageState);

const {
  savingsForm,
  savingsTotal,
  savingsMemberBalances,
  withdrawAllSavings,
  setSavingsDefaults,
} = savings;

const cash = useCashForm(pageState, () =>
  category.value === Category.CashIn
    ? TransactionDirection.In
    : TransactionDirection.Out,
);

const { otherForm, cashCategoryOptions } = cash;
const isSavingsBook = computed(() => isCash.value && cash.isSavingsBook.value);

// Uraian of payments, loans, and savings follows the generated text (as the
// backend writes it) until the user types their own; empty falls back to it.
const typedDescription = pageState.field("typedDescription", () => ({
  text: "",
  edited: false,
}));

const generatedDescription = computed(
  () =>
    (
      ({
        [Category.Payment]: payment.generatedDescription.value,
        [Category.Loan]: loan.generatedDescription.value,
        [Category.SavingsWithdrawal]: savings.generatedDescription.value,
        [Category.CashIn]: "",
        [Category.CashOut]: "",
      }) satisfies Record<Category, string>
    )[category.value],
);

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

function openModal() {
  category.value = Category.Payment;
  posting.value.businessDate = todayIso();
  payment.reset();
  savings.reset();
  loan.reset();
  cash.reset();
  Object.assign(typedDescription.value, { text: "", edited: false });
  modalOpen.value = true;
}

const submitLabel = computed(
  () =>
    (
      ({
        [Category.Payment]: `Catat setoran ${formatCurrency(paymentTotal.value)}`,
        [Category.Loan]: "Catat realisasi",
        [Category.SavingsWithdrawal]: `Catat pengambilan ${formatCurrency(savingsTotal.value)}`,
        [Category.CashIn]: "Catat",
        [Category.CashOut]: "Catat",
      }) satisfies Record<Category, string>
    )[category.value],
);

async function submit() {
  const submittedModal = modalOpen.forTab(pageState.tabId.value);
  const options = { businessDate: posting.value.businessDate };
  const description = descriptionOverride.value;
  submitting.value = true;

  try {
    const saved = await (
      {
        [Category.Payment]: () => payment.submit(options, description),
        [Category.Loan]: () => loan.submit(options, description),
        [Category.SavingsWithdrawal]: () =>
          savings.submit(options, description),
        [Category.CashIn]: () => cash.submit(options),
        [Category.CashOut]: () => cash.submit(options),
      } satisfies Record<Category, () => Promise<boolean>>
    )[category.value]();

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
        <div class="movement-group" role="group" aria-label="Anggota">
          <p class="movement-group__label" aria-hidden="true">Anggota</p>
          <div class="movement-options movement-options--three">
            <label
              class="movement-option"
              :class="{ 'is-selected': category === Category.Payment }"
            >
              <input
                v-model="category"
                type="radio"
                :value="Category.Payment"
              />
              <ReceiptText :size="18" />
              <span>Setoran anggota</span>
            </label>
            <label
              class="movement-option"
              :class="{
                'is-selected': category === Category.SavingsWithdrawal,
              }"
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
              :class="{ 'is-selected': category === Category.Loan }"
            >
              <input v-model="category" type="radio" :value="Category.Loan" />
              <HandCoins :size="18" />
              <span>Realisasi pinjaman</span>
            </label>
          </div>
        </div>
        <div class="movement-group" role="group" aria-label="Kas lainnya">
          <p class="movement-group__label" aria-hidden="true">Kas lainnya</p>
          <div class="movement-options">
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
              <input
                v-model="category"
                type="radio"
                :value="Category.CashOut"
              />
              <BanknoteArrowUp :size="18" />
              <span>Kas keluar</span>
            </label>
          </div>
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
            v-model="paymentForm.borrower"
            :options="paymentBorrowerOptions"
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
            required
          />
        </label>
        <div v-if="!isNonMemberPayment" class="field-row field-row--three">
          <label class="field">
            <span>Simpanan pokok</span>
            <RupiahInput
              v-model="paymentForm.pokok"
              :min="0"
              aria-label="Simpanan pokok"
            />
          </label>
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
        <section v-if="paymentLoans.length" class="payment-loans">
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
        <div class="field-row field-row--three">
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
          <label class="field">
            <span>PG/TN</span>
            <UiSelect
              v-model="loanForm.guarantee"
              :options="guaranteeOptions"
              aria-label="PG/TN"
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
        <label class="field">
          <span>Rekomendasi</span>
          <input
            v-model="loanForm.recommender"
            placeholder="Nama yang merekomendasikan (boleh kosong)"
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
            <span>Bunga (% per tahun)</span>
            <input
              v-model.number="loanAnnualRate"
              type="number"
              inputmode="decimal"
              min="0"
              max="100"
              step="0.01"
              aria-label="Bunga pinjaman dalam persen per tahun"
              required
            />
          </label>
        </div>
        <div class="field-row">
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
        <div v-if="exceedsOutstandingLimit" class="limit-alert" role="alert">
          <div class="limit-alert__title">
            <TriangleAlert :size="20" />
            Melebihi batas pinjaman
            {{ formatCurrency(financialParameters.maxLoanOutstanding) }}
          </div>
          <dl>
            <div>
              <dt>Sisa pinjaman berjalan</dt>
              <dd>{{ formatCurrency(borrowerOutstanding) }}</dd>
            </div>
            <div>
              <dt>Setelah pinjaman ini</dt>
              <dd>{{ formatCurrency(outstandingAfterLoan) }}</dd>
            </div>
            <div>
              <dt>Lebih</dt>
              <dd>
                {{
                  formatCurrency(
                    outstandingAfterLoan -
                      financialParameters.maxLoanOutstanding,
                  )
                }}
              </dd>
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
.movement-group + .movement-group {
  margin-top: 12px;
}
.movement-group__label {
  margin: 0 0 7px;
  color: var(--soft);
  font-size: 9.5px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.movement-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 9px;
}
.movement-options--three {
  grid-template-columns: repeat(3, minmax(0, 1fr));
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
.limit-alert {
  display: grid;
  gap: 10px;
  padding: 14px 16px;
  border: 1px solid var(--red);
  border-radius: 11px;
  color: var(--red);
  background: var(--red-soft);
}
.limit-alert__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
}
.limit-alert dl {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin: 0;
  font-size: 11px;
}
.limit-alert dd {
  margin: 0;
  font-weight: 600;
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
  .limit-alert dl {
    grid-template-columns: 1fr;
  }
  .field-row,
  .movement-options--three {
    grid-template-columns: 1fr;
  }
}
</style>
