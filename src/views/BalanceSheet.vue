<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { PencilLine } from "lucide-vue-next";
import BalanceSheetItemsDialog from "../components/BalanceSheetItemsDialog.vue";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import { balanceSheetAt, sheetRows } from "../balance-sheet";
import {
  Channel,
  formatPeriod,
  formatSheetNumber,
  useBalanceSheetItems,
  useCashBook,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const pageState = useWorkspacePageState("/balance-sheet");
const { loans, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();
const { cashBook } = useCashBook(Channel.Cash);
const { balanceSheetItems } = useBalanceSheetItems();
// The first column is 31 Desember of the year before, as in the sheet.
const { monthlyLedger: previousLedger } = useMonthlyLedger(-1);
const { cashBook: previousCashBook } = useCashBook(Channel.Cash, -1);
const { balanceSheetItems: previousItems } = useBalanceSheetItems(-1);
const itemsOpen = ref(false);

const periods = computed(() => [...monthlyLedger.value.periods].sort());
// Empty means the last month of the year shown.
const period = pageState.field("period", () => "");
watch(periods, (list) => {
  if (period.value && !list.includes(period.value)) period.value = "";
});
const selectedPeriod = computed(
  () => period.value || periods.value[periods.value.length - 1] || "",
);
const periodOptions = computed(() =>
  [...periods.value]
    .reverse()
    .map((value) => ({ value, label: formatPeriod(value) })),
);
const previousPeriod = computed(() => `${selectedYear.value - 1}-12`);

const current = computed(() =>
  balanceSheetAt(
    selectedPeriod.value,
    monthlyLedger.value,
    cashBook.value,
    loans,
    balanceSheetItems.value,
  ),
);
const previous = computed(() =>
  balanceSheetAt(
    previousPeriod.value,
    previousLedger.value,
    previousCashBook.value,
    loans,
    {
      ...previousItems.value,
      // Without a typed balance, this year's opening Kas Buku Tabungan is the
      // bank at 31 Desember.
      BANK_BALANCE:
        previousItems.value.BANK_BALANCE ||
        (cashBook.value?.savingsBookOpeningBalance ?? 0),
    },
  ),
);
const rows = computed(() => sheetRows([previous.value, current.value]));
const differences = computed(() => [
  previous.value.difference,
  current.value.difference,
]);

const lastDay = (value: string) => {
  const [year, month] = value.split("-").map(Number);
  if (!year) return "";
  return `${new Date(year, month, 0).getDate()} ${formatPeriod(value)}`;
};
const columnLabel = (value: string) => {
  const [year, month] = value.split("-").map(Number);
  return new Date(year, month - 1, 1)
    .toLocaleDateString("en-US", {
      month: "short",
      year: "2-digit",
    })
    .replace(" ", "-");
};
const columns = computed(() => [
  columnLabel(previousPeriod.value),
  columnLabel(selectedPeriod.value),
]);
const hasEstimate = computed(() =>
  rows.value.some((row) =>
    [row.left, row.right].some(
      (cell) => cell.kind === "line" && cell.estimated.some(Boolean),
    ),
  ),
);
</script>

<template>
  <div class="page-stack balance-page">
    <PageHeader title="Neraca" :refresh="refresh">
      <template #before-actions>
        <UiSelect
          v-if="periods.length"
          v-model="period"
          :options="periodOptions"
          label="Bulan"
          aria-label="Bulan"
          variant="toolbar"
        />
        <button
          class="button button--secondary"
          type="button"
          @click="itemsOpen = true"
        >
          <PencilLine :size="16" /> Pos neraca {{ selectedYear }}
        </button>
      </template>
    </PageHeader>

    <section
      v-if="monthlyLedgerLoading && !periods.length"
      class="panel balance-empty"
    >
      Memuat data…
    </section>
    <section v-else-if="!periods.length" class="panel balance-empty">
      <strong>Belum ada data untuk {{ selectedYear }}</strong>
    </section>

    <template v-else>
      <p v-if="current.difference !== 0" class="balance-check" role="status">
        Belum seimbang: aktiva − pasiva =
        <strong>{{ formatSheetNumber(current.difference) }}</strong> per
        {{ lastDay(selectedPeriod) }}. Periksa pos neraca {{ selectedYear }}
        dan kategori biaya di Buku Kas.
      </p>

      <section class="panel balance-sheet">
        <header class="balance-sheet__title">
          <h2>KOPERASI PENSIUNAN BRI BINA SEJAHTERA</h2>
          <strong>NERACA</strong>
          <u>POSISI {{ lastDay(selectedPeriod).toUpperCase() }}</u>
        </header>
        <div class="balance-sheet__scroll">
          <table class="balance-table">
            <colgroup>
              <col class="balance-table__label" />
              <col class="balance-table__amount" />
              <col class="balance-table__amount" />
              <col class="balance-table__label" />
              <col class="balance-table__amount" />
              <col class="balance-table__amount" />
            </colgroup>
            <thead>
              <tr>
                <th colspan="3">AKTIVA</th>
                <th colspan="3">PASIVA</th>
              </tr>
              <tr>
                <th>URAIAN</th>
                <th v-for="label in columns" :key="`a-${label}`">
                  {{ label }}
                </th>
                <th>URAIAN</th>
                <th v-for="label in columns" :key="`p-${label}`">
                  {{ label }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, index) in rows"
                :key="index"
                :class="{
                  'balance-table__grand': index === rows.length - 1,
                }"
              >
                <template
                  v-for="(cell, side) in [row.left, row.right]"
                  :key="side"
                >
                  <template v-if="cell.kind === 'heading'">
                    <td class="balance-table__heading">{{ cell.label }}</td>
                    <td></td>
                    <td></td>
                  </template>
                  <template v-else-if="cell.kind === 'line'">
                    <td>{{ cell.label }}</td>
                    <td
                      v-for="(amount, column) in cell.amounts"
                      :key="column"
                      class="num-cell"
                    >
                      {{ formatSheetNumber(amount)
                      }}<sup v-if="cell.estimated[column]">*</sup>
                    </td>
                  </template>
                  <template v-else-if="cell.kind === 'total'">
                    <td class="balance-table__total">{{ cell.label }}</td>
                    <td
                      v-for="(amount, column) in cell.amounts"
                      :key="column"
                      class="num-cell balance-table__total"
                    >
                      {{ formatSheetNumber(amount) }}
                    </td>
                  </template>
                  <template v-else>
                    <td></td>
                    <td></td>
                    <td></td>
                  </template>
                </template>
              </tr>
              <tr class="balance-table__difference">
                <td colspan="3"></td>
                <td>Selisih aktiva − pasiva</td>
                <td
                  v-for="(amount, column) in differences"
                  :key="column"
                  class="num-cell"
                >
                  {{ amount === 0 ? "0" : formatSheetNumber(amount) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p v-if="hasEstimate" class="field-hint">
          * Bank dihitung dari Kas Buku Tabungan dan belum tentu sama dengan
          saldo bank. Isi saldo bank 31 Desember di Pos neraca.
        </p>
      </section>

      <section class="panel balance-income">
        <h2>Laba Bersih s/d {{ formatPeriod(selectedPeriod) }}</h2>
        <table class="balance-income__table">
          <tbody>
            <tr>
              <td>Pendapatan bunga pinjaman</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.interest) }}
              </td>
            </tr>
            <tr>
              <td>Pendapatan provisi pinjaman</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.provision) }}
              </td>
            </tr>
            <tr v-for="expense in current.income.expenses" :key="expense.label">
              <td>{{ expense.label }}</td>
              <td class="num-cell">
                {{ formatSheetNumber(-expense.amount) }}
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td>Laba Bersih</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.netIncome) }}
              </td>
            </tr>
          </tfoot>
        </table>
        <p class="field-hint">
          Biaya diambil dari baris Buku Kas yang diberi kategori biaya. Baris
          tanpa kategori tidak ikut dihitung.
        </p>
      </section>
    </template>

    <BalanceSheetItemsDialog v-model:open="itemsOpen" />
  </div>
</template>

<style scoped>
.balance-page {
  gap: 12px;
  padding-bottom: 16px;
}
.balance-empty {
  padding: 32px;
  color: var(--muted);
  text-align: center;
}
.balance-check {
  margin: 0;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-left: 3px solid var(--amber);
  background: var(--amber-soft);
  font-size: 12px;
}
.balance-sheet {
  padding: 18px 20px;
  min-width: 0;
}
.balance-sheet__title {
  display: grid;
  justify-items: center;
  gap: 4px;
  margin-bottom: 14px;
  font-size: 13px;
}
.balance-sheet__title h2 {
  margin: 0;
  color: var(--green);
  font-size: 17px;
  font-weight: 600;
}
.balance-sheet__scroll {
  overflow-x: auto;
}
.balance-table {
  width: 100%;
  min-width: 900px;
  border-collapse: collapse;
  font-size: 12px;
}
.balance-table__label {
  width: 26%;
}
.balance-table__amount {
  width: 12%;
}
.balance-table th,
.balance-table td {
  padding: 5px 7px;
  border: 1px solid var(--line-strong);
}
.balance-table thead th {
  font-style: italic;
  font-weight: 800;
  text-align: center;
}
.balance-table td:nth-child(4) {
  border-left: 2px solid var(--ink);
}
.balance-table__heading {
  font-style: italic;
  font-weight: 800;
  text-decoration: underline;
}
.balance-table__total {
  font-style: italic;
  font-weight: 800;
}
.balance-table__grand td {
  border-top: 2px solid var(--ink);
  border-bottom: 3px double var(--ink);
}
.balance-table__difference td {
  border: 0;
  color: var(--muted);
  font-style: italic;
}
.balance-table sup {
  color: var(--amber);
}
.balance-income {
  padding: 15px 17px;
}
.balance-income h2 {
  margin: 0 0 8px;
  font-size: 14px;
}
.balance-income__table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.balance-income__table td {
  padding: 6px 4px;
  border-bottom: 1px solid var(--line);
}
.balance-income__table tfoot td {
  padding-top: 10px;
  border-top: 2px solid var(--line-strong);
  border-bottom: 0;
  font-weight: 800;
}
.field-hint {
  margin: 8px 0 0;
  color: var(--soft);
  font-size: 10px;
}
</style>
