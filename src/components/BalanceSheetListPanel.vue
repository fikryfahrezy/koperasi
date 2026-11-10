<script setup lang="ts">
import { computed, ref, watch } from "vue";
import BalanceSheetDetailsEditor, {
  type DetailRow,
} from "./BalanceSheetDetailsEditor.vue";
import { type BalanceSheetItem, balanceSheetItems } from "../balance-sheet";
import {
  type BalanceSheetDetail,
  useBalanceSheetDetails,
  useKoperasiStore,
} from "../store/koperasi";

// The list behind a Neraca line of the book year shown (Piutang Lainnya,
// Aktiva Tetap), opened with Ubah under Catatan. Simpan or Batal closes it.
const props = defineProps<{ itemKey: BalanceSheetItem; title: string }>();
const emit = defineEmits<{ done: [] }>();
const { selectedYear, setBalanceSheetItems } = useKoperasiStore();
const { balanceSheetDetails } = useBalanceSheetDetails();
const config = computed(() => {
  const item = balanceSheetItems.find((entry) => entry.key === props.itemKey);
  return item && "detailed" in item ? item.detailed : {};
});

const toRows = (details: BalanceSheetDetail[]): DetailRow[] =>
  details.map((detail) => ({
    name: detail.name,
    amount: Math.abs(detail.amount),
    deduction: detail.amount < 0,
    date: detail.loanDate ?? "",
    note: detail.note,
  }));
const toDetails = (rows: DetailRow[]): BalanceSheetDetail[] =>
  rows
    .filter((row) => row.name.trim() || row.amount)
    .map((row) => ({
      name: row.name.trim(),
      amount: row.deduction ? -row.amount : row.amount,
      loanDate: row.date || null,
      note: row.note.trim(),
    }));

const saved = computed(() => balanceSheetDetails.value[props.itemKey] ?? []);
const rows = ref<DetailRow[]>([]);
watch(saved, () => (rows.value = toRows(saved.value)), { immediate: true });
const saving = ref(false);

async function save() {
  saving.value = true;
  try {
    if (
      await setBalanceSheetItems(
        selectedYear.value,
        {},
        { [props.itemKey]: toDetails(rows.value) },
      )
    )
      emit("done");
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form class="list-panel" @submit.prevent="save">
    <BalanceSheetDetailsEditor
      v-model="rows"
      :label="`${title} ${selectedYear}`"
      v-bind="config"
    />
    <div class="list-panel__actions">
      <button
        class="button button--secondary"
        type="button"
        @click="emit('done')"
      >
        Batal
      </button>
      <button class="button button--primary" type="submit" :disabled="saving">
        Simpan
      </button>
    </div>
  </form>
</template>

<style scoped>
.list-panel {
  display: grid;
  gap: 8px;
  width: min(640px, 100%);
}
.list-panel__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
