<script setup lang="ts">
import { computed, ref, watch } from "vue";
import UiModal from "./UiModal.vue";
import {
  type IncentiveKind,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

// Insentif rates are decided at the RAT for each book year, so they are edited
// per year: Insentif Pinjaman on Pinjaman, Insentif Simpanan on Simpanan.
const props = defineProps<{ kind: IncentiveKind }>();
const open = defineModel<boolean>("open", { required: true });
const { selectedYear, setIncentiveRate } = useKoperasiStore();
const { monthlyLedger } = useMonthlyLedger();
const percent = ref(0);
const saving = ref(false);

const label = computed(() =>
  props.kind === "loan"
    ? "Insentif pinjaman (% dari bunga + provisi)"
    : "Insentif simpanan (% dari Ratas Simpanan)",
);
// Fractions to percent, without floating-point noise (0.101 -> 10.1).
const toPercent = (rate: number) => Math.round(rate * 100_000) / 1000;
watch(open, (isOpen) => {
  if (!isOpen) return;
  percent.value = toPercent(
    props.kind === "loan"
      ? monthlyLedger.value.loanIncentiveRate
      : monthlyLedger.value.savingsIncentiveRate,
  );
});

async function save() {
  saving.value = true;
  try {
    if (await setIncentiveRate(props.kind, selectedYear.value, percent.value))
      open.value = false;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <UiModal
    :open="open"
    :title="`${kind === 'loan' ? 'Insentif pinjaman' : 'Insentif simpanan'} ${selectedYear}`"
    @close="open = false"
  >
    <form class="form-stack" @submit.prevent="save">
      <label class="field"
        ><span>{{ label }}</span
        ><input
          v-model.number="percent"
          type="number"
          min="0"
          max="100"
          step="0.01"
          required
          :aria-label="label"
      /></label>
      <div class="modal-actions">
        <button
          class="button button--secondary"
          type="button"
          @click="open = false"
        >
          Batal
        </button>
        <button class="button button--primary" type="submit" :disabled="saving">
          Simpan
        </button>
      </div>
    </form>
  </UiModal>
</template>
