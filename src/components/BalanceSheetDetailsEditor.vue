<script setup lang="ts">
import { computed, ref } from "vue";
import { Plus, Trash2 } from "lucide-vue-next";
import RupiahInput from "./RupiahInput.vue";
import { formatSheetNumber } from "../store/koperasi";

// The list behind a Neraca line (Piutang Lainnya: a bon per row with the date
// borrowed; Aktiva Tetap: a pos per row). A Pengurang row is subtracted, e.g.
// "Bayar dari SHU" or Akum. Penyusutan. The line is the total.
export type DetailRow = {
  name: string;
  amount: number;
  deduction: boolean;
  date: string;
  note: string;
};

const props = withDefaults(
  defineProps<{
    label: string;
    dateLabel?: string;
    addLabel?: string;
    /** A new row under a row keeps its name (several bon per person). */
    keepName?: boolean;
  }>(),
  { dateLabel: "Tanggal", addLabel: "Tambah baris", keepName: false },
);
const rows = defineModel<DetailRow[]>({ required: true });

const emptyRow = (name = ""): DetailRow => ({
  name,
  amount: 0,
  deduction: false,
  date: "",
  note: "",
});
const dateFocus = ref(-1);
const signed = (row: DetailRow) => (row.deduction ? -row.amount : row.amount);
const total = computed(() =>
  rows.value.reduce((sum, row) => sum + signed(row), 0),
);

function addRow(index: number) {
  rows.value.splice(
    index + 1,
    0,
    emptyRow(props.keepName ? (rows.value[index]?.name ?? "") : ""),
  );
}
</script>

<template>
  <div class="details">
    <div class="details__head">
      <span>{{ label }}</span>
      <strong>{{ formatSheetNumber(total) }}</strong>
    </div>
    <div class="details__scroll">
      <table class="details__table">
        <thead>
          <tr>
            <th>Nama</th>
            <th>{{ dateLabel }}</th>
            <th>Jumlah</th>
            <th>Pengurang</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <template v-for="(row, index) in rows" :key="index">
            <tr>
              <td>
                <input v-model="row.name" required aria-label="Nama" />
              </td>
              <td>
                <!-- WebKit shows an empty date input as today's date in grey;
                     a text field until it is used keeps it visibly empty. -->
                <input
                  v-model="row.date"
                  :type="row.date || dateFocus === index ? 'date' : 'text'"
                  placeholder="-"
                  :aria-label="dateLabel"
                  @focus="dateFocus = index"
                  @blur="dateFocus = -1"
                />
              </td>
              <td><RupiahInput v-model="row.amount" aria-label="Jumlah" /></td>
              <td class="details__check">
                <input
                  v-model="row.deduction"
                  type="checkbox"
                  aria-label="Pengurang"
                />
              </td>
              <td class="details__actions">
                <button
                  class="icon-button"
                  type="button"
                  aria-label="Tambah baris di bawahnya"
                  @click="addRow(index)"
                >
                  <Plus :size="14" />
                </button>
                <button
                  class="icon-button"
                  type="button"
                  aria-label="Hapus baris"
                  @click="rows.splice(index, 1)"
                >
                  <Trash2 :size="14" />
                </button>
              </td>
            </tr>
            <!-- Keterangan under the row, so the table needs no horizontal scroll. -->
            <tr class="details__note">
              <td colspan="5">
                <input
                  v-model="row.note"
                  placeholder="Keterangan (opsional)"
                  aria-label="Keterangan"
                />
              </td>
            </tr>
          </template>
          <tr v-if="!rows.length">
            <td colspan="5" class="details__empty">Belum ada baris.</td>
          </tr>
        </tbody>
      </table>
    </div>
    <button
      class="button button--secondary details__add"
      type="button"
      @click="rows.push(emptyRow())"
    >
      <Plus :size="16" /> {{ addLabel }}
    </button>
  </div>
</template>

<style scoped>
.details {
  display: grid;
  gap: 8px;
}
.details__head {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  font-weight: 700;
}
/* Long lists scroll inside the box, so the rest of the form stays short. */
.details__scroll {
  max-height: 280px;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 9px;
}
.details__table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.details__table thead th {
  position: sticky;
  top: 0;
  z-index: 1;
  padding: 4px 6px;
  background: #fff;
  box-shadow: inset 0 -1px var(--line);
  text-align: left;
}
.details__table td {
  padding: 3px 4px;
}
/* Text, date and amount fields; the Pengurang checkbox keeps its own look. */
.details__table :deep(input:not([type="checkbox"])) {
  width: 100%;
  min-width: 0;
  height: 32px;
  padding: 0 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  font: inherit;
}
.details__note td {
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
}
.details__note input {
  height: 28px;
  font-size: 11px;
}
.details__check {
  text-align: center;
}
.details__check input {
  width: 16px;
  height: 16px;
  margin: 0;
  vertical-align: middle;
  accent-color: var(--green);
}
.details__actions {
  white-space: nowrap;
}
.details__empty {
  padding: 12px;
  color: var(--muted);
  text-align: center;
}
.details__add {
  justify-self: start;
}
</style>
