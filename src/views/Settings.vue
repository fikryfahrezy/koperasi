<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { CalendarRange, SlidersHorizontal } from "lucide-vue-next";
import DateInput from "../components/DateInput.vue";
import PageHeader from "../components/PageHeader.vue";
import RupiahInput from "../components/RupiahInput.vue";
import {
  todayIso,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

// Parameters kept in `parameters`: the form defaults (versioned by date) and
// the rates decided at the RAT for each book year.
const {
  financialParameters,
  selectedYear,
  setFinancialParameters,
  setIncentiveRate,
  setSittingAllowance,
} = useKoperasiStore();
const { monthlyLedger } = useMonthlyLedger();

// Fractions to percent, without floating-point noise (0.101 -> 10.1).
const toPercent = (rate: number) => Math.round(rate * 100_000) / 1000;

const form = reactive({
  effectiveDate: todayIso(),
  principalSavings: 0,
  mandatorySavings: 0,
  provisionRate: 0,
  annualRate: 0,
  maxLoanOutstanding: 0,
});

const resetForm = () => {
  form.effectiveDate = todayIso();
  form.principalSavings = financialParameters.principalSavings;
  form.mandatorySavings = financialParameters.mandatorySavings;
  form.provisionRate = financialParameters.provisionRate;
  form.annualRate = financialParameters.annualRate;
  form.maxLoanOutstanding = financialParameters.maxLoanOutstanding;
};

watch(() => ({ ...financialParameters }), resetForm, { immediate: true });

const formKeys = [
  "principalSavings",
  "mandatorySavings",
  "provisionRate",
  "annualRate",
  "maxLoanOutstanding",
] as const;

const formChanged = computed(() =>
  formKeys.some((key) => form[key] !== financialParameters[key]),
);

const savingForm = ref(false);

async function saveForm() {
  if (savingForm.value || !formChanged.value) return;
  savingForm.value = true;

  try {
    // Only what changed gets a new row from the date.
    const changed = Object.fromEntries(
      formKeys
        .filter((key) => form[key] !== financialParameters[key])
        .map((key) => [key, form[key]]),
    );

    // The form shows the values in force today (a later date waits).
    if (
      await setFinancialParameters({
        effectiveDate: form.effectiveDate,
        ...changed,
      })
    )
      resetForm();
  } finally {
    savingForm.value = false;
  }
}

const yearly = reactive({ loanRate: 0, savingsRate: 0, sittingAllowance: 0 });

const resetYearly = () => {
  yearly.loanRate = toPercent(monthlyLedger.value.loanIncentiveRate);
  yearly.savingsRate = toPercent(monthlyLedger.value.savingsIncentiveRate);
  yearly.sittingAllowance = monthlyLedger.value.sittingAllowance;
};

watch(monthlyLedger, resetYearly, { immediate: true });

const yearlyChanged = computed(
  () =>
    yearly.loanRate !== toPercent(monthlyLedger.value.loanIncentiveRate) ||
    yearly.savingsRate !==
      toPercent(monthlyLedger.value.savingsIncentiveRate) ||
    yearly.sittingAllowance !== monthlyLedger.value.sittingAllowance,
);

const savingYearly = ref(false);

async function saveYearly() {
  if (savingYearly.value || !yearlyChanged.value) return;
  savingYearly.value = true;

  try {
    const year = selectedYear.value;
    // Read every change first: each save refetches the ledger, and the watch
    // above then resets `yearly` to the saved values.
    const { loanRate, savingsRate, sittingAllowance } = yearly;
    const saved = monthlyLedger.value;

    if (loanRate !== toPercent(saved.loanIncentiveRate))
      await setIncentiveRate("loan", year, loanRate);
    if (savingsRate !== toPercent(saved.savingsIncentiveRate))
      await setIncentiveRate("savings", year, savingsRate);
    if (sittingAllowance !== saved.sittingAllowance)
      await setSittingAllowance(year, sittingAllowance);
  } finally {
    savingYearly.value = false;
  }
}
</script>

<template>
  <div class="page-stack settings-page">
    <PageHeader title="Pengaturan" />

    <form class="panel settings-card" @submit.prevent="saveForm">
      <header class="settings-card__head">
        <span class="settings-card__icon"
          ><SlidersHorizontal :size="20"
        /></span>
        <div>
          <h2>Nilai awal transaksi</h2>
          <p>
            Simpanan pokok menjadi batas minimal di form Tambah anggota;
            provisi, bunga, dan batas sisa pinjaman menjadi isian awal dan
            peringatan di form Realisasi pinjaman. Nilai baru berlaku sejak
            tanggal yang dipilih; transaksi yang sudah dicatat tidak berubah.
            <template v-if="financialParameters.effectiveDate">
              Nilai sekarang berlaku sejak
              {{ financialParameters.effectiveDate }}.
            </template>
          </p>
        </div>
      </header>

      <div class="settings-grid">
        <label class="field"
          ><span>Simpanan pokok (minimal anggota baru)</span
          ><RupiahInput
            v-model="form.principalSavings"
            :min="0"
            aria-label="Simpanan pokok"
        /></label>
        <label class="field"
          ><span>Simpanan wajib per bulan (belum dipakai di form)</span
          ><RupiahInput
            v-model="form.mandatorySavings"
            :min="0"
            aria-label="Simpanan wajib per bulan"
        /></label>
        <label class="field"
          ><span>Provisi (%)</span
          ><input
            v-model.number="form.provisionRate"
            type="number"
            min="0"
            max="100"
            step="0.01"
            required
            aria-label="Provisi"
        /></label>
        <label class="field"
          ><span>Bunga pinjaman per tahun (%)</span
          ><input
            v-model.number="form.annualRate"
            type="number"
            min="0"
            max="100"
            step="0.01"
            required
            aria-label="Bunga pinjaman per tahun"
        /></label>
        <label class="field"
          ><span>Batas sisa pinjaman per peminjam (0 = tanpa batas)</span
          ><RupiahInput
            v-model="form.maxLoanOutstanding"
            :min="0"
            aria-label="Batas sisa pinjaman per peminjam"
        /></label>
        <div class="field">
          <span>Berlaku sejak</span>
          <DateInput
            v-model="form.effectiveDate"
            required
            aria-label="Berlaku sejak"
          />
        </div>
      </div>

      <div class="settings-actions">
        <button
          class="button button--secondary"
          type="button"
          :disabled="!formChanged || savingForm"
          @click="resetForm"
        >
          Batal
        </button>
        <button
          class="button button--primary"
          type="submit"
          :disabled="!formChanged || savingForm"
        >
          {{ savingForm ? "Menyimpan…" : "Simpan" }}
        </button>
      </div>
    </form>

    <form class="panel settings-card" @submit.prevent="saveYearly">
      <header class="settings-card__head">
        <span class="settings-card__icon settings-card__icon--amber"
          ><CalendarRange :size="20"
        /></span>
        <div>
          <h2>Keputusan RAT tahun buku {{ selectedYear }}</h2>
          <p>
            Dipakai untuk Insentif di halaman Pinjaman dan Simpanan, dan uang
            duduk di Daftar SHU. Tahun berikutnya memakai nilai terakhir sampai
            diubah. Ganti tahun buku di atas untuk mengubah tahun lain.
          </p>
        </div>
      </header>

      <div class="settings-grid">
        <label class="field"
          ><span>Insentif pinjaman (% dari bunga + provisi)</span
          ><input
            v-model.number="yearly.loanRate"
            type="number"
            min="0"
            max="100"
            step="0.01"
            required
            aria-label="Insentif pinjaman"
        /></label>
        <label class="field"
          ><span>Insentif simpanan (% dari Ratas Simpanan)</span
          ><input
            v-model.number="yearly.savingsRate"
            type="number"
            min="0"
            max="100"
            step="0.01"
            required
            aria-label="Insentif simpanan"
        /></label>
        <label class="field"
          ><span>Uang duduk + uang makan per anggota</span
          ><RupiahInput
            v-model="yearly.sittingAllowance"
            :min="0"
            aria-label="Uang duduk + uang makan per anggota"
        /></label>
      </div>

      <div class="settings-actions">
        <button
          class="button button--secondary"
          type="button"
          :disabled="!yearlyChanged || savingYearly"
          @click="resetYearly"
        >
          Batal
        </button>
        <button
          class="button button--primary"
          type="submit"
          :disabled="!yearlyChanged || savingYearly"
        >
          {{ savingYearly ? "Menyimpan…" : "Simpan" }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 820px;
  gap: 14px;
}
.settings-card {
  display: grid;
  gap: 16px;
  padding: 18px 20px;
}
.settings-card__head {
  display: flex;
  gap: 16px;
}
.settings-card__icon {
  flex: 0 0 auto;
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  color: var(--green);
  background: var(--green-soft);
}
.settings-card__icon--amber {
  color: var(--amber);
  background: var(--amber-soft);
}
.settings-card h2 {
  margin: 0 0 4px;
  font-size: 14px;
}
.settings-card p {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.5;
}
.settings-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px 16px;
}
.settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
