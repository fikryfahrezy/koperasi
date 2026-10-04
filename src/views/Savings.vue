<script setup lang="ts">
import { computed, watch } from "vue";
import { CalendarX2, Search } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import { useVirtualSheet } from "../composables/useVirtualSheet";
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

const pageState = useWorkspacePageState("/savings");

const { members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const period = pageState.field("period", () => "");
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
const periodOptions = computed(() => [
  { value: "", label: "Semua" },
  ...[...monthlyLedger.value.periods]
    .reverse()
    .map((value) => ({ value, label: formatPeriod(value) })),
]);
const visiblePeriods = computed(() =>
  period.value ? [period.value] : [...monthlyLedger.value.periods].sort(),
);
watch(
  () => monthlyLedger.value.periods,
  (periods) => {
    if (period.value && !periods.includes(period.value)) period.value = "";
  },
  { immediate: true },
);

const memberNames = computed(
  () => new Map(members.map((member) => [member.id, member.name])),
);

// Spreadsheet rows follow member numbers (M001, M002, ...).
const rowsForPeriod = (selectedPeriod: string) =>
  monthlyLedger.value.savings
    .filter((row) => row.period === selectedPeriod)
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({
      ...row,
      no: index + 1,
      name: memberNames.value.get(row.memberId) ?? row.memberId,
    }))
    .filter((row) =>
      row.name
        .toLowerCase()
        .includes(debouncedQuery.value.trim().toLowerCase()),
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
const totalsForRows = (monthRows: ReturnType<typeof rowsForPeriod>) => {
  const sum = Object.fromEntries(numericKeys.map((key) => [key, 0])) as Record<
    NumericKey,
    number
  >;
  for (const row of monthRows) {
    for (const key of numericKeys) sum[key] += row[key];
  }
  return sum;
};
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
const movementKeysForPeriod = (hasShu: boolean) =>
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
  ).filter((key) => key !== "shu" || hasShu);
const closingKeys = [
  "principalClosing",
  "mandatoryClosing",
  "voluntaryClosing",
] as const;
const periodTables = computed(() =>
  visiblePeriods.value.map((period) => {
    const monthRows = rowsForPeriod(period);
    const hasShu = period.endsWith("-01");
    return {
      period,
      monthRows,
      rowsById: new Map(monthRows.map((row) => [row.memberId, row])),
      totals: totalsForRows(monthRows),
      hasShu,
      movementKeys: movementKeysForPeriod(hasShu),
    };
  }),
);
const monthRows = computed(() => {
  const rowsById = new Map<string, ReturnType<typeof rowsForPeriod>[number]>();
  for (const table of periodTables.value) {
    for (const row of table.monthRows) rowsById.set(row.memberId, row);
  }
  return [...rowsById.values()]
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({ ...row, no: index + 1 }));
});
const totalColumnCount = computed(
  () =>
    2 +
    periodTables.value.reduce(
      (count, table) => count + (table.hasShu ? 16 : 15),
      0,
    ),
);
const columnWidths = computed(() => [
  40,
  240,
  ...Array(totalColumnCount.value - 2).fill(110),
]);
const { scroller, table, renderedRows, paddingTop, paddingBottom, navigation } =
  useVirtualSheet(
    monthRows,
    (row) => row.memberId,
    pageState.tabId,
    pageState.active,
    () => Array(totalColumnCount.value).fill(1),
  );
</script>
<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Simpanan" :refresh="refresh">
      <template #title-meta>
        <span v-if="monthlyLedgerLoading" class="sheet-loading" role="status">
          Memuat simpanan…
        </span>
      </template>
      <template #before-actions>
        <label v-if="monthlyLedger.periods.length" class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            type="search"
            aria-label="Cari nama pensiunan"
            placeholder="Cari nama pensiunan..."
          />
        </label>
        <UiSelect
          v-if="monthlyLedger.periods.length"
          v-model="period"
          :options="periodOptions"
          label="Bulan"
          aria-label="Bulan"
          variant="toolbar"
        />
      </template>
    </PageHeader>
    <section
      v-if="monthlyLedgerLoading"
      class="panel backend-state"
      aria-busy="true"
    >
      <strong>Memuat simpanan…</strong>
    </section>
    <section
      v-else-if="!monthlyLedger.periods.length"
      class="panel year-empty-state"
    >
      <CalendarX2 :size="36" />
      <strong>Belum ada data simpanan untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain.</p>
    </section>
    <section v-else class="panel table-panel">
      <div ref="scroller" class="data-table-wrap sheet-wrap">
        <table
          ref="table"
          v-table-navigation="navigation"
          class="sheet-table sheet-table--virtual"
          :style="{
            width: `${columnWidths.reduce((sum, width) => sum + width, 0)}px`,
          }"
          :aria-rowcount="monthRows.length + 5"
        >
          <colgroup>
            <col
              v-for="(width, index) in columnWidths"
              :key="index"
              :style="{ width: `${width}px` }"
            />
          </colgroup>
          <thead>
            <tr>
              <th rowspan="4" class="sheet-sticky sheet-no">No</th>
              <th rowspan="4" class="sheet-sticky sheet-name">
                Nama Pensiunan
              </th>
              <template
                v-for="{ period, hasShu } in periodTables"
                :key="period"
              >
                <th :colspan="hasShu ? 16 : 15" class="sheet-group">
                  {{ formatPeriod(period) }}
                </th>
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu } in periodTables"
                :key="period"
              >
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
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu } in periodTables"
                :key="period"
              >
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
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu } in periodTables"
                :key="period"
              >
                <th>Debet</th>
                <th>Kredit</th>
                <th>Debet</th>
                <th>Kredit</th>
                <th>Debet</th>
                <th>Kredit</th>
              </template>
            </tr>
          </thead>
          <tbody>
            <tr v-if="paddingTop" class="sheet-spacer" aria-hidden="true">
              <td
                :colspan="totalColumnCount"
                :style="{ height: `${paddingTop}px` }"
              ></td>
            </tr>
            <tr
              v-for="{ row, index } in renderedRows"
              :key="row.memberId"
              :data-virtual-row="index"
              :aria-rowindex="index + 5"
            >
              <td class="sheet-sticky sheet-no">{{ row.no }}</td>
              <td class="sheet-sticky sheet-name">
                <strong>{{ row.name }}</strong>
              </td>
              <template v-for="table in periodTables" :key="table.period">
                <template
                  v-for="monthRow in [table.rowsById.get(row.memberId)]"
                  :key="table.period"
                >
                  <td v-for="key in openingKeys" :key="key" class="num-cell">
                    {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                  </td>
                  <td class="num-cell sheet-total">
                    {{
                      monthRow ? formatSheetNumber(openingTotal(monthRow)) : "-"
                    }}
                  </td>
                  <td>
                    {{
                      monthRow
                        ? formatSheetDate(monthRow.transactionDate) || "-"
                        : "-"
                    }}
                  </td>
                  <td
                    v-for="key in table.movementKeys"
                    :key="key"
                    class="num-cell"
                  >
                    {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                  </td>
                  <td v-for="key in closingKeys" :key="key" class="num-cell">
                    {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                  </td>
                  <td class="num-cell sheet-total">
                    <strong>{{
                      monthRow ? formatSheetNumber(closingTotal(monthRow)) : "-"
                    }}</strong>
                  </td>
                </template>
              </template>
            </tr>
            <tr v-if="paddingBottom" class="sheet-spacer" aria-hidden="true">
              <td
                :colspan="totalColumnCount"
                :style="{ height: `${paddingBottom}px` }"
              ></td>
            </tr>
          </tbody>
          <tfoot>
            <tr
              :data-virtual-row="monthRows.length"
              :aria-rowindex="monthRows.length + 5"
            >
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH</td>
              <template v-for="table in periodTables" :key="table.period">
                <td v-for="key in openingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(table.totals[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  {{ formatSheetNumber(openingTotal(table.totals)) }}
                </td>
                <td></td>
                <td
                  v-for="key in table.movementKeys"
                  :key="key"
                  class="num-cell"
                >
                  {{ formatSheetNumber(table.totals[key]) }}
                </td>
                <td v-for="key in closingKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(table.totals[key]) }}
                </td>
                <td class="num-cell sheet-total">
                  {{ formatSheetNumber(closingTotal(table.totals)) }}
                </td>
              </template>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
  </div>
</template>
