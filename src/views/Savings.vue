<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { CalendarX2, Percent, Search } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import { useVirtualSheetColumns } from "../composables/useVirtualSheetColumns";
import { useVirtualSheet } from "../composables/useVirtualSheet";
import ExportButton from "../components/ExportButton.vue";
import IncentiveRatesDialog from "../components/IncentiveRatesDialog.vue";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import {
  formatPeriod,
  formatPreviousPeriod,
  formatSheetDate,
  formatSheetNumber,
  matchesMember,
  type SavingsMonth,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";
import {
  savingsLeadingLetters,
  savingsMonthLetters,
  savingsTailLetters,
} from "../sheet-columns";

const pageState = useWorkspacePageState("/savings");
const formatRate = (rate: number) =>
  `${(rate * 100).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;
const ratesOpen = ref(false);

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

const memberById = computed(
  () => new Map(members.map((member) => [member.id, member])),
);

// Spreadsheet rows follow member numbers (M001, M002, ...).
const rowsForPeriod = (selectedPeriod: string) =>
  monthlyLedger.value.savings
    .filter((row) => row.period === selectedPeriod)
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({
      ...row,
      no: index + 1,
      name: memberById.value.get(row.memberId)?.name ?? row.memberId,
    }))
    .filter((row) =>
      matchesMember(
        debouncedQuery.value,
        row.name,
        memberById.value.get(row.memberId)?.memberNumber,
      ),
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
// Ratas Simpanan (kolom DA) and Insentif Simpanan 3,65% (DC, DD) as of the last
// month shown, computed by the backend (rules 2.3a).
const lastPeriod = computed(
  () => visiblePeriods.value[visiblePeriods.value.length - 1] ?? "",
);
const incentiveById = computed(
  () =>
    new Map(
      monthlyLedger.value.savings
        .filter((row) => row.period === lastPeriod.value)
        .map((row) => [
          row.memberId,
          {
            average: row.savingsAverage,
            incentiveRaw: row.savingsIncentiveRaw,
            incentive: row.savingsIncentive,
          },
        ]),
    ),
);
const incentiveKeys = ["average", "incentiveRaw", "incentive"] as const;
// The month groups, then Ratas and Insentif Simpanan once at the end.
const columnGroups = computed(() => [
  ...periodTables.value.map((table) => ({ ...table, kind: "month" as const })),
  ...(periodTables.value.length
    ? [
        {
          ...periodTables.value[periodTables.value.length - 1]!,
          kind: "tail" as const,
          period: "tail",
        },
      ]
    : []),
]);
const groupColumns = (group: (typeof columnGroups.value)[number]) =>
  group.kind === "tail" ? incentiveKeys.length : group.hasShu ? 16 : 15;

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
    columnGroups.value.reduce((count, group) => count + groupColumns(group), 0),
);
const incentiveTotals = computed(() => {
  const sum = { average: 0, incentiveRaw: 0, incentive: 0 };
  for (const row of monthRows.value) {
    const value = incentiveById.value.get(row.memberId);
    if (!value) continue;
    for (const key of incentiveKeys) sum[key] += value[key];
  }
  return sum;
});
const {
  scroller,
  table,
  renderedRows,
  paddingTop,
  paddingBottom,
  navigation: rowNavigation,
} = useVirtualSheet(
  monthRows,
  (row) => row.memberId,
  pageState.tabId,
  pageState.active,
  () => Array(totalColumnCount.value).fill(1),
);
const {
  renderedGroups: renderedPeriodTables,
  leftColumns,
  rightColumns,
  widths: columnWidths,
  totalWidth,
  scrollToColumn,
} = useVirtualSheetColumns(
  columnGroups,
  groupColumns,
  [56, 240],
  scroller,
  pageState.active,
);
const renderedColumnCount = computed(() => columnWidths.value.length);
const navigation = computed(() => ({
  ...rowNavigation.value,
  async scrollToRow(index: number, column = 0) {
    scrollToColumn(column);
    await rowNavigation.value.scrollToRow(index);
    await nextTick();
  },
}));
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
            aria-label="Cari nama atau ID anggota"
            placeholder="Cari nama atau ID anggota..."
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
        <button
          v-if="monthlyLedger.periods.length"
          class="button button--secondary"
          type="button"
          @click="ratesOpen = true"
        >
          <Percent :size="16" /> Insentif {{ selectedYear }}
        </button>
      </template>
      <template #actions>
        <ExportButton v-if="monthlyLedger.periods.length" kind="savings" />
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
          v-sticky-header
          class="sheet-table sheet-table--virtual"
          :style="{
            width: `${totalWidth}px`,
          }"
          :aria-colcount="totalColumnCount"
          :aria-rowcount="monthRows.length + 6"
        >
          <colgroup>
            <col
              v-for="(width, index) in columnWidths"
              :key="index"
              :style="{ width: `${width}px` }"
            />
          </colgroup>
          <thead>
            <tr class="sheet-letters">
              <th class="sheet-sticky sheet-no">
                {{ savingsLeadingLetters[0] }}
              </th>
              <th class="sheet-sticky sheet-name">
                {{ savingsLeadingLetters[1] }}
              </th>
              <th
                v-if="leftColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
              <template
                v-for="{ period, hasShu, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'month'">
                  <th
                    v-for="(letter, index) in savingsMonthLetters(
                      period,
                      hasShu,
                      selectedYear,
                    )"
                    :key="index"
                  >
                    {{ letter }}
                  </th>
                </template>
                <template v-else>
                  <th
                    v-for="(letter, index) in savingsTailLetters(selectedYear)"
                    :key="index"
                  >
                    {{ letter }}
                  </th>
                </template>
              </template>
              <th
                v-if="rightColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
            </tr>
            <tr>
              <th rowspan="4" class="sheet-sticky sheet-no">No</th>
              <th rowspan="4" class="sheet-sticky sheet-name">
                Nama Pensiunan
              </th>
              <th
                v-if="leftColumns"
                rowspan="4"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
              <template
                v-for="{ period, hasShu, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'month'">
                  <th
                    :colspan="hasShu ? 16 : 15"
                    class="sheet-group"
                    :data-period="period"
                  >
                    {{ formatPeriod(period) }}
                  </th>
                </template>
                <template v-else>
                  <th rowspan="4">
                    Ratas simpanan<br />s/d {{ formatPeriod(lastPeriod) }}
                  </th>
                  <th colspan="2" rowspan="2" class="sheet-group">
                    Insentif simpanan
                  </th>
                </template>
              </template>
              <th
                v-if="rightColumns"
                rowspan="4"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'month'">
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
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'month'">
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
                <template v-else>
                  <th rowspan="2">
                    {{ formatRate(monthlyLedger.savingsIncentiveRate) }}
                  </th>
                  <th rowspan="2">Pembulatan</th>
                </template>
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, hasShu, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'month'">
                  <th>Debet</th>
                  <th>Kredit</th>
                  <th>Debet</th>
                  <th>Kredit</th>
                  <th>Debet</th>
                  <th>Kredit</th>
                </template>
              </template>
            </tr>
          </thead>
          <tbody>
            <tr v-if="paddingTop" class="sheet-spacer" aria-hidden="true">
              <td
                :colspan="renderedColumnCount"
                :style="{ height: `${paddingTop}px` }"
              ></td>
            </tr>
            <tr
              v-for="{ row, index } in renderedRows"
              :key="row.memberId"
              :data-virtual-row="index"
              :aria-rowindex="index + 6"
            >
              <td class="sheet-sticky sheet-no">{{ row.no }}</td>
              <td class="sheet-sticky sheet-name">
                <strong>{{ row.name }}</strong>
              </td>
              <td
                v-if="leftColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
                data-virtual-gap
                :data-virtual-span="leftColumns"
              ></td>
              <template
                v-for="table in renderedPeriodTables"
                :key="table.period"
              >
                <template v-if="table.kind === 'tail'">
                  <td v-for="key in incentiveKeys" :key="key" class="num-cell">
                    {{
                      formatSheetNumber(
                        incentiveById.get(row.memberId)?.[key] ?? 0,
                      )
                    }}
                  </td>
                </template>
                <template
                  v-for="monthRow in table.kind === 'month'
                    ? [table.rowsById.get(row.memberId)]
                    : []"
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
              <td
                v-if="rightColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
                data-virtual-gap
                :data-virtual-span="rightColumns"
              ></td>
            </tr>
            <tr v-if="paddingBottom" class="sheet-spacer" aria-hidden="true">
              <td
                :colspan="renderedColumnCount"
                :style="{ height: `${paddingBottom}px` }"
              ></td>
            </tr>
          </tbody>
          <tfoot>
            <tr
              :data-virtual-row="monthRows.length"
              :aria-rowindex="monthRows.length + 6"
            >
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH</td>
              <td
                v-if="leftColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
                data-virtual-gap
                :data-virtual-span="leftColumns"
              ></td>
              <template
                v-for="table in renderedPeriodTables"
                :key="table.period"
              >
                <template v-if="table.kind === 'tail'">
                  <td v-for="key in incentiveKeys" :key="key" class="num-cell">
                    {{ formatSheetNumber(incentiveTotals[key]) }}
                  </td>
                </template>
                <template v-else>
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
              </template>
              <td
                v-if="rightColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
                data-virtual-gap
                :data-virtual-span="rightColumns"
              ></td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
    <IncentiveRatesDialog v-model:open="ratesOpen" kind="savings" />
  </div>
</template>
