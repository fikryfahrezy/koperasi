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
  LoanGroup,
  matchesMember,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

import {
  afterMutation,
  beforeMutation,
  incomeKeys,
  leadingColumnWidths,
  loanRowsForPeriod,
  loanSections,
  type LoanSection,
  monthColumnCount,
  monthlyRate,
  mutation,
  nextDueKeys,
  nextPeriod,
  openingColumnCount,
  openingKeys,
  quarterLabels,
  sumLoanRows,
  tailColumnCount,
} from "../loan-sheet";

import {
  loanTailLetters,
  loanLeadingLetters,
  loanMonthLetters,
  loanOpeningLetters,
} from "../sheet-columns";

const pageState = useWorkspacePageState("/loans");
const ratesOpen = ref(false);

const { loans, members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const { period, periodOptions, visiblePeriods } = usePeriodFilter(
  pageState,
  () => monthlyLedger.value.periods,
);

const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
const groupFilter = pageState.field<LoanGroup | "">("groupFilter", () => "");

const groupOptions = [
  { value: "", label: "Semua" },
  { value: LoanGroup.Member, label: "Anggota PP BRI" },
  { value: LoanGroup.NonMember, label: "Non Anggota PP BRI" },
];

const loanById = computed(() => new Map(loans.map((loan) => [loan.id, loan])));

const memberNumbers = computed(
  () => new Map(members.map((member) => [member.id, member.memberNumber])),
);

// The export holds the month, the section and the loans the page shows.
const exportFilter = computed(() => ({
  period: period.value,
  loanGroup: groupFilter.value,
  ids: debouncedQuery.value.trim()
    ? loans
        .filter((loan) =>
          matchesMember(
            debouncedQuery.value,
            loan.memberName,
            memberNumbers.value.get(loan.memberId),
          ),
        )
        .map((loan) => loan.id)
    : undefined,
}));

const sectionsOf = (rows: Parameters<typeof loanSections>[0]) =>
  loanSections(rows, {
    group: groupFilter.value,
    query: debouncedQuery.value,
    memberNumbers: memberNumbers.value,
  });

const periodTables = computed(() =>
  visiblePeriods.value.map((period) => {
    const sections = sectionsOf(
      loanRowsForPeriod(monthlyLedger.value.loans, loanById.value, period),
    );

    const rows = sections.flatMap((section) => section.rows);

    return {
      period,
      sections,
      rowsById: new Map(rows.map((row) => [row.loanId, row])),
      totals: sumLoanRows(rows),
    };
  }),
);

// The rows are those of the last month shown: every loan that has run up to
// that month. Each month's totals still count every loan of that month.
const sections = computed(() => {
  const last = periodTables.value[periodTables.value.length - 1];

  return sectionsOf(
    last
      ? [...last.rowsById.values()].sort((a, b) =>
          a.loanId.localeCompare(b.loanId),
        )
      : [],
  );
});

const plafondTotal = computed(() =>
  sections.value.reduce((sum, section) => sum + section.totals.plafond, 0),
);

// The opening columns of the first month shown, the month groups, then one
// "Pemasukan" group as of the last month shown (JUMLAH PEMASUKAN per triwulan
// and Insentif Pinjaman in the sheet).
const columnGroups = computed(() => {
  const months = periodTables.value.map((table) => ({
    ...table,
    kind: "month" as "opening" | "month" | "tail",
  }));

  const first = periodTables.value[0];
  const last = periodTables.value[periodTables.value.length - 1];

  return first && last
    ? [
        { ...first, kind: "opening" as const, period: "opening" },
        ...months,
        { ...last, kind: "tail" as const, period: "tail" },
      ]
    : months;
});

const firstPeriod = computed(() => periodTables.value[0]?.period ?? "");

const incomePeriod = computed(
  () => periodTables.value[periodTables.value.length - 1]?.period ?? "",
);

const sectionTotals = (
  table: (typeof columnGroups.value)[number],
  group: LoanGroup,
) => table.sections.find((section) => section.group === group)!.totals;

const totalColumnCount = computed(
  () =>
    leadingColumnWidths.length +
    monthColumnCount * periodTables.value.length +
    (periodTables.value.length ? openingColumnCount + tailColumnCount : 0),
);

const totalColumns = (key: string) =>
  key === "openingBalance" || key === "closingBalance";

type SheetRow =
  | { kind: "heading" | "subtotal"; section: LoanSection; key: string }
  | {
      kind: "loan";
      section: LoanSection;
      row: LoanSection["rows"][number];
      key: string;
    };

const sheetRows = computed<SheetRow[]>(() =>
  sections.value.flatMap((section) => [
    { kind: "heading" as const, section, key: `${section.group}:heading` },
    ...section.rows.map((row) => ({
      kind: "loan" as const,
      section,
      row,
      key: row.loanId,
    })),
    { kind: "subtotal" as const, section, key: `${section.group}:subtotal` },
  ]),
);

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
  rows: sheetRows,
  rowKey: (row) => row.key,
  columnSpans: (index) =>
    sheetRows.value[index]?.kind === "heading"
      ? [1, 1, totalColumnCount.value - 2]
      : sheetRows.value[index]?.kind === "loan"
        ? Array(totalColumnCount.value).fill(1)
        : [1, 1, 1, 7, ...Array(totalColumnCount.value - 10).fill(1)],
  groups: columnGroups,
  groupColumns: (group) =>
    group.kind === "month"
      ? monthColumnCount
      : group.kind === "opening"
        ? openingColumnCount
        : tailColumnCount,
  leadingWidths: leadingColumnWidths,
});
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Pinjaman" :refresh="refresh">
      <template #title-meta>
        <span v-if="monthlyLedgerLoading" class="sheet-loading" role="status">
          Memuat pinjaman…
        </span>
      </template>
      <template #before-actions>
        <label v-if="monthlyLedger.periods.length" class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            type="search"
            aria-label="Cari nama peminjam atau ID anggota"
            placeholder="Cari nama peminjam atau ID anggota..."
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
        <UiSelect
          v-if="monthlyLedger.periods.length"
          v-model="groupFilter"
          :options="groupOptions"
          label="Kelompok pinjaman"
          aria-label="Kelompok pinjaman"
          variant="toolbar"
        />
      </template>
      <template #actions>
        <button
          v-if="monthlyLedger.periods.length"
          class="button button--secondary"
          type="button"
          @click="ratesOpen = true"
        >
          <Percent :size="16" /> Insentif {{ selectedYear }}
        </button>
        <ExportButton
          v-if="monthlyLedger.periods.length"
          kind="loans"
          :filter="exportFilter"
        />
      </template>
    </PageHeader>
    <section
      v-if="monthlyLedgerLoading"
      class="panel backend-state"
      aria-busy="true"
    >
      <strong>Memuat pinjaman…</strong>
    </section>
    <section
      v-else-if="!monthlyLedger.periods.length"
      class="panel year-empty-state"
    >
      <CalendarX2 :size="36" />
      <strong>Belum ada data pinjaman untuk {{ selectedYear }}</strong>
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
          :aria-rowcount="sheetRows.length + 6"
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
              <th
                v-for="(letter, index) in loanLeadingLetters"
                :key="index"
                :class="{
                  'sheet-sticky': index < 2,
                  'sheet-no': index === 0,
                  'sheet-name': index === 1,
                }"
              >
                {{ letter }}
              </th>
              <th
                v-if="leftColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
              <template
                v-for="{ period, kind } in renderedPeriodTables"
                :key="period"
              >
                <th
                  v-for="(letter, index) in kind === 'month'
                    ? loanMonthLetters(period)
                    : kind === 'opening'
                      ? loanOpeningLetters
                      : loanTailLetters(nextPeriod(incomePeriod))"
                  :key="index"
                >
                  {{ letter }}
                </th>
              </template>
              <th
                v-if="rightColumns"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
            </tr>
            <tr>
              <th rowspan="4" class="sheet-sticky sheet-no">No</th>
              <th rowspan="4" class="sheet-sticky sheet-name">Nama</th>
              <th rowspan="4">Plafond</th>
              <th rowspan="4" colspan="2">Bunga per-bulan</th>
              <th rowspan="4">PG/TN</th>
              <th rowspan="4">Jenis<br />Pinjaman</th>
              <th rowspan="4">Tanggal<br />Realisasi</th>
              <th rowspan="4">Tanggal<br />Jatuh Tempo</th>
              <th rowspan="4">Jangka<br />Waktu</th>
              <th
                v-if="leftColumns"
                rowspan="4"
                class="sheet-column-spacer"
                aria-hidden="true"
              ></th>
              <template
                v-for="{ period, kind } in renderedPeriodTables"
                :key="period"
              >
                <th
                  v-if="kind === 'month'"
                  :colspan="monthColumnCount"
                  class="sheet-group"
                  :data-period="period"
                >
                  {{ formatPeriod(period) }}
                </th>
                <template v-else-if="kind === 'opening'">
                  <th rowspan="4" class="sheet-total">
                    Saldo Posisi<br />{{ formatPreviousPeriod(firstPeriod) }}
                  </th>
                  <th colspan="3" rowspan="3" class="sheet-group">
                    Kewajiban setor tiap bulan
                  </th>
                  <th colspan="3" rowspan="3" class="sheet-group">
                    Tunggakan s/d {{ formatPreviousPeriod(firstPeriod) }}
                  </th>
                </template>
                <template v-else>
                  <th colspan="3" rowspan="3" class="sheet-group">
                    Kewajiban s/d {{ formatPeriod(nextPeriod(incomePeriod)) }}
                  </th>
                  <th rowspan="4">Rekomendasi</th>
                  <th :colspan="incomeKeys.length" class="sheet-group">
                    Pemasukan s/d {{ formatPeriod(incomePeriod) }}
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
                v-for="{ period, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'tail'">
                  <th
                    v-for="label in quarterLabels"
                    :key="label"
                    colspan="3"
                    rowspan="2"
                    class="sheet-group"
                  >
                    Jumlah pemasukan {{ label }}
                  </th>
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Total pemasukan tahun {{ selectedYear }}
                  </th>
                  <th colspan="2" rowspan="2" class="sheet-group">
                    Insentif Pinjaman
                  </th>
                </template>
                <template v-else-if="kind === 'month'">
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Kewajiban setor s/d {{ formatPeriod(period) }}
                  </th>
                  <th colspan="6" class="sheet-group sheet-group--mutasi">
                    Bulan {{ formatPeriod(period) }}
                  </th>
                  <th rowspan="3" class="sheet-total">
                    Saldo Posisi<br />{{ formatPeriod(period) }}
                  </th>
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Tunggakan s/d {{ formatPeriod(period) }}
                  </th>
                </template>
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, kind } in renderedPeriodTables"
                :key="period"
              >
                <th
                  v-if="kind === 'month'"
                  colspan="6"
                  class="sheet-group sheet-group--mutasi"
                >
                  Mutasi bulan {{ formatPeriod(period) }}
                </th>
              </template>
            </tr>
            <tr>
              <template
                v-for="{ period, kind } in renderedPeriodTables"
                :key="period"
              >
                <template v-if="kind === 'tail'">
                  <th>Pokok</th>
                  <th>Bunga</th>
                  <th>Jumlah</th>
                  <template v-for="label in quarterLabels" :key="label">
                    <th>Bunga</th>
                    <th>Provisi</th>
                    <th>Jumlah</th>
                  </template>
                  <th>Bunga</th>
                  <th>Provisi</th>
                  <th class="sheet-total">Jumlah</th>
                  <th>
                    {{ formatRate(monthlyLedger.loanIncentiveRate) }}
                  </th>
                  <th>Pembulatan</th>
                </template>
                <template
                  v-for="block in kind === 'opening'
                    ? 2
                    : kind === 'month'
                      ? 1
                      : 0"
                  :key="block"
                >
                  <th>Pokok</th>
                  <th>Bunga</th>
                  <th>Jumlah</th>
                </template>
                <template v-if="kind === 'month'">
                  <th>Tanggal</th>
                  <th>Jumlah</th>
                  <th>Pokok Debet</th>
                  <th>Pokok Kredit</th>
                  <th>Bunga</th>
                  <th>Provisi</th>
                  <th>Pokok</th>
                  <th>Bunga</th>
                  <th>Jumlah</th>
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
            <template
              v-for="{ row: item, index } in renderedRows"
              :key="item.key"
            >
              <tr
                v-if="item.kind === 'heading'"
                :data-virtual-row="index"
                :aria-rowindex="index + 6"
              >
                <td class="sheet-sticky sheet-no"></td>
                <td class="sheet-sticky sheet-name">
                  <strong>{{ item.section.title }}</strong>
                </td>
                <td
                  :colspan="renderedColumnCount - 2"
                  :data-virtual-span="totalColumnCount - 2"
                ></td>
              </tr>
              <tr
                v-else-if="item.kind === 'loan'"
                :data-virtual-row="index"
                :aria-rowindex="index + 6"
              >
                <td class="sheet-sticky sheet-no">{{ item.row.no }}</td>
                <td class="sheet-sticky sheet-name">
                  <strong>{{ item.row.loan.memberName }}</strong>
                </td>
                <td class="num-cell">
                  {{ formatSheetNumber(item.row.loan.plafond) }}
                </td>
                <td class="num-cell">{{ monthlyRate(item.row.loan) }}</td>
                <td>{{ item.row.loan.interestType.toLowerCase() }}</td>
                <td>{{ item.row.loan.guarantee }}</td>
                <td>{{ item.row.loan.loanType ?? "" }}</td>
                <td>{{ formatSheetDate(item.row.loan.realizationDate) }}</td>
                <td>{{ formatSheetDate(item.row.loan.dueDate) }}</td>
                <td class="num-cell">{{ item.row.loan.tenor }}</td>
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
                  <template
                    v-for="monthRow in [table.rowsById.get(item.row.loanId)]"
                    :key="table.period"
                  >
                    <template v-if="table.kind === 'tail'">
                      <td
                        v-for="key in nextDueKeys"
                        :key="key"
                        class="num-cell"
                      >
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                      <td>{{ item.row.loan.recommender }}</td>
                      <td
                        v-for="key in incomeKeys"
                        :key="key"
                        class="num-cell"
                        :class="{ 'sheet-total': key === 'incomeTotal' }"
                      >
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                    </template>
                    <template v-else-if="table.kind === 'opening'">
                      <td
                        v-for="key in openingKeys"
                        :key="key"
                        class="num-cell"
                        :class="{ 'sheet-total': totalColumns(key) }"
                      >
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                    </template>
                    <template v-else>
                      <td
                        v-for="key in beforeMutation"
                        :key="key"
                        class="num-cell"
                        :class="{ 'sheet-total': totalColumns(key) }"
                      >
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                      <td>
                        {{
                          monthRow
                            ? formatSheetDate(monthRow.transactionDate)
                            : "-"
                        }}
                      </td>
                      <td v-for="key in mutation" :key="key" class="num-cell">
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                      <td
                        v-for="key in afterMutation"
                        :key="key"
                        class="num-cell"
                        :class="{ 'sheet-total': totalColumns(key) }"
                      >
                        {{ monthRow ? formatSheetNumber(monthRow[key]) : "-" }}
                      </td>
                    </template>
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
              <tr v-else :data-virtual-row="index" :aria-rowindex="index + 6">
                <td class="sheet-sticky sheet-no"></td>
                <td class="sheet-sticky sheet-name">SUB JUMLAH</td>
                <td class="num-cell">
                  {{ formatSheetNumber(item.section.totals.plafond) }}
                </td>
                <td colspan="7"></td>
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
                    <td v-for="key in nextDueKeys" :key="key" class="num-cell">
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
                    </td>
                    <td></td>
                    <td
                      v-for="key in incomeKeys"
                      :key="key"
                      class="num-cell"
                      :class="{ 'sheet-total': key === 'incomeTotal' }"
                    >
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
                    </td>
                  </template>
                  <template v-else-if="table.kind === 'opening'">
                    <td
                      v-for="key in openingKeys"
                      :key="key"
                      class="num-cell"
                      :class="{ 'sheet-total': totalColumns(key) }"
                    >
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
                    </td>
                  </template>
                  <template v-else>
                    <td
                      v-for="key in beforeMutation"
                      :key="key"
                      class="num-cell"
                      :class="{ 'sheet-total': totalColumns(key) }"
                    >
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
                    </td>
                    <td></td>
                    <td v-for="key in mutation" :key="key" class="num-cell">
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
                    </td>
                    <td
                      v-for="key in afterMutation"
                      :key="key"
                      class="num-cell"
                      :class="{ 'sheet-total': totalColumns(key) }"
                    >
                      {{
                        formatSheetNumber(
                          sectionTotals(table, item.section.group)[key],
                        )
                      }}
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
            </template>
            <tr v-if="paddingBottom" class="sheet-spacer" aria-hidden="true">
              <td
                :colspan="renderedColumnCount"
                :style="{ height: `${paddingBottom}px` }"
              ></td>
            </tr>
          </tbody>
          <tfoot>
            <tr
              :data-virtual-row="sheetRows.length"
              :aria-rowindex="sheetRows.length + 6"
            >
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH TOTAL</td>
              <td class="num-cell">
                {{ formatSheetNumber(plafondTotal) }}
              </td>
              <td colspan="7"></td>
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
                  <td v-for="key in nextDueKeys" :key="key" class="num-cell">
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                  <td></td>
                  <td
                    v-for="key in incomeKeys"
                    :key="key"
                    class="num-cell"
                    :class="{ 'sheet-total': key === 'incomeTotal' }"
                  >
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                </template>
                <template v-else-if="table.kind === 'opening'">
                  <td
                    v-for="key in openingKeys"
                    :key="key"
                    class="num-cell"
                    :class="{ 'sheet-total': totalColumns(key) }"
                  >
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                </template>
                <template v-else>
                  <td
                    v-for="key in beforeMutation"
                    :key="key"
                    class="num-cell"
                    :class="{ 'sheet-total': totalColumns(key) }"
                  >
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                  <td></td>
                  <td v-for="key in mutation" :key="key" class="num-cell">
                    {{ formatSheetNumber(table.totals[key]) }}
                  </td>
                  <td
                    v-for="key in afterMutation"
                    :key="key"
                    class="num-cell"
                    :class="{ 'sheet-total': totalColumns(key) }"
                  >
                    {{ formatSheetNumber(table.totals[key]) }}
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
    <IncentiveRatesDialog v-model:open="ratesOpen" kind="loan" />
  </div>
</template>
