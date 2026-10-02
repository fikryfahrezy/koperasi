<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CalendarX2 } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import {
  formatPeriod,
  formatPreviousPeriod,
  formatSheetDate,
  formatSheetNumber,
  type SavingsMonth,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const { members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const period = ref("");
const periodOptions = computed(() =>
  [...monthlyLedger.periods]
    .reverse()
    .map((value) => ({ value, label: formatPeriod(value) })),
);
watch(
  () => monthlyLedger.periods,
  (periods) => {
    if (!periods.includes(period.value))
      period.value = periods[periods.length - 1] ?? "";
  },
  { immediate: true },
);

const memberNames = computed(
  () => new Map(members.map((member) => [member.id, member.name])),
);

// Spreadsheet rows follow member numbers (M001, M002, ...).
const monthRows = computed(() =>
  monthlyLedger.savings
    .filter((row) => row.period === period.value)
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({
      ...row,
      no: index + 1,
      name: memberNames.value.get(row.memberId) ?? row.memberId,
    })),
);
type NumericKey = {
  [K in keyof SavingsMonth]: SavingsMonth[K] extends number ? K : never;
}[keyof SavingsMonth];
const numericKeys: NumericKey[] = [
  "principalOpening",
  "mandatoryOpening",
  "voluntaryOpening",
  "principalIn",
  "principalOut",
  "mandatoryIn",
  "mandatoryOut",
  "voluntaryIn",
  "voluntaryOut",
  "shu",
  "principalClosing",
  "mandatoryClosing",
  "voluntaryClosing",
];
const totals = computed(() => {
  const sum = Object.fromEntries(numericKeys.map((key) => [key, 0])) as Record<
    NumericKey,
    number
  >;
  for (const row of monthRows.value) {
    for (const key of numericKeys) sum[key] += row[key];
  }
  return sum;
});
const openingTotal = (row: Record<NumericKey, number>) =>
  row.principalOpening + row.mandatoryOpening + row.voluntaryOpening;
const closingTotal = (row: Record<NumericKey, number>) =>
  row.principalClosing + row.mandatoryClosing + row.voluntaryClosing;

const openingKeys = [
  "principalOpening",
  "mandatoryOpening",
  "voluntaryOpening",
] as const;
// Debit = outflow, Credit = inflow, as in the spreadsheet. The profit-sharing column appears only
// in the January block (profit sharing for the previous fiscal year).
const hasShu = computed(() => period.value.endsWith("-01"));
const movementKeys = computed(() =>
  (
    [
      "principalOut",
      "principalIn",
      "mandatoryOut",
      "mandatoryIn",
      "voluntaryOut",
      "voluntaryIn",
      "shu",
    ] as const
  ).filter((key) => key !== "shu" || hasShu.value),
);
const closingKeys = [
  "principalClosing",
  "mandatoryClosing",
  "voluntaryClosing",
] as const;
</script>
<template>
  <div class="page-stack">
    <PageHeader title="Simpanan" :refresh="refresh" />
    <section
      v-if="!monthlyLedgerLoading && !monthlyLedger.periods.length"
      class="panel year-empty-state"
    >
      <CalendarX2 :size="36" />
      <strong>Belum ada data simpanan untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain.</p>
    </section>
    <template v-else>
      <section class="panel table-panel">
        <div class="toolbar">
          <UiSelect
            v-model="period"
            :options="periodOptions"
            aria-label="Bulan"
            variant="toolbar"
          />
        </div>
        <div v-if="period" class="data-table-wrap sheet-wrap">
          <table class="sheet-table">
            <thead>
              <tr>
                <th rowspan="3" class="sheet-sticky sheet-no">No</th>
                <th rowspan="3" class="sheet-sticky sheet-name">
                  Nama Pensiunan
                </th>
                <th colspan="3" class="sheet-group">
                  Saldo bulan {{ formatPreviousPeriod(period) }}
                </th>
                <th rowspan="3" class="sheet-total">
                  Total simpanan<br />{{ formatPreviousPeriod(period) }}
                </th>
                <th
                  :colspan="hasShu ? 8 : 7"
                  class="sheet-group sheet-group--mutasi"
                >
                  Mutasi bulan {{ formatPeriod(period) }}
                </th>
                <th colspan="3" class="sheet-group">
                  Saldo simpanan bulan {{ formatPeriod(period) }}
                </th>
                <th rowspan="3" class="sheet-total">
                  Total simpanan<br />{{ formatPeriod(period) }}
                </th>
              </tr>
              <tr>
                <th rowspan="2">Pokok</th>
                <th rowspan="2">Wajib</th>
                <th rowspan="2">Manasuka</th>
                <th rowspan="2">Tanggal</th>
                <th colspan="2">Simpanan Pokok</th>
                <th colspan="2">Simpanan Wajib</th>
                <th colspan="2">Manasuka</th>
                <th v-if="hasShu" rowspan="2">
                  SHU Tahun Buku {{ selectedYear - 1 }}
                </th>
                <th rowspan="2">Pokok</th>
                <th rowspan="2">Wajib</th>
                <th rowspan="2">Manasuka</th>
              </tr>
              <tr>
                <th>Debet</th>
                <th>Kredit</th>
                <th>Debet</th>
                <th>Kredit</th>
                <th>Debet</th>
                <th>Kredit</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in monthRows" :key="row.memberId">
                <td class="sheet-sticky sheet-no">{{ row.no }}</td>
                <td class="sheet-sticky sheet-name">
                  <strong>{{ row.name }}</strong>
                </td>
                <td v-for="key in openingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(row[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  {{ formatSheetNumber(openingTotal(row)) }}
                </td>
                <td>{{ formatSheetDate(row.transactionDate) || "-" }}</td>
                <td v-for="key in movementKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(row[key]) }}
                </td>
                <td v-for="key in closingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(row[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  <strong>{{ formatSheetNumber(closingTotal(row)) }}</strong>
                </td>
              </tr>
            </tbody>
            <tfoot>
              <tr>
                <td class="sheet-sticky sheet-no"></td>
                <td class="sheet-sticky sheet-name">JUMLAH</td>
                <td v-for="key in openingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(totals[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  {{ formatSheetNumber(openingTotal(totals)) }}
                </td>
                <td></td>
                <td v-for="key in movementKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(totals[key]) }}
                </td>
                <td v-for="key in closingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(totals[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  {{ formatSheetNumber(closingTotal(totals)) }}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>
