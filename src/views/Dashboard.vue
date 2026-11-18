<script setup lang="ts">
import { computed } from "vue";
import PageHeader from "../components/PageHeader.vue";
import { cashAndBankAt } from "../balance-sheet";

import {
  Channel,
  formatCurrency,
  formatPeriod,
  formatSheetNumber,
  LoanGroup,
  type LoanMonth,
  useCashBook,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const { loans, members, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();
const { cashBook } = useCashBook(Channel.Cash);

// The ledger runs through the current month, so its last period is "now".
const periods = computed(() => [...monthlyLedger.value.periods].sort());

const lastPeriod = computed(
  () => periods.value[periods.value.length - 1] ?? "",
);

const cashAndBank = computed(() =>
  cashAndBankAt(lastPeriod.value, cashBook.value),
);

const loanById = computed(() => new Map(loans.map((loan) => [loan.id, loan])));

const memberById = computed(
  () => new Map(members.map((member) => [member.id, member])),
);

const loanRows = computed(() =>
  monthlyLedger.value.loans.filter((row) => row.period === lastPeriod.value),
);

const savingsRows = computed(() =>
  monthlyLedger.value.savings.filter((row) => row.period === lastPeriod.value),
);

const sum = <T,>(rows: T[], amount: (row: T) => number) =>
  rows.reduce((total, row) => total + amount(row), 0);

const savings = computed(() => {
  const principal = sum(savingsRows.value, (row) => row.principalClosing);
  const mandatory = sum(savingsRows.value, (row) => row.mandatoryClosing);
  const voluntary = sum(savingsRows.value, (row) => row.voluntaryClosing);

  return {
    principal,
    mandatory,
    voluntary,
    total: principal + mandatory + voluntary,
  };
});

const groupOf = (row: LoanMonth) => loanById.value.get(row.loanId)?.loanGroup;

const loanSummary = computed(() => {
  const rows = loanRows.value;

  const balance = (group: LoanGroup) =>
    sum(
      rows.filter((row) => groupOf(row) === group),
      (row) => row.closingBalance,
    );

  // Tunggakan is what is still owed: a loan paid ahead (lebih bayar, e.g. paid
  // off early through a suplesi) shows a negative arrears in the sheet, which
  // must not lower the total of the others.
  const arrearsPrincipal = sum(rows, (row) =>
    Math.max(row.arrearsPrincipal, 0),
  );

  const arrearsInterest = sum(rows, (row) => Math.max(row.arrearsInterest, 0));

  return {
    total: sum(rows, (row) => row.closingBalance),
    member: balance(LoanGroup.Member),
    nonMember: balance(LoanGroup.NonMember),
    active: rows.filter((row) => row.closingBalance > 0).length,
    inArrears: rows.filter(
      (row) =>
        Math.max(row.arrearsPrincipal, 0) + Math.max(row.arrearsInterest, 0) >
        0,
    ).length,
    arrearsPrincipal,
    arrearsInterest,
    arrearsTotal: arrearsPrincipal + arrearsInterest,
  };
});

// Bunga + provisi received and new loans disbursed, per month of the year.
const months = computed(() =>
  periods.value.map((period) => {
    const rows = monthlyLedger.value.loans.filter(
      (row) => row.period === period,
    );

    const interest = sum(rows, (row) => row.interestPaid);
    const provision = sum(rows, (row) => row.provision);

    return {
      period,
      interest,
      provision,
      income: interest + provision,
      disbursed: sum(rows, (row) => row.disbursed),
    };
  }),
);

type MonthKey = "interest" | "provision" | "income" | "disbursed";

const totalOf = (key: MonthKey) => sum(months.value, (month) => month[key]);

const incomeTotal = computed(() => totalOf("income"));

const monthKeys: readonly MonthKey[] = [
  "interest",
  "provision",
  "income",
  "disbursed",
];

const monthTotals = computed(
  () =>
    Object.fromEntries(monthKeys.map((key) => [key, totalOf(key)])) as Record<
      MonthKey,
      number
    >,
);

const topArrears = computed(() =>
  loanRows.value
    .map((row) => {
      const loan = loanById.value.get(row.loanId);

      return {
        loanId: row.loanId,
        name: loan?.memberName ?? row.loanId,
        group: loan?.loanGroup ?? "",
        balance: row.closingBalance,
        principal: row.arrearsPrincipal,
        interest: row.arrearsInterest,
        total: row.arrearsPrincipal + row.arrearsInterest,
      };
    })
    .filter((row) => row.total > 0)
    .sort((a, b) => b.total - a.total)
    .slice(0, 10),
);

const savingsMembers = computed(
  () =>
    savingsRows.value.filter(
      (row) =>
        memberById.value.has(row.memberId) &&
        row.principalClosing + row.mandatoryClosing + row.voluntaryClosing > 0,
    ).length,
);

const share = (part: number, whole: number) =>
  whole
    ? `${((part / whole) * 100).toLocaleString("id-ID", { maximumFractionDigits: 1 })}%`
    : "-";
</script>

<template>
  <div class="page-stack dashboard">
    <PageHeader title="Dashboard" :refresh="refresh">
      <template #title-meta>
        <span v-if="lastPeriod" class="page-heading__meta">
          Posisi {{ formatPeriod(lastPeriod) }}
        </span>
      </template>
    </PageHeader>

    <section
      v-if="monthlyLedgerLoading && !periods.length"
      class="panel dashboard__empty"
    >
      Memuat data…
    </section>
    <section v-else-if="!periods.length" class="panel dashboard__empty">
      <strong>Belum ada data untuk {{ selectedYear }}</strong>
    </section>

    <template v-else>
      <div class="dashboard__tiles">
        <article class="panel dashboard__tile">
          <span>Kas</span>
          <strong>{{ formatCurrency(cashAndBank.cash) }}</strong>
          <small>
            Kas buku tabungan
            {{ formatCurrency(cashAndBank.bank) }}
          </small>
        </article>
        <article class="panel dashboard__tile">
          <span>Total simpanan</span>
          <strong>{{ formatCurrency(savings.total) }}</strong>
          <small>{{ savingsMembers }} anggota penyimpan</small>
        </article>
        <article class="panel dashboard__tile">
          <span>Saldo pinjaman</span>
          <strong>{{ formatCurrency(loanSummary.total) }}</strong>
          <small>{{ loanSummary.active }} pinjaman berjalan</small>
        </article>
        <article class="panel dashboard__tile dashboard__tile--warning">
          <span>Tunggakan</span>
          <strong>{{ formatCurrency(loanSummary.arrearsTotal) }}</strong>
          <small>{{ loanSummary.inArrears }} pinjaman menunggak</small>
        </article>
        <article class="panel dashboard__tile">
          <span>Pendapatan {{ selectedYear }}</span>
          <strong>{{ formatCurrency(incomeTotal) }}</strong>
          <small>Bunga dan provisi pinjaman</small>
        </article>
        <article class="panel dashboard__tile">
          <span>Anggota</span>
          <strong>{{ members.length.toLocaleString("id-ID") }}</strong>
          <small>Terdaftar di aplikasi</small>
        </article>
      </div>

      <section class="panel dashboard__card">
        <h2>Pendapatan dan pinjaman baru per bulan {{ selectedYear }}</h2>
        <div class="data-table-wrap">
          <table class="dashboard__table">
            <thead>
              <tr>
                <th>Bulan</th>
                <th class="num-cell">Pendapatan bunga</th>
                <th class="num-cell">Pendapatan provisi</th>
                <th class="num-cell">Jumlah bunga + provisi</th>
                <th class="num-cell">Pinjaman baru (realisasi)</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="month in months" :key="month.period">
                <td>{{ formatPeriod(month.period) }}</td>
                <td v-for="key in monthKeys" :key="key" class="num-cell">
                  <strong v-if="key === 'income'">{{
                    formatSheetNumber(month[key])
                  }}</strong>
                  <template v-else>{{
                    formatSheetNumber(month[key])
                  }}</template>
                </td>
              </tr>
            </tbody>
            <tfoot>
              <tr>
                <td>Jumlah</td>
                <td v-for="key in monthKeys" :key="key" class="num-cell">
                  {{ formatSheetNumber(monthTotals[key]) }}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
      </section>

      <section class="panel dashboard__card">
        <h2>10 tunggakan terbesar</h2>
        <p v-if="!topArrears.length" class="dashboard__card-meta">
          Tidak ada tunggakan pada {{ formatPeriod(lastPeriod) }}.
        </p>
        <div v-else class="data-table-wrap">
          <table class="dashboard__table">
            <thead>
              <tr>
                <th>No</th>
                <th>Nama</th>
                <th>Kelompok</th>
                <th class="num-cell">Saldo pinjaman</th>
                <th class="num-cell">Tunggakan pokok</th>
                <th class="num-cell">Tunggakan bunga</th>
                <th class="num-cell">Jumlah</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, index) in topArrears" :key="row.loanId">
                <td>{{ index + 1 }}</td>
                <td>
                  <strong>{{ row.name }}</strong>
                </td>
                <td>{{ row.group }}</td>
                <td class="num-cell">{{ formatSheetNumber(row.balance) }}</td>
                <td class="num-cell">{{ formatSheetNumber(row.principal) }}</td>
                <td class="num-cell">{{ formatSheetNumber(row.interest) }}</td>
                <td class="num-cell">
                  <strong>{{ formatSheetNumber(row.total) }}</strong>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.dashboard {
  gap: 12px;
  padding-bottom: 16px;
}
.dashboard__empty {
  padding: 32px;
  color: var(--muted);
  text-align: center;
}
.dashboard__tiles {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: 10px;
}
.dashboard__tile {
  display: grid;
  gap: 5px;
  padding: 15px 17px;
}
.dashboard__tile span {
  color: var(--muted);
  font-size: 10.5px;
  font-weight: 600;
}
.dashboard__tile strong {
  font-size: 19px;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}
.dashboard__tile small {
  color: var(--soft);
  font-size: 10px;
}
.dashboard__tile--warning {
  border-left: 3px solid var(--amber);
}
.dashboard__card {
  padding: 15px 17px;
  min-width: 0;
}
.dashboard__card h2 {
  margin: 0 0 10px;
  font-size: 13px;
}
.dashboard__card-meta {
  margin: -4px 0 12px;
  color: var(--muted);
  font-size: 11px;
}
.dashboard__list {
  margin: 0;
  display: grid;
}
.dashboard__list > div {
  display: grid;
  grid-template-columns: 1fr auto 52px;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}
.dashboard__list > div:last-child {
  border-bottom: 0;
}
.dashboard__list dt {
  color: var(--muted);
}
.dashboard__list dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.dashboard__list small {
  color: var(--soft);
  font-size: 10.5px;
  text-align: right;
}
.dashboard__list-total {
  font-weight: 700;
}
.dashboard__list-total dt {
  color: var(--ink);
}
.dashboard__table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.dashboard__table th {
  padding: 7px 8px;
  border-bottom: 1px solid var(--line-strong);
  color: var(--muted);
  font-size: 10.5px;
  font-weight: 600;
  text-align: left;
}
.dashboard__table th.num-cell {
  text-align: right;
}
.dashboard__table tfoot td {
  border-top: 2px solid var(--line-strong);
  color: var(--ink);
  font-weight: 700;
}
.dashboard__table td {
  padding: 7px 8px;
  border-bottom: 1px solid var(--line);
}
</style>
