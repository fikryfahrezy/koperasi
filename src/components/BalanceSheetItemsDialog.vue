<script setup lang="ts">
import { ref, watch } from "vue";
import RupiahInput from "./RupiahInput.vue";
import UiModal from "./UiModal.vue";
import { balanceSheetItems } from "../balance-sheet";
import { useBalanceSheetItems, useKoperasiStore } from "../store/koperasi";

// Neraca lines that do not come from the ledger are typed once per book year,
// like the incentive rates.
const open = defineModel<boolean>("open", { required: true });
const { selectedYear, setBalanceSheetItems } = useKoperasiStore();
const { balanceSheetItems: saved } = useBalanceSheetItems();
const values = ref<Record<string, number>>({});
const saving = ref(false);

watch(open, (isOpen) => {
  if (!isOpen) return;
  values.value = Object.fromEntries(
    balanceSheetItems.map((item) => [item.key, saved.value[item.key] ?? 0]),
  );
});

async function save() {
  saving.value = true;
  try {
    if (await setBalanceSheetItems(selectedYear.value, values.value))
      open.value = false;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <UiModal
    :open="open"
    size="lg"
    :title="`Pos neraca ${selectedYear}`"
    @close="open = false"
  >
    <form class="form-stack" @submit.prevent="save">
      <p class="field-hint">
        Pos yang tidak tercatat di Buku Kas, Simpanan, atau Pinjaman. Isi sesuai
        neraca tahun buku {{ selectedYear }} (hasil RAT dan pemakaian dana).
      </p>
      <div class="balance-items">
        <label v-for="item in balanceSheetItems" :key="item.key" class="field">
          <span>{{ item.label }}</span>
          <RupiahInput v-model="values[item.key]" :aria-label="item.label" />
          <small v-if="'hint' in item" class="field-hint">{{
            item.hint
          }}</small>
        </label>
      </div>
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

<style scoped>
.balance-items {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px 16px;
}
.field-hint {
  margin: 0;
  color: var(--soft);
  font-size: 10px;
}
</style>
