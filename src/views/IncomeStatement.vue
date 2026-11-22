<script setup lang="ts">
import { computed, watch } from "vue";
import ExportButton from "../components/ExportButton.vue";
import PageHeader from "../components/PageHeader.vue";
import PageTitleTabs from "../components/PageTitleTabs.vue";
import BreakdownSheet from "../components/BreakdownSheet.vue";
import UiSelect from "../components/UiSelect.vue";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import { balanceSheetAt, previousYearLedger } from "../balance-sheet";
import {
  type IncomeAmounts,
  incomeBreakdown,
  incomeStatement,
} from "../income-statement";
import { incomeStatementSheet } from "../income-statement-export";
import {
  Channel,
  formatPeriod,
  formatSheetNumber,
  useBalanceSheetDetails,
  useBalanceSheetItems,
  useCashBook,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

// LABA RUGI of BUKU KAS HARIAN: Bunga + Provisi less the Buku Kas biaya, from
// January to the month shown, beside 31 Desember of the year before. The
// numbers are the Laba Bersih of the Neraca.
const pageState = useWorkspacePageState("/income-statement");

const IncomeView = { Sheet: "sheet", Breakdown: "breakdown" } as const;
type IncomeView = (typeof IncomeView)[keyof typeof IncomeView];

const view = pageState.field<IncomeView>("view", () => IncomeView.Sheet);

const viewTabs = [
  { value: IncomeView.Sheet, label: "Laba Rugi" },
  { value: IncomeView.Breakdown, label: "Rincian" },
];
const { loans, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();
const { cashBook } = useCashBook(Channel.Cash);
const { balanceSheetItems } = useBalanceSheetItems();
const { balanceSheetDetails } = useBalanceSheetDetails();
const { monthlyLedger: previousLedger } = useMonthlyLedger(-1);
const { cashBook: previousCashBook } = useCashBook(Channel.Cash, -1);
const { balanceSheetItems: previousItems } = useBalanceSheetItems(-1);
const { balanceSheetDetails: previousDetails } = useBalanceSheetDetails(-1);

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

const currentSheet = computed(() =>
  balanceSheetAt(
    selectedPeriod.value,
    monthlyLedger.value,
    cashBook.value,
    loans,
    balanceSheetItems.value,
    balanceSheetDetails.value,
  ),
);

const statement = computed(() =>
  incomeStatement([
    balanceSheetAt(
      previousPeriod.value,
      previousYearLedger(
        previousLedger.value,
        monthlyLedger.value,
        previousPeriod.value,
      ),
      previousCashBook.value,
      loans,
      previousItems.value,
      previousDetails.value,
    ),
    currentSheet.value,
  ]),
);

// Tab Rincian: the month shown, Pendapatan beside Biaya as on the sheet.
const breakdown = computed(() => incomeBreakdown(currentSheet.value));

const breakdownSides = computed(() => {
  const b = breakdown.value;

  return [
    {
      title: "PENDAPATAN",
      sections: [
        {
          title: "",
          lines: b.revenue,
          // The side's total is the section's.
          totalLabel: "",
          total: b.revenueTotal,
        },
      ],
      totalLabel: "TOTAL PENDAPATAN",
      total: b.revenueTotal,
    },
    {
      title: "BIAYA",
      sections: [
        {
          title: "",
          lines: b.expenses,
          totalLabel: "TOTAL BIAYA",
          total: b.expenseTotal,
        },
      ],
      totalLabel: "LABA",
      total: b.profit,
    },
  ];
});

type Cell =
  | { kind: "heading" | "blank"; label?: string }
  | { kind: "line" | "total"; label: string; amounts: IncomeAmounts };

// Pendapatan on the left, Biaya on the right, then TOTAL on both sides.
const rows = computed(() => {
  const s = statement.value;
  const leftCells: Cell[] = [
    { kind: "heading", label: "PENDAPATAN" },
    ...s.revenue.map((line) => ({ kind: "line" as const, ...line })),
  ];
  const rightCells: Cell[] = [
    { kind: "heading", label: "BIAYA" },
    ...s.expenses.map((line) => ({ kind: "line" as const, ...line })),
    { kind: "total", label: "TOTAL BIAYA", amounts: s.expenseTotal },
    { kind: "blank" },
    { kind: "total", label: "LABA", amounts: s.profit },
  ];
  const length = Math.max(leftCells.length, rightCells.length);
  const blank: Cell = { kind: "blank" };

  return [
    ...Array.from({ length }, (_, index) => ({
      left: leftCells[index] ?? blank,
      right: rightCells[index] ?? blank,
    })),
    { left: blank, right: blank },
    {
      left: { kind: "total", label: "TOTAL", amounts: s.total } as Cell,
      right: { kind: "total", label: "TOTAL", amounts: s.total } as Cell,
    },
  ];
});

const lastDay = (value: string) => {
  const [year, month] = value.split("-").map(Number);
  if (!year) return "";
  return `${new Date(year, month, 0).getDate()} ${formatPeriod(value)}`;
};

const columnLabel = (value: string) => {
  const [year, month] = value.split("-").map(Number);

  return new Date(year, month - 1, 1)
    .toLocaleDateString("en-US", { month: "short", year: "2-digit" })
    .replace(" ", "-");
};

const columns = computed(() => [
  columnLabel(previousPeriod.value),
  columnLabel(selectedPeriod.value),
]);

const amountText = (value: number | null) =>
  value === null ? "-" : formatSheetNumber(value);

const COMPANY = "KOPERASI PENSIUNAN BRI BINA SEJAHTERA";

const exportSheet = () => {
  const position = lastDay(selectedPeriod.value).toUpperCase();

  return incomeStatementSheet({
    company: COMPANY,
    position,
    periods: [previousPeriod.value, selectedPeriod.value],
    statement: statement.value,
    fileName: `LABA RUGI POSISI ${position}.xlsx`,
  });
};
</script>

<template>
  <div class="page-stack income-page">
    <PageHeader title="Laba Rugi" :refresh="refresh">
      <template #title>
        <PageTitleTabs v-model="view" label="Laba Rugi" :tabs="viewTabs" />
      </template>
      <template #before-actions>
        <UiSelect
          v-if="periods.length"
          v-model="period"
          :options="periodOptions"
          label="Bulan"
          aria-label="Bulan"
          variant="toolbar"
        />
      </template>
      <template #actions>
        <ExportButton
          v-if="periods.length && view === IncomeView.Sheet"
          :sheet="exportSheet"
        />
      </template>
    </PageHeader>

    <section
      v-if="monthlyLedgerLoading && !periods.length"
      class="panel income-empty"
    >
      Memuat data…
    </section>
    <section v-else-if="!periods.length" class="panel income-empty">
      <strong>Belum ada data untuk {{ selectedYear }}</strong>
    </section>

    <BreakdownSheet
      v-else-if="view === IncomeView.Breakdown"
      title="RINCIAN LABA - RUGI"
      :position="lastDay(selectedPeriod).toUpperCase()"
      :sides="breakdownSides"
    />

    <section v-else class="panel income-sheet">
      <header class="income-sheet__title">
        <h2>{{ COMPANY }}</h2>
        <strong>LABA - RUGI</strong>
        <u>POSISI {{ lastDay(selectedPeriod).toUpperCase() }}</u>
      </header>

      <div class="income-sheet__scroll">
        <table class="income-table">
          <colgroup>
            <col class="income-table__label" />
            <col class="income-table__amount" />
            <col class="income-table__amount" />
            <col class="income-table__label" />
            <col class="income-table__amount" />
            <col class="income-table__amount" />
          </colgroup>
          <thead>
            <tr>
              <th rowspan="2">URAIAN</th>
              <th colspan="2">TAHUN</th>
              <th rowspan="2">URAIAN</th>
              <th colspan="2">TAHUN</th>
            </tr>
            <tr>
              <th v-for="label in columns" :key="`l-${label}`">{{ label }}</th>
              <th v-for="label in columns" :key="`r-${label}`">{{ label }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(row, index) in rows"
              :key="index"
              :class="{ 'income-table__grand': index === rows.length - 1 }"
            >
              <template
                v-for="(cell, side) in [row.left, row.right]"
                :key="side"
              >
                <template v-if="cell.kind === 'line' || cell.kind === 'total'">
                  <td :class="{ 'income-table__total': cell.kind === 'total' }">
                    {{ cell.label }}
                  </td>
                  <td
                    v-for="(amount, column) in cell.amounts"
                    :key="column"
                    class="num-cell"
                    :class="{ 'income-table__total': cell.kind === 'total' }"
                  >
                    {{ amountText(amount) }}
                  </td>
                </template>
                <template v-else>
                  <td class="income-table__heading">{{ cell.label }}</td>
                  <td></td>
                  <td></td>
                </template>
              </template>
            </tr>
          </tbody>
        </table>
      </div>

      <p
        v-for="(typed, column) in statement.typedProfit"
        v-show="typed !== undefined"
        :key="column"
        class="income-check"
        role="status"
      >
        Laba {{ columns[column] }} di Neraca diisi
        <strong>{{ formatSheetNumber(typed ?? 0) }}</strong
        >, berbeda dari hitungan di sini.
      </p>
    </section>
  </div>
</template>

<style scoped>
.income-page {
  gap: 12px;
  padding-bottom: 16px;
}

.income-empty {
  padding: 32px;
  color: var(--muted);
  text-align: center;
}

.income-sheet {
  padding: 18px 20px;
  min-width: 0;
}

.income-sheet__title {
  display: grid;
  justify-items: center;
  gap: 4px;
  margin-bottom: 14px;
  font-size: 13px;
}

.income-sheet__title h2 {
  margin: 0;
  color: var(--green);
  font-size: 17px;
  font-weight: 600;
}

.income-sheet__scroll {
  overflow-x: auto;
}

.income-table {
  width: 100%;
  min-width: 900px;
  border-collapse: collapse;
  font-size: 12px;
}

.income-table__label {
  width: 26%;
}

.income-table__amount {
  width: 12%;
}

.income-table th,
.income-table td {
  padding: 5px 7px;
  border: 1px solid var(--line-strong);
}

.income-table thead th {
  font-style: italic;
  font-weight: 800;
  text-align: center;
}

.income-table tbody td:nth-child(4),
.income-table thead tr:first-child th:nth-child(3) {
  border-left: 2px solid var(--ink);
}

.income-table__heading,
.income-table__total {
  font-weight: 800;
}

.income-table__grand td {
  border-top: 2px solid var(--ink);
}

.income-check {
  margin: 10px 0 0;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-left: 3px solid var(--amber);
  background: var(--amber-soft);
  font-size: 12px;
}
</style>
