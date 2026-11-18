<script setup lang="ts">
import { ref, watch } from "vue";
import RupiahInput from "./RupiahInput.vue";
import UiModal from "./UiModal.vue";
import { useKoperasiStore, useMonthlyLedger } from "../store/koperasi";

// Uang duduk + uang makan is decided at the RAT for each book year, like the
// Insentif rates, so it is edited per year.
const open = defineModel<boolean>("open", { required: true });
const { selectedYear, setSittingAllowance } = useKoperasiStore();
const { monthlyLedger } = useMonthlyLedger();
const amount = ref(0);
const saving = ref(false);
const label = "Uang duduk + uang makan per anggota";

watch(open, (isOpen) => {
  if (isOpen) amount.value = monthlyLedger.value.sittingAllowance;
});

async function save() {
  saving.value = true;

  try {
    if (await setSittingAllowance(selectedYear.value, amount.value))
      open.value = false;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <UiModal
    :open="open"
    :title="`Uang duduk ${selectedYear}`"
    @close="open = false"
  >
    <form class="form-stack" @submit.prevent="save">
      <label class="field"
        ><span>{{ label }}</span
        ><RupiahInput v-model="amount" :min="0" :aria-label="label"
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
