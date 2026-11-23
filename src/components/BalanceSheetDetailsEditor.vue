<script setup lang="ts">
import { computed, ref } from "vue";
import { Plus } from "lucide-vue-next";
import RupiahInput from "./RupiahInput.vue";
import { type BonStatus, isPaidOff } from "../balance-sheet";
import {
  formatDate,
  formatSheetNumber,
  useKoperasiStore,
} from "../store/koperasi";

// The list behind a Neraca line (Piutang Lainnya: a bon per row with the date
// borrowed; Aktiva Tetap: a pos per row). A Pengurang row is subtracted, e.g.
// "Bayar dari SHU" or Akum. Penyusutan. The line is the total.
export type DetailRow = {
  name: string;
  amount: number;
  deduction: boolean;
  date: string;
  note: string;
  /** Piutang Lainnya: the key of a bon row, so its payments stay linked. */
  bonKey: string;
};

const props = withDefaults(
  defineProps<{
    label: string;
    dateLabel?: string;
    addLabel?: string;
    /** Piutang Lainnya: each bon by key with its payments; shows the Status
     * column (Lunas / Sisa). */
    bonStatuses?: Map<string, BonStatus>;
  }>(),
  { dateLabel: "Tanggal", addLabel: "Tambah baris", bonStatuses: undefined },
);

const rows = defineModel<DetailRow[]>({ required: true });

const emptyRow = (): DetailRow => ({
  name: "",
  amount: 0,
  deduction: false,
  date: "",
  note: "",
  bonKey: `BON-${crypto.randomUUID()}`,
});

const statusOf = (row: DetailRow) =>
  row.deduction ? undefined : props.bonStatuses?.get(row.bonKey);

const { notify } = useKoperasiStore();

// A bon paid with Bayar Kas Bon stays: removing it would leave the payment
// without its bon (Simpan refuses it too).
function remove(index: number) {
  const row = rows.value[index];

  if (statusOf(row)?.payments.length) {
    notify(
      "Bon tidak bisa dihapus",
      `Bon ${row.name.trim() || "ini"} sudah dibayar lewat Buku Kas. Hapus dulu pembayarannya di Buku Kas.`,
      "warning",
    );
    return;
  }

  rows.value.splice(index, 1);
}

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
            <th class="details__note-head">Keterangan</th>
            <th v-if="bonStatuses">Status</th>
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
            <td v-if="bonStatuses" class="details__status">
              <template v-if="statusOf(row)">
                <strong v-if="isPaidOff(statusOf(row))" class="details__paid">
                  Lunas<template v-if="statusOf(row)!.paidOffOn">
                    {{ formatDate(statusOf(row)!.paidOffOn!) }}</template
                  >
                </strong>
                <span v-else-if="statusOf(row)!.payments.length">
                  Sisa {{ formatSheetNumber(statusOf(row)!.left) }}
                </span>
                <span v-else class="details__muted">Belum dibayar</span>
                <ul
                  v-if="statusOf(row)!.payments.length"
                  class="details__payments"
                >
                  <li
                    v-for="(payment, line) in statusOf(row)!.payments"
                    :key="line"
                  >
                    {{ formatDate(payment.date) }} ·
                    {{ formatSheetNumber(payment.amount) }} · Buku Kas
                  </li>
                </ul>
              </template>
              <span v-else-if="!row.deduction" class="details__muted"
                >Bon baru</span
              >
            </td>
            <td class="details__actions">
              <button
                class="details__delete"
                type="button"
                @click="remove(index)"
              >
                Hapus
              </button>
            </td>
          </tr>
          <tr v-if="!rows.length">
            <td :colspan="bonStatuses ? 7 : 6" class="details__empty">
              Belum ada baris.
            </td>
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
/* Keterangan holds names such as "Iwan Setiawan / H Salmon". */
.details__note-head {
  min-width: 260px;
}
.details__status,
.details__actions {
  vertical-align: top;
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
.details__status {
  min-width: 170px;
  font-size: 11px;
}
.details__paid {
  color: var(--green);
}
.details__muted {
  color: var(--muted);
}
.details__payments {
  margin: 2px 0 0;
  padding: 0;
  color: var(--muted);
  list-style: none;
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
