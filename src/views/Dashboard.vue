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
  const arrearsPrincipal = sum(rows, (row) => row.arrearsPrincipal);
  const arrearsInterest = sum(rows, (row) => row.arrearsInterest);
  return {
    total: sum(rows, (row) => row.closingBalance),
    member: balance(LoanGroup.Member),
    nonMember: balance(LoanGroup.NonMember),
    active: rows.filter((row) => row.closingBalance > 0).length,
    inArrears: rows.filter(
      (row) => row.arrearsPrincipal + row.arrearsInterest > 0,
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
const incomeTotal = computed(() => sum(months.value, (month) => month.income));
const disbursedTotal = computed(() =>
  sum(months.value, (month) => month.disbursed),
);
const maxOf = (key: "income" | "disbursed") =>
  Math.max(1, ...months.value.map((month) => month[key]));
const shortMonth = (period: string) => formatPeriod(period).slice(0, 3);
const barCharts = computed(() => [
  {
    key: "income" as const,
    title: "Pendapatan bunga + provisi per bulan",
    total: incomeTotal.value,
    max: maxOf("income"),
  },
  {
    key: "disbursed" as const,
    title: "Pinjaman baru (realisasi) per bulan",
    total: disbursedTotal.value,
    max: maxOf("disbursed"),
  },
]);

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

      <div class="dashboard__grid">
        <section class="panel dashboard__card">
          <h2>Komposisi simpanan</h2>
          <dl class="dashboard__list">
            <div>
              <dt>Simpanan pokok</dt>
              <dd>{{ formatSheetNumber(savings.principal) }}</dd>
              <small>{{ share(savings.principal, savings.total) }}</small>
            </div>
            <div>
              <dt>Simpanan wajib</dt>
              <dd>{{ formatSheetNumber(savings.mandatory) }}</dd>
              <small>{{ share(savings.mandatory, savings.total) }}</small>
            </div>
            <div>
              <dt>Simpanan manasuka</dt>
              <dd>{{ formatSheetNumber(savings.voluntary) }}</dd>
              <small>{{ share(savings.voluntary, savings.total) }}</small>
            </div>
            <div class="dashboard__list-total">
              <dt>Jumlah</dt>
              <dd>{{ formatSheetNumber(savings.total) }}</dd>
              <small></small>
            </div>
          </dl>
        </section>
        <section class="panel dashboard__card">
          <h2>Pinjaman</h2>
          <dl class="dashboard__list">
            <div>
              <dt>Anggota PP BRI</dt>
              <dd>{{ formatSheetNumber(loanSummary.member) }}</dd>
              <small>{{ share(loanSummary.member, loanSummary.total) }}</small>
            </div>
            <div>
              <dt>Non anggota PP BRI</dt>
              <dd>{{ formatSheetNumber(loanSummary.nonMember) }}</dd>
              <small>{{
                share(loanSummary.nonMember, loanSummary.total)
              }}</small>
            </div>
            <div>
              <dt>Tunggakan pokok</dt>
              <dd>{{ formatSheetNumber(loanSummary.arrearsPrincipal) }}</dd>
              <small></small>
            </div>
            <div>
              <dt>Tunggakan bunga</dt>
              <dd>{{ formatSheetNumber(loanSummary.arrearsInterest) }}</dd>
              <small></small>
            </div>
          </dl>
        </section>
      </div>

      <div class="dashboard__grid">
        <section
          v-for="chart in barCharts"
          :key="chart.key"
          class="panel dashboard__card"
        >
          <h2>{{ chart.title }}</h2>
          <p class="dashboard__card-meta">
            Jumlah {{ selectedYear }}:
            <strong>{{ formatSheetNumber(chart.total) }}</strong>
          </p>
          <div class="dashboard__bars" role="table" :aria-label="chart.title">
            <div
              v-for="month in months"
              :key="month.period"
              class="dashboard__bar"
              role="row"
              tabindex="0"
              :data-value="`${formatPeriod(month.period)}: ${formatSheetNumber(month[chart.key])}`"
            >
              <span class="dashboard__bar-track" role="cell">
                <span
                  class="dashboard__bar-fill"
                  :style="{
                    height: `${(month[chart.key] / chart.max) * 100}%`,
                  }"
                ></span>
              </span>
              <span class="dashboard__bar-label" role="cell">
                {{ shortMonth(month.period) }}
              </span>
            </div>
          </div>
        </section>
      </div>

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
.dashboard__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 12px;
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
.dashboard__bars {
  height: 170px;
  display: grid;
  grid-auto-columns: 1fr;
  grid-auto-flow: column;
  gap: 6px;
  padding-top: 26px;
}
.dashboard__bar {
  position: relative;
  min-width: 0;
  display: grid;
  grid-template-rows: 1fr auto;
  gap: 5px;
  outline: 0;
}
.dashboard__bar-track {
  display: flex;
  align-items: flex-end;
  border-bottom: 1px solid var(--line-strong);
}
.dashboard__bar-fill {
  width: 100%;
  max-width: 28px;
  margin: 0 auto;
  background: var(--green);
  border-radius: 4px 4px 0 0 !important;
}
.dashboard__bar:hover .dashboard__bar-fill,
.dashboard__bar:focus-visible .dashboard__bar-fill {
  background: var(--green-dark);
}
.dashboard__bar-label {
  color: var(--soft);
  font-size: 10px;
  text-align: center;
}
.dashboard__bar:hover::after,
.dashboard__bar:focus-visible::after {
  content: attr(data-value);
  position: absolute;
  bottom: calc(100% - 20px);
  left: 50%;
  z-index: 2;
  transform: translateX(-50%);
  padding: 4px 7px;
  border: 1px solid var(--line-strong);
  color: var(--ink);
  background: #fff;
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  pointer-events: none;
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
.dashboard__table td {
  padding: 7px 8px;
  border-bottom: 1px solid var(--line);
}
</style>
