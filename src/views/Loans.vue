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
  type Loan,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const { loans, selectedYear, refresh } = useKoperasiStore();
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

const loanById = computed(() => new Map(loans.map((loan) => [loan.id, loan])));

// Spreadsheet rows follow loan numbers (L001, L002, ...). Derived columns:
// opening arrears = closing arrears - this month's payment obligation + amount paid;
// payment obligation through this month = opening arrears + this month's payment obligation.
const monthRows = computed(() =>
  monthlyLedger.loans
    .filter((row) => row.period === period.value)
    .map((row) => ({ ...row, loan: loanById.value.get(row.loanId) }))
    .filter((row): row is typeof row & { loan: Loan } => Boolean(row.loan))
    .sort((a, b) => a.loanId.localeCompare(b.loanId))
    .map((row, index) => {
      const priorArrearsPrincipal =
        row.arrearsPrincipal - row.scheduledPrincipal + row.principalPaid;
      const priorArrearsInterest =
        row.arrearsInterest - row.scheduledInterest + row.interestPaid;
      return {
        ...row,
        no: index + 1,
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
    }),
);
type MonthRow = (typeof monthRows.value)[number];

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
const totals = computed(() => {
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
  for (const row of monthRows.value as MonthRow[]) {
    for (const key of keys) sum[key] += row[key];
  }
  return sum;
});

const monthlyRate = (loan: Loan) =>
  `${(loan.rate / 12).toLocaleString("id-ID", { maximumFractionDigits: 2 })}%`;
const totalColumns = (key: string) =>
  key === "openingBalance" || key === "closingBalance";
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Pinjaman" :refresh="refresh" />
    <section
      v-if="!monthlyLedgerLoading && !monthlyLedger.periods.length"
      class="panel year-empty-state"
    >
      <CalendarX2 :size="36" />
      <strong>Belum ada data pinjaman untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain.</p>
    </section>
    <section v-else class="panel table-panel">
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
              <th rowspan="3" class="sheet-sticky sheet-name">Nama</th>
              <th rowspan="3">Plafond</th>
              <th rowspan="3" colspan="2">Bunga per-bulan</th>
              <th rowspan="3">PG/TN</th>
              <th rowspan="3">Jenis<br />Pinjaman</th>
              <th rowspan="3">Tanggal<br />Realisasi</th>
              <th rowspan="3">Tanggal<br />Jatuh Tempo</th>
              <th rowspan="3">Jangka<br />Waktu</th>
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
            </tr>
            <tr>
              <th colspan="6" class="sheet-group sheet-group--mutasi">
                Mutasi bulan {{ formatPeriod(period) }}
              </th>
            </tr>
            <tr>
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
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in monthRows" :key="row.loanId">
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
              <td>Bulanan</td>
              <td>{{ row.loan.realizationDate }}</td>
              <td>{{ row.loan.dueDate }}</td>
              <td class="num-cell">{{ row.loan.tenor }}</td>
              <td
                v-for="key in beforeMutation"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': totalColumns(key) }"
              >
                {{ formatSheetNumber(row[key]) }}
              </td>
              <td>{{ formatSheetDate(row.transactionDate) }}</td>
              <td v-for="key in mutation" :key="key" class="num-cell">
                {{ formatSheetNumber(row[key]) }}
              </td>
              <td
                v-for="key in afterMutation"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': totalColumns(key) }"
              >
                {{ formatSheetNumber(row[key]) }}
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH</td>
              <td class="num-cell">{{ formatSheetNumber(totals.plafond) }}</td>
              <td colspan="7"></td>
              <td
                v-for="key in beforeMutation"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': totalColumns(key) }"
              >
                {{ formatSheetNumber(totals[key]) }}
              </td>
              <td></td>
              <td v-for="key in mutation" :key="key" class="num-cell">
                {{ formatSheetNumber(totals[key]) }}
              </td>
              <td
                v-for="key in afterMutation"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': totalColumns(key) }"
              >
                {{ formatSheetNumber(totals[key]) }}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
  </div>
</template>
