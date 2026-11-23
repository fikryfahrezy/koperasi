<script setup lang="ts">
import { computed, ref } from "vue";
import { Plus } from "lucide-vue-next";
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

withDefaults(
  defineProps<{
    label: string;
    dateLabel?: string;
    addLabel?: string;
  }>(),
  { dateLabel: "Tanggal", addLabel: "Tambah baris" },
);

const rows = defineModel<DetailRow[]>({ required: true });

const emptyRow = (): DetailRow => ({
  name: "",
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
            <th>Keterangan</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, index) in rows" :key="index">
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
            <td>
              <input
                v-model="row.note"
                placeholder="Opsional"
                aria-label="Keterangan"
              />
            </td>
            <td class="details__actions">
              <button
                class="details__delete"
                type="button"
                @click="rows.splice(index, 1)"
              >
                Hapus
              </button>
            </td>
          </tr>
          <tr v-if="!rows.length">
            <td colspan="6" class="details__empty">Belum ada baris.</td>
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
/* The whole list shows; only a narrow window scrolls it sideways. */
.details__scroll {
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 9px;
}
.details__table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.details__table thead th {
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
.details__table tbody tr + tr td {
  border-top: 1px solid var(--line);
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
.details__delete {
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  color: var(--ink);
  background: #fff;
  font: inherit;
  font-weight: 700;
  cursor: pointer;
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
