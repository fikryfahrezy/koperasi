<script setup lang="ts">
import { computed, ref } from "vue";
import { CalendarX2, Search, Wallet } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import SittingAllowanceDialog from "../components/SittingAllowanceDialog.vue";
import { registeredBy, shuRows, sumShuRows } from "../shu";

import {
  formatPeriod,
  formatRate,
  formatSheetNumber,
  matchesMember,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

// Daftar Penerimaan SHU of the book year, as DAFTAR SHU TAHUN BUKU in the RAT
// workbook: Insentif Simpanan of the Ratas Simpanan plus Insentif Pinjaman of
// the jasa pinjaman (only members who borrowed have one), plus uang duduk,
// rounded up to the thousand. The rates are set on Simpanan and Pinjaman.
const pageState = useWorkspacePageState("/shu");
const allowanceOpen = ref(false);

const { members, loans, selectedYear, refresh } = useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();
const { monthlyLedger: nextLedger } = useMonthlyLedger(1);

// Members registered in the year are listed even without savings or loans
// (they still get the uang duduk). Imported members have no join date, so one
// whose savings start only next year joined then and is left out.
const registeredMemberIds = computed(() => {
  const startsLater = new Set(
    nextLedger.value.savings.map((row) => row.memberId),
  );

  return members
    .filter(
      (member) =>
        registeredBy(member.joinedAt, selectedYear.value) &&
        !startsLater.has(member.id),
    )
    .map((member) => member.id);
});

const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);

const memberById = computed(
  () => new Map(members.map((member) => [member.id, member])),
);

const borrowerOf = computed(
  () => new Map(loans.map((loan) => [loan.id, loan.memberId])),
);

// As of the last month of the year (December once the year is over).
const lastPeriod = computed(() => {
  const periods = monthlyLedger.value.periods;
  return [...periods].sort()[periods.length - 1] ?? "";
});

const allRows = computed(() =>
  shuRows({
    periods: monthlyLedger.value.periods,
    period: lastPeriod.value,
    savings: monthlyLedger.value.savings,
    loans: monthlyLedger.value.loans,
    borrowerOf: borrowerOf.value,
    memberIds: registeredMemberIds.value,
    savingsRate: monthlyLedger.value.savingsIncentiveRate,
    loanRate: monthlyLedger.value.loanIncentiveRate,
    sittingAllowance: monthlyLedger.value.sittingAllowance,
  }).map((row, index) => ({
    ...row,
    no: index + 1,
    name: memberById.value.get(row.memberId)?.name ?? row.memberId,
    memberNumber: memberById.value.get(row.memberId)?.memberNumber ?? "",
  })),
);

const rows = computed(() =>
  allRows.value.filter((row) =>
    matchesMember(debouncedQuery.value, row.name, row.memberNumber),
  ),
);

const totals = computed(() => sumShuRows(rows.value));

const amountKeys = [
  "savingsAverage",
  "savingsShu",
  "loanIncome",
  "loanShu",
  "shu",
  "sittingAllowance",
  "total",
  "rounded",
] as const;

// Members without a loan show "-" for Jasa Pinjaman and its SHU.
const formatAmount = (value: number) => formatSheetNumber(Math.round(value));
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="SHU" :refresh="refresh">
      <template #title-meta>
        <span v-if="monthlyLedgerLoading" class="sheet-loading" role="status">
          Memuat SHU…
        </span>
        <span v-else-if="lastPeriod" class="shu-meta">
          Tahun buku {{ selectedYear }}, posisi {{ formatPeriod(lastPeriod) }}
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
      </template>
      <template #actions>
        <button
          v-if="monthlyLedger.periods.length"
          class="button button--secondary"
          type="button"
          @click="allowanceOpen = true"
        >
          <Wallet :size="16" /> Uang duduk {{ selectedYear }}
        </button>
      </template>
    </PageHeader>
    <section
      v-if="monthlyLedgerLoading"
      class="panel backend-state"
      aria-busy="true"
    >
      <strong>Memuat SHU…</strong>
    </section>
    <section
      v-else-if="!monthlyLedger.periods.length"
      class="panel year-empty-state"
    >
      <CalendarX2 :size="36" />
      <strong>Belum ada data untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain.</p>
    </section>
    <section v-else class="panel table-panel">
      <div class="data-table-wrap sheet-wrap">
        <table v-table-navigation v-sticky-header class="sheet-table">
          <thead>
            <tr>
              <th rowspan="3" class="sheet-sticky sheet-no">No</th>
              <th rowspan="3" class="sheet-sticky sheet-name">Nama anggota</th>
              <th colspan="2">Simpanan</th>
              <th colspan="2">Pinjaman</th>
              <th rowspan="3">Jumlah SHU</th>
              <th rowspan="3">Uang duduk + uang makan</th>
              <th rowspan="3">Total</th>
              <th rowspan="3" class="sheet-total">Pembulatan</th>
            </tr>
            <tr>
              <th rowspan="2">Ratas</th>
              <th>SHU</th>
              <th rowspan="2">Jasa pinjaman</th>
              <th>SHU</th>
            </tr>
            <tr>
              <th class="shu-rate">
                {{ formatRate(monthlyLedger.savingsIncentiveRate) }}
              </th>
              <th class="shu-rate">
                {{ formatRate(monthlyLedger.loanIncentiveRate) }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.memberId">
              <td class="sheet-sticky sheet-no">{{ row.no }}</td>
              <td class="sheet-sticky sheet-name">{{ row.name }}</td>
              <td
                v-for="key in amountKeys"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': key === 'rounded' }"
              >
                {{ formatAmount(row[key]) }}
              </td>
            </tr>
            <tr v-if="!rows.length">
              <td class="sheet-empty" :colspan="amountKeys.length + 2">
                Tidak ada anggota yang cocok.
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td class="sheet-sticky sheet-no"></td>
              <td class="sheet-sticky sheet-name">JUMLAH</td>
              <td
                v-for="key in amountKeys"
                :key="key"
                class="num-cell"
                :class="{ 'sheet-total': key === 'rounded' }"
              >
                {{ formatAmount(totals[key]) }}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
    <SittingAllowanceDialog v-model:open="allowanceOpen" />
  </div>
</template>

<style scoped>
.shu-meta {
  color: var(--soft);
  font-size: 10.5px;
}
.shu-rate {
  font-weight: 600;
  text-transform: none;
}
</style>
