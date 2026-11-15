<script setup lang="ts">
import { computed, ref } from "vue";
import { CalendarX2, Percent, Search } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { usePeriodFilter } from "../composables/usePeriodFilter";
import { useVirtualSheetGrid } from "../composables/useVirtualSheetGrid";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import ExportButton from "../components/ExportButton.vue";
import IncentiveRatesDialog from "../components/IncentiveRatesDialog.vue";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";

import {
  formatPeriod,
  formatPreviousPeriod,
  formatRate,
  formatSheetDate,
  formatSheetNumber,
  matchesMember,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

import {
  closingKeys,
  closingTotal,
  incentiveKeys,
  incentivesAt,
  leadingColumnWidths,
  movementKeysForPeriod,
  openingKeys,
  openingTotal,
  type SavingsSheetRow,
  savingsRowsForPeriod,
  sumSavingsRows,
} from "../savings-sheet";

import {
  savingsLeadingLetters,
  savingsMonthLetters,
  savingsOpeningLetters,
  savingsTailLetters,
} from "../sheet-columns";

const pageState = useWorkspacePageState("/savings");
const ratesOpen = ref(false);

const { members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const { period, periodOptions, visiblePeriods } = usePeriodFilter(
  pageState,
  () => monthlyLedger.value.periods,
);

const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);

const memberById = computed(
  () => new Map(members.map((member) => [member.id, member])),
);

// The export holds the month and the members the page shows.
const exportFilter = computed(() => ({
  period: period.value,
  ids: debouncedQuery.value.trim()
    ? [
        ...new Set(monthlyLedger.value.savings.map((row) => row.memberId)),
      ].filter((memberId) => {
        const member = memberById.value.get(memberId);

        return matchesMember(
          debouncedQuery.value,
          member?.name ?? memberId,
          member?.memberNumber,
        );
      })
    : undefined,
}));

const periodTables = computed(() =>
  visiblePeriods.value.map((period) => {
    const monthRows = savingsRowsForPeriod(
      monthlyLedger.value.savings,
      memberById.value,
      period,
      debouncedQuery.value,
    );

    const hasShu = period.endsWith("-01");

    return {
      period,
      monthRows,
      rowsById: new Map(monthRows.map((row) => [row.memberId, row])),
      totals: sumSavingsRows(monthRows),
      hasShu,
      movementKeys: movementKeysForPeriod(hasShu),
    };
  }),
);

// Ratas Simpanan and Insentif Simpanan as of the last month shown.
const lastPeriod = computed(
  () => visiblePeriods.value[visiblePeriods.value.length - 1] ?? "",
);

const incentiveById = computed(() =>
  incentivesAt(monthlyLedger.value.savings, lastPeriod.value),
);

// Saldo before the first month shown once (as SALDO BULAN DESEMBER in the
// sheet), the month groups, then Ratas and Insentif Simpanan once at the end.
const firstPeriod = computed(() => visiblePeriods.value[0] ?? "");

const columnGroups = computed(() => {
  const first = periodTables.value[0];
  const last = periodTables.value[periodTables.value.length - 1];

  const months = periodTables.value.map((table) => ({
    ...table,
    kind: "month" as "opening" | "month" | "tail",
  }));

  return first && last
    ? [
        { ...first, kind: "opening" as const, period: "opening" },
        ...months,
        { ...last, kind: "tail" as const, period: "tail" },
      ]
    : months;
});

const groupColumns = (group: (typeof columnGroups.value)[number]) =>
  group.kind === "tail"
    ? incentiveKeys.length
    : group.kind === "opening"
      ? openingKeys.length + 1
      : group.hasShu
        ? 12
        : 11;

// Every member of any month shown, numbered once.
const monthRows = computed(() => {
  const rowsById = new Map<string, SavingsSheetRow>();

  for (const table of periodTables.value) {
    for (const row of table.monthRows) rowsById.set(row.memberId, row);
  }

  return [...rowsById.values()]
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({ ...row, no: index + 1 }));
});

const totalColumnCount = computed(
  () =>
    leadingColumnWidths.length +
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
  renderedGroups: renderedPeriodTables,
  leftColumns,
  rightColumns,
  columnWidths,
  totalWidth,
  renderedColumnCount,
  navigation,
} = useVirtualSheetGrid({
  pageState,
  rows: monthRows,
  rowKey: (row) => row.memberId,
  columnSpans: () => Array(totalColumnCount.value).fill(1),
  groups: columnGroups,
  groupColumns,
  leadingWidths: leadingColumnWidths,
});
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
        <ExportButton
          v-if="monthlyLedger.periods.length"
          kind="savings"
          :filter="exportFilter"
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
                <template v-else-if="kind === 'opening'">
                  <th
                    v-for="(letter, index) in savingsOpeningLetters(
                      firstPeriod,
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
                    :colspan="hasShu ? 12 : 11"
                    class="sheet-group"
                    :data-period="period"
                  >
                    {{ formatPeriod(period) }}
                  </th>
                </template>
                <template v-else-if="kind === 'opening'">
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Saldo bulan {{ formatPreviousPeriod(firstPeriod) }}
                  </th>
                  <th rowspan="4" class="sheet-total">
                    Total simpanan<br />{{ formatPreviousPeriod(firstPeriod) }}
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
                <template v-if="kind === 'opening'">
                  <th rowspan="2">Pokok</th>
                  <th rowspan="2">Wajib</th>
                  <th rowspan="2">Manasuka</th>
                </template>
                <template v-else-if="kind === 'month'">
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
                  v-for="monthRow in table.kind === 'opening'
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
                </template>
                <template
                  v-for="monthRow in table.kind === 'month'
                    ? [table.rowsById.get(row.memberId)]
                    : []"
                  :key="table.period"
                >
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
                <template v-else-if="table.kind === 'opening'">
                  <td v-for="key in openingKeys" :key="key" class="num-cell">
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                  <td class="num-cell sheet-total">
                    {{ formatSheetNumber(openingTotal(table.totals)) }}
                  </td>
                </template>
                <template v-else>
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
