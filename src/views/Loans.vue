<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CalendarX2, Search } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import { useProgressiveRows } from "../composables/useProgressiveRows";
import {
  formatPeriod,
  formatPreviousPeriod,
  formatSheetDate,
  formatSheetNumber,
  type Loan,
  LoanGroup,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const { loans, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();

const period = ref("");
const query = ref("");
const groupFilter = ref<LoanGroup | "">("");
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

// Spreadsheet rows follow loan numbers (L001, L002, ...). Derived columns:
// opening arrears = closing arrears - this month's payment obligation + amount paid;
// payment obligation through this month = opening arrears + this month's payment obligation.
const rowsForPeriod = (selectedPeriod: string) =>
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
  | (typeof afterMutation)[number];
function sumRows(rows: MonthRow[]) {
  const keys: TotalKey[] = [
    "plafond",
    ...beforeMutation,
    ...mutation,
    ...afterMutation,
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
          row.loan.memberName
            .toLowerCase()
            .includes(query.value.trim().toLowerCase()),
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
const sectionTotals = (
  table: (typeof periodTables.value)[number],
  group: LoanGroup,
) => table.sections.find((section) => section.group === group)!.totals;

const monthlyRate = (loan: Loan) =>
  `${(loan.rate / 12).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;
// No, Nama, Plafond, Bunga (2), PG/TN, Jenis, Realisasi, Jatuh Tempo, Jangka Waktu,
// the numeric blocks and the mutation date.
const monthColumnCount =
  beforeMutation.length + 1 + mutation.length + afterMutation.length;
const totalColumnCount = computed(
  () => 10 + monthColumnCount * periodTables.value.length,
);
const totalColumns = (key: string) =>
  key === "openingBalance" || key === "closingBalance";
const allRows = computed(() =>
  sections.value.flatMap((section) => section.rows),
);
const { renderedRowCount, totalRowCount, renderingRows } = useProgressiveRows(
  () => allRows.value,
  () => totalColumnCount.value,
);
const renderedSections = computed(() => {
  let remaining = renderedRowCount.value;
  return sections.value.map((section) => {
    const rows = section.rows.slice(0, Math.max(0, remaining));
    remaining -= section.rows.length;
    return { ...section, rows };
  });
});
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Pinjaman" :refresh="refresh">
      <template #title-meta>
        <span
          v-if="monthlyLedgerLoading || renderingRows"
          class="sheet-loading"
          role="status"
        >
          {{
            monthlyLedgerLoading
              ? "Memuat pinjaman…"
              : `Menampilkan ${renderedRowCount} dari ${totalRowCount} baris…`
          }}
        </span>
      </template>
      <template #before-actions>
        <label v-if="monthlyLedger.periods.length" class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            type="search"
            aria-label="Cari nama peminjam"
            placeholder="Cari nama peminjam..."
          />
        </label>
        <UiSelect
          v-if="monthlyLedger.periods.length"
          v-model="period"
          :options="periodOptions"
          aria-label="Bulan"
          variant="toolbar"
        />
        <UiSelect
          v-if="monthlyLedger.periods.length"
          v-model="groupFilter"
          :options="groupOptions"
          aria-label="Kelompok pinjaman"
          variant="toolbar"
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
    <section v-else class="panel table-panel" :aria-busy="renderingRows">
      <div class="data-table-wrap sheet-wrap">
        <table v-table-navigation class="sheet-table">
          <thead>
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
              <template v-for="{ period } in periodTables" :key="period">
                <th :colspan="monthColumnCount" class="sheet-group">
                  {{ formatPeriod(period) }}
                </th>
              </template>
            </tr>
            <tr>
              <template v-for="{ period } in periodTables" :key="period">
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
            </tr>
            <tr>
              <template v-for="{ period } in periodTables" :key="period">
                <th colspan="6" class="sheet-group sheet-group--mutasi">
                  Mutasi bulan {{ formatPeriod(period) }}
                </th>
              </template>
            </tr>
            <tr>
              <template v-for="{ period } in periodTables" :key="period">
                <template v-for="block in 3" :key="block">
                  <th>Pokok</th>
                  <th>Bunga</th>
                  <th>Jumlah</th>
                </template>
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
            </tr>
          </thead>
          <tbody v-for="section in renderedSections" :key="section.group">
            <tr>
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">
                <strong>{{ section.title }}</strong>
              </td>
              <td :colspan="totalColumnCount - 2"></td>
            </tr>
            <tr
              v-for="row in section.rows"
              :key="row.loanId"
              v-memo="[row, periodTables]"
            >
              <td class="sheet-sticky sheet-no">{{ row.no }}</td>
              <td class="sheet-sticky sheet-name">
                <strong>{{ row.loan.memberName }}</strong>
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(row.loan.plafond) }}
              </td>
              <td class="num-cell">{{ monthlyRate(row.loan) }}</td>
              <td>{{ row.loan.interestType.toLowerCase() }}</td>
              <td>{{ row.loan.guarantee }}</td>
              <td>{{ row.loan.loanType ?? "" }}</td>
              <td>{{ row.loan.realizationDate }}</td>
              <td>{{ row.loan.dueDate }}</td>
              <td class="num-cell">{{ row.loan.tenor }}</td>
              <template v-for="table in periodTables" :key="table.period">
                <template
                  v-for="monthRow in [table.rowsById.get(row.loanId)]"
                  :key="table.period"
                >
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
                      monthRow ? formatSheetDate(monthRow.transactionDate) : "-"
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
            </tr>
            <tr>
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">SUB JUMLAH</td>
              <td class="num-cell">
                {{ formatSheetNumber(section.totals.plafond) }}
              </td>
              <td colspan="7"></td>
              <template v-for="table in periodTables" :key="table.period">
                <td
                  v-for="key in beforeMutation"
                  :key="key"
                  class="num-cell"
                  :class="{ 'sheet-total': totalColumns(key) }"
                >
                  {{
                    formatSheetNumber(sectionTotals(table, section.group)[key])
                  }}
                </td>
                <td></td>
                <td v-for="key in mutation" :key="key" class="num-cell">
                  {{
                    formatSheetNumber(sectionTotals(table, section.group)[key])
                  }}
                </td>
                <td
                  v-for="key in afterMutation"
                  :key="key"
                  class="num-cell"
                  :class="{ 'sheet-total': totalColumns(key) }"
                >
                  {{
                    formatSheetNumber(sectionTotals(table, section.group)[key])
                  }}
                </td>
              </template>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH TOTAL</td>
              <td class="num-cell">
                {{ formatSheetNumber(plafondTotal) }}
              </td>
              <td colspan="7"></td>
              <template v-for="table in periodTables" :key="table.period">
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
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
  </div>
</template>
