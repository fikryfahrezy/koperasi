<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { CalendarX2, Percent, Search } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import { useVirtualSheetColumns } from "../composables/useVirtualSheetColumns";
import { useVirtualSheet } from "../composables/useVirtualSheet";
import IncentiveRatesDialog from "../components/IncentiveRatesDialog.vue";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import {
  formatPeriod,
  formatPreviousPeriod,
  formatSheetDate,
  formatSheetNumber,
  type Loan,
  LoanGroup,
  matchesMember,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";
import {
  loanTailLetters,
  loanLeadingLetters,
  loanMonthLetters,
} from "../sheet-columns";

const pageState = useWorkspacePageState("/loans");
const formatRate = (rate: number) =>
  `${(rate * 100).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;
const ratesOpen = ref(false);

const { loans, members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const period = pageState.field("period", () => "");
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
const groupFilter = pageState.field<LoanGroup | "">("groupFilter", () => "");
const groupOptions = [
  { value: "", label: "Semua" },
  { value: LoanGroup.Member, label: "Anggota PP BRI" },
  { value: LoanGroup.NonMember, label: "Non Anggota PP BRI" },
];
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

const loanById = computed(() => new Map(loans.map((loan) => [loan.id, loan])));
const memberNumbers = computed(
  () => new Map(members.map((member) => [member.id, member.memberNumber])),
);

// Spreadsheet rows follow loan numbers (L001, L002, ...). Derived columns:
// opening arrears = closing arrears - this month's payment obligation + amount paid;
// payment obligation through this month = opening arrears + this month's payment obligation.
const monthRowsForPeriod = (selectedPeriod: string) =>
  monthlyLedger.value.loans
    .filter((row) => row.period === selectedPeriod)
    .map((row) => ({ ...row, loan: loanById.value.get(row.loanId) }))
    .filter((row): row is typeof row & { loan: Loan } => Boolean(row.loan))
    .sort((a, b) => a.loanId.localeCompare(b.loanId))
    .map((row) => {
      const priorArrearsPrincipal =
        row.arrearsPrincipal - row.scheduledPrincipal + row.principalPaid;
      const priorArrearsInterest =
        row.arrearsInterest - row.scheduledInterest + row.interestPaid;
      return {
        ...row,
        plafond: row.loan.plafond,
        scheduledTotal: row.scheduledPrincipal + row.scheduledInterest,
        priorArrearsPrincipal,
        priorArrearsInterest,
        priorArrearsTotal: priorArrearsPrincipal + priorArrearsInterest,
        duePrincipal: priorArrearsPrincipal + row.scheduledPrincipal,
        dueInterest: priorArrearsInterest + row.scheduledInterest,
        dueTotal:
          priorArrearsPrincipal +
          row.scheduledPrincipal +
          priorArrearsInterest +
          row.scheduledInterest,
        cashIn: row.principalPaid + row.interestPaid + row.provision,
        arrearsTotal: row.arrearsPrincipal + row.arrearsInterest,
      };
    });

// Jumlah pemasukan per triwulan and Insentif Pinjaman (kolom EJ–EW) come from
// the backend, which also hides paid-off loans whose borrower has another loan
// running and moves their income to that loan (rules 1.10).
const quarterLabels = [
  "Jan s/d Mar",
  "Apr s/d Jun",
  "Jul s/d Sep",
  "Okt s/d Des",
] as const;
const incomeKeys = [
  ...quarterLabels.flatMap(
    (_, quarter) =>
      [
        `q${quarter + 1}Interest`,
        `q${quarter + 1}Provision`,
        `q${quarter + 1}Total`,
      ] as const,
  ),
  "incentiveRaw",
  "incentive",
] as const;
type IncomeKey = (typeof incomeKeys)[number];
// Kewajiban s/d bulan depan (kolom EE–EG after September): arrears plus next
// month's installment.
const nextDueKeys = [
  "nextDuePrincipal",
  "nextDueInterest",
  "nextDueTotal",
] as const;
// Kewajiban s/d bulan depan, Rekomendasi, then the Pemasukan columns.
const tailColumnCount = nextDueKeys.length + 1 + incomeKeys.length;
const nextPeriod = (period: string) => {
  const [year, month] = period.split("-").map(Number);
  return month === 12
    ? `${year + 1}-01`
    : `${year}-${String(month + 1).padStart(2, "0")}`;
};

const rowsForPeriod = (selectedPeriod: string) =>
  monthRowsForPeriod(selectedPeriod)
    .filter((row) => row.shown)
    .map((row) => {
      const income = {} as Record<IncomeKey, number>;
      row.incomeInterest.forEach((amount, quarter) => {
        const provision = row.incomeProvision[quarter] ?? 0;
        income[`q${quarter + 1}Interest` as IncomeKey] = amount;
        income[`q${quarter + 1}Provision` as IncomeKey] = provision;
        income[`q${quarter + 1}Total` as IncomeKey] = amount + provision;
      });
      income.incentiveRaw = row.incentiveRaw;
      income.incentive = row.incentive;
      return {
        ...row,
        ...income,
        nextDueTotal: row.nextDuePrincipal + row.nextDueInterest,
      };
    });
type MonthRow = ReturnType<typeof rowsForPeriod>[number];

// Numeric columns after the loan term, in spreadsheet order.
const beforeMutation = [
  "openingBalance",
  "scheduledPrincipal",
  "scheduledInterest",
  "scheduledTotal",
  "priorArrearsPrincipal",
  "priorArrearsInterest",
  "priorArrearsTotal",
  "duePrincipal",
  "dueInterest",
  "dueTotal",
] as const;
const mutation = [
  "cashIn",
  "disbursed",
  "principalPaid",
  "interestPaid",
  "provision",
] as const;
const afterMutation = [
  "closingBalance",
  "arrearsPrincipal",
  "arrearsInterest",
  "arrearsTotal",
] as const;
type TotalKey =
  | "plafond"
  | (typeof beforeMutation)[number]
  | (typeof mutation)[number]
  | (typeof afterMutation)[number]
  | IncomeKey
  | (typeof nextDueKeys)[number];
function sumRows(rows: MonthRow[]) {
  const keys: TotalKey[] = [
    "plafond",
    ...beforeMutation,
    ...mutation,
    ...afterMutation,
    ...incomeKeys,
    ...nextDueKeys,
  ];
  const sum = Object.fromEntries(keys.map((key) => [key, 0])) as Record<
    TotalKey,
    number
  >;
  for (const row of rows) {
    for (const key of keys) sum[key] += row[key];
  }
  return sum;
}
// Sections of PINJAMAN BULANAN; numbering restarts in each section, as in the sheet.
const sectionsForRows = (monthRows: MonthRow[]) =>
  [
    { group: LoanGroup.Member, title: "PINJAMAN ANGGOTA PP BRI" },
    { group: LoanGroup.NonMember, title: "PINJAMAN NON ANGGOTA PP BRI" },
  ]
    .filter(
      (section) => !groupFilter.value || section.group === groupFilter.value,
    )
    .map((section) => {
      const rows = monthRows
        .filter((row) => row.loan.loanGroup === section.group)
        .map((row, index) => ({ ...row, no: index + 1 }))
        .filter((row) =>
          matchesMember(
            debouncedQuery.value,
            row.loan.memberName,
            memberNumbers.value.get(row.loan.memberId),
          ),
        );
      return { ...section, rows, totals: sumRows(rows) };
    });
const periodTables = computed(() =>
  visiblePeriods.value.map((period) => {
    const sections = sectionsForRows(rowsForPeriod(period));
    return {
      period,
      sections,
      rowsById: new Map(
        sections
          .flatMap((section) => section.rows)
          .map((row) => [row.loanId, row]),
      ),
      totals: sumRows(sections.flatMap((section) => section.rows)),
    };
  }),
);

const sections = computed(() => {
  const rowsById = new Map<string, MonthRow>();
  for (const table of periodTables.value) {
    for (const row of table.rowsById.values()) rowsById.set(row.loanId, row);
  }
  return sectionsForRows(
    [...rowsById.values()].sort((a, b) => a.loanId.localeCompare(b.loanId)),
  );
});
const plafondTotal = computed(() =>
  sections.value.reduce((sum, section) => sum + section.totals.plafond, 0),
);
// The month groups, then one "Pemasukan" group as of the last month shown
// (JUMLAH PEMASUKAN per triwulan and Insentif Pinjaman in the sheet).
const columnGroups = computed(() => {
  const months = periodTables.value.map((table) => ({
    ...table,
    kind: "month" as const,
  }));
  const last = periodTables.value[periodTables.value.length - 1];
  return last
    ? [...months, { ...last, kind: "tail" as const, period: "tail" }]
    : months;
});
const incomePeriod = computed(
  () => periodTables.value[periodTables.value.length - 1]?.period ?? "",
);
const sectionTotals = (
  table: (typeof columnGroups.value)[number],
  group: LoanGroup,
) => table.sections.find((section) => section.group === group)!.totals;

const monthlyRate = (loan: Loan) =>
  `${(loan.rate / 12).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;
// No, Nama, Plafond, Bunga (2), PG/TN, Jenis, Realisasi, Jatuh Tempo, Jangka Waktu,
// the numeric blocks and the mutation date.
const monthColumnCount =
  beforeMutation.length + 1 + mutation.length + afterMutation.length;
const totalColumnCount = computed(
  () =>
    10 +
    monthColumnCount * periodTables.value.length +
    (periodTables.value.length ? tailColumnCount : 0),
);
const totalColumns = (key: string) =>
  key === "openingBalance" || key === "closingBalance";

type Section = (typeof sections.value)[number];
type SheetRow =
  | { kind: "heading" | "subtotal"; section: Section; key: string }
  | {
      kind: "loan";
      section: Section;
      row: Section["rows"][number];
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
  navigation: rowNavigation,
} = useVirtualSheet(
  sheetRows,
  (row) => row.key,
  pageState.tabId,
  pageState.active,
  (index) =>
    sheetRows.value[index]?.kind === "heading"
      ? [1, 1, totalColumnCount.value - 2]
      : sheetRows.value[index]?.kind === "loan"
        ? Array(totalColumnCount.value).fill(1)
        : [1, 1, 1, 7, ...Array(totalColumnCount.value - 10).fill(1)],
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
  (group) => (group.kind === "month" ? monthColumnCount : tailColumnCount),
  [56, 240, 120, 80, 80, 80, 100, 120, 120, 90],
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
        <button
          v-if="monthlyLedger.periods.length"
          class="button button--secondary"
          type="button"
          @click="ratesOpen = true"
        >
          <Percent :size="16" /> Insentif {{ selectedYear }}
        </button>
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
                  <th colspan="2" rowspan="2" class="sheet-group">
                    Insentif Pinjaman
                  </th>
                </template>
                <template v-else>
                  <th rowspan="3" class="sheet-total">
                    Saldo Posisi<br />{{ formatPreviousPeriod(period) }}
                  </th>
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Kewajiban setor tiap bulan
                  </th>
                  <th colspan="3" rowspan="2" class="sheet-group">
                    Tunggakan s/d {{ formatPreviousPeriod(period) }}
                  </th>
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
                  <th>
                    {{ formatRate(monthlyLedger.loanIncentiveRate) }}
                  </th>
                  <th>Pembulatan</th>
                </template>
                <template
                  v-for="block in kind === 'month' ? 3 : 0"
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
                <td>{{ item.row.loan.realizationDate }}</td>
                <td>{{ item.row.loan.dueDate }}</td>
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
                      <td v-for="key in incomeKeys" :key="key" class="num-cell">
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
                    <td v-for="key in incomeKeys" :key="key" class="num-cell">
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
                  <td v-for="key in incomeKeys" :key="key" class="num-cell">
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
