<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { CalendarX2, Plus, Search, Trash2 } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import PageTitleTabs from "../components/PageTitleTabs.vue";
import UiModal from "../components/UiModal.vue";
import CashEntryDialog from "../components/CashEntryDialog.vue";
import ExportButton from "../components/ExportButton.vue";
import UiSelect from "../components/UiSelect.vue";

import {
  deletedRowsOf,
  isExcluded,
  matchesSearch,
  plainWords,
  savingsBookTotalsOf,
  cashPlusSavingsOf,
} from "../cash-book-rows";

import {
  Channel,
  type TransactionGroup,
  formatCurrency,
  formatDate,
  formatPeriod,
  formatLoanLabel,
  formatSheetNumber,
  type CashBookRow,
  TransactionDirection,
  useKoperasiStore,
  useCashBook,
} from "../store/koperasi";

import { cashBookLetters } from "../sheet-columns";

const pageState = useWorkspacePageState("/cash-ledger");

const {
  selectedYear,
  loans,
  loadFinancialParameters,
  reverseTransaction,
  getTransactionGroup,
  refresh,
} = useKoperasiStore();

// --- Cash ledger (following the daily cash ledger spreadsheet) ----------------------------
const { cashBook, loadingBook } = useCashBook(Channel.Cash);

const pendingDelete = pageState.field<CashBookRow | null>(
  "pendingDelete",
  () => null,
);

// "Detail": one row can hold several items, e.g. a deposit paying two loans
// plus wajib, or a suplesi payoff.
const pendingDetail = pageState.field<CashBookRow | null>(
  "pendingDetail",
  () => null,
);

const detailItems = computed(() =>
  (pendingDetail.value?.components ?? []).map((item) => {
    const loan = item.loanId
      ? loans.find((candidate) => candidate.id === item.loanId)
      : undefined;

    return { ...item, loanLabel: loan ? formatLoanLabel(loan) : "" };
  }),
);

// Hapus of an imported row also cancels the movements paired with it: list them.
const deleteGroup = ref<TransactionGroup | null>(null);

watch(pendingDelete, async (row) => {
  deleteGroup.value = null;
  if (!row?.groupId) return;
  const group = await getTransactionGroup(row.id);
  if (pendingDelete.value?.id === row.id) deleteGroup.value = group;
});

const deleteAlso = computed(() =>
  (deleteGroup.value?.transactions ?? [])
    .filter((transaction) => transaction.id !== pendingDelete.value?.id)
    .map((transaction) => ({
      ...transaction,
      ...plainWords(transaction, loans),
    })),
);

onMounted(() => {
  void loadFinancialParameters();
});

const rows = computed(() => cashBook.value?.rows ?? []);
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);

// Month range ("YYYY-MM"): an empty start means January, an empty end the
// latest row. Saldo awal and the totals follow the range.
const startMonth = pageState.field("startMonth", () => "");
const endMonth = pageState.field("endMonth", () => "");
watch(selectedYear, () => {
  startMonth.value = "";
  endMonth.value = "";
});

const monthOf = (row: CashBookRow) => row.businessDate.slice(0, 7);
const months = computed(() => [...new Set(rows.value.map(monthOf))].sort());
const startOptions = computed(() => [
  { value: "", label: "Januari" },
  ...months.value.map((month) => ({
    value: month,
    label: formatPeriod(month),
    disabled: Boolean(endMonth.value) && month > endMonth.value,
  })),
]);
const endOptions = computed(() => [
  { value: "", label: "Terbaru" },
  ...months.value.map((month) => ({
    value: month,
    label: formatPeriod(month),
    disabled: Boolean(startMonth.value) && month < startMonth.value,
  })),
]);

const hasRange = computed(() => Boolean(startMonth.value || endMonth.value));
const inRange = (row: CashBookRow) =>
  (!startMonth.value || monthOf(row) >= startMonth.value) &&
  (!endMonth.value || monthOf(row) <= endMonth.value);
const rangeLabel = computed(() => {
  const first = startMonth.value || months.value[0];
  const last = endMonth.value || months.value[months.value.length - 1];
  if (!first || !last) return "";
  return first === last
    ? formatPeriod(first)
    : `${formatPeriod(first)} – ${formatPeriod(last)}`;
});

const postedRows = computed(() => rows.value.filter((row) => !isExcluded(row)));
const rowsBefore = computed(() =>
  startMonth.value
    ? postedRows.value.filter((row) => monthOf(row) < startMonth.value)
    : [],
);
const rangeRows = computed(() => postedRows.value.filter(inRange));

const filteredRows = computed(() =>
  rangeRows.value.filter((row) => matchesSearch(row, debouncedQuery.value)),
);

// "Riwayat hapus" swaps the sheet for every deleted row of the year.
const showDeleted = pageState.field("showDeleted", () => false);

const ledgerTabs = computed(() => [
  { value: false, label: "Buku kas harian" },
  { value: true, label: "Riwayat hapus", count: deletedRows.value.length },
]);

// The export holds the rows the search and month range leave.
const exportFilter = computed(() =>
  debouncedQuery.value.trim() || hasRange.value
    ? { ids: filteredRows.value.map((row) => row.id) }
    : {},
);

const deletedRows = computed(() => deletedRowsOf(rows.value));

const filteredDeletedRows = computed(() =>
  deletedRows.value.filter((row) => matchesSearch(row, debouncedQuery.value)),
);

// As in the spreadsheet, the date appears only on the first row of each day.
const showDate = (index: number) =>
  index === 0 ||
  filteredRows.value[index - 1].businessDate !==
    filteredRows.value[index].businessDate;

// KAS BUKU TABUNGAN (the bank savings book), as on the sheet: Keluar = Tambahan
// Kas dari Tabungan, Masuk = the opening balance and Transfer ke Tabungan. The last column is
// cash + bank after each row; the bank part is an estimate, since the bank also
// receives money that never passes through Buku Kas.
const savingsBookOpening = computed(
  () => cashBook.value?.savingsBookOpeningBalance ?? null,
);

const cashPlusSavings = computed(() =>
  cashPlusSavingsOf(rows.value, savingsBookOpening.value ?? 0),
);

// Saldo awal of the range: the balances after the last row before it.
const openingCash = computed(
  () =>
    rowsBefore.value[rowsBefore.value.length - 1]?.balance ??
    cashBook.value?.openingBalance ??
    0,
);
const openingBank = computed(() => {
  if (savingsBookOpening.value === null) return null;
  const moved = savingsBookTotalsOf(rowsBefore.value);
  return savingsBookOpening.value + moved.in - moved.out;
});

// JUMLAH of the range (the whole year when no month is chosen).
const cashTotals = computed(() => {
  if (!hasRange.value)
    return {
      out: cashBook.value?.totalOut ?? 0,
      in: cashBook.value?.totalIn ?? 0,
      closing: cashBook.value?.closingBalance ?? 0,
    };
  const cashRows = rangeRows.value.filter(
    (row) => row.channel === Channel.Cash,
  );
  const amountOf = (direction: TransactionDirection) =>
    cashRows
      .filter((row) => row.direction === direction)
      .reduce((sum, row) => sum + row.amount, 0);
  return {
    out: amountOf(TransactionDirection.Out),
    in: amountOf(TransactionDirection.In),
    closing:
      rangeRows.value[rangeRows.value.length - 1]?.balance ?? openingCash.value,
  };
});
const savingsBookTotals = computed(() => savingsBookTotalsOf(rangeRows.value));

const entryDialog = ref<InstanceType<typeof CashEntryDialog> | null>(null);

async function deletePending() {
  if (!pendingDelete.value) return;
  const submitted = pendingDelete.forTab(pageState.tabId.value);
  const reversed = await reverseTransaction(submitted.value!);
  if (reversed) submitted.value = null;
}

async function refreshPage() {
  await refresh();
  await loadFinancialParameters();
}
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader
      :title="showDeleted ? 'Riwayat hapus' : 'Buku kas harian'"
      :refresh="refreshPage"
    >
      <template #title>
        <PageTitleTabs
          v-model="showDeleted"
          label="Buku kas"
          :tabs="ledgerTabs"
        />
      </template>
      <template #before-actions>
        <label class="search-field">
          <Search :size="18" />
          <input
            v-model="query"
            type="search"
            aria-label="Cari uraian, nama atau ID anggota"
            placeholder="Cari uraian atau ID anggota..."
          />
        </label>
        <template v-if="!showDeleted && months.length">
          <UiSelect
            v-model="startMonth"
            :options="startOptions"
            label="Dari"
            aria-label="Dari bulan"
            variant="toolbar"
          />
          <UiSelect
            v-model="endMonth"
            :options="endOptions"
            label="Sampai"
            aria-label="Sampai bulan"
            variant="toolbar"
          />
        </template>
      </template>
      <template #actions>
        <ExportButton
          v-if="!showDeleted"
          kind="cash-book"
          :filter="exportFilter"
        />
        <button
          class="button button--primary"
          type="button"
          @click="entryDialog?.open()"
        >
          <Plus :size="18" /> Catat transaksi
        </button>
      </template>
    </PageHeader>

    <section class="panel table-panel">
      <div
        v-if="!loadingBook && !rows.length && !cashBook?.openingBalance"
        class="year-empty-state year-empty-state--inline"
      >
        <CalendarX2 :size="36" />
        <strong>Belum ada transaksi kas untuk {{ selectedYear }}</strong>
        <p>Catat transaksi baru dengan tombol Catat transaksi.</p>
      </div>
      <div v-else-if="showDeleted" class="data-table-wrap sheet-wrap">
        <table
          v-table-navigation
          v-sticky-header
          class="sheet-table cash-book-table deleted-table"
        >
          <thead>
            <tr>
              <th>Tanggal</th>
              <th class="cash-book-table__desc">Uraian</th>
              <th>Dihapus</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="item in filteredDeletedRows"
              :key="item.id"
              class="cash-book-table__row"
            >
              <td>{{ formatDate(item.businessDate) }}</td>
              <td class="cash-book-table__desc">
                <span class="deleted-table__desc">{{ item.description }}</span>
                <small class="deleted-table__meta">{{ item.summary }}</small>
              </td>
              <td>{{ item.deletedAt }}</td>
            </tr>
            <tr v-if="!filteredDeletedRows.length">
              <td colspan="3">
                {{
                  debouncedQuery.trim()
                    ? "Tidak ada transaksi terhapus yang cocok dengan pencarian."
                    : `Belum ada transaksi yang dihapus di ${selectedYear}.`
                }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else class="data-table-wrap sheet-wrap">
        <table
          v-table-navigation
          v-sticky-header
          class="sheet-table cash-book-table"
        >
          <thead>
            <tr class="sheet-letters">
              <th v-for="letter in cashBookLetters" :key="letter">
                {{ letter }}
              </th>
              <th></th>
            </tr>
            <tr>
              <th rowspan="2">Tanggal</th>
              <th rowspan="2" class="cash-book-table__desc">Uraian</th>
              <th colspan="2" class="sheet-group">Mutasi</th>
              <th rowspan="2" class="sheet-total">Jumlah Saldo Kas</th>
              <th colspan="2" class="sheet-group">KAS BUKU TABUNGAN</th>
              <th rowspan="2" class="sheet-total">Kas + Tabungan</th>
              <th
                rowspan="2"
                class="cash-book-table__actions"
                aria-label="Aksi"
              ></th>
            </tr>
            <tr>
              <th>Debet</th>
              <th>Kredit</th>
              <th>Keluar</th>
              <th>Masuk</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td></td>
              <td class="cash-book-table__desc">
                <template v-if="startMonth"
                  >Saldo awal {{ formatPeriod(startMonth) }}</template
                >
                <template v-else
                  >Saldo Kas Fisik Neraca Posisi 31 Desember
                  {{ selectedYear - 1 }}</template
                >
              </td>
              <td></td>
              <td></td>
              <td class="num-cell sheet-total">
                <strong>{{ formatSheetNumber(openingCash) }}</strong>
              </td>
              <td></td>
              <td class="num-cell">
                <strong v-if="openingBank !== null">{{
                  formatSheetNumber(openingBank)
                }}</strong>
              </td>
              <td class="num-cell sheet-total">
                <strong v-if="openingBank !== null">{{
                  formatSheetNumber(openingCash + openingBank)
                }}</strong>
              </td>
              <td class="cash-book-table__actions"></td>
            </tr>
            <tr
              v-for="(item, index) in filteredRows"
              :key="item.id"
              class="cash-book-table__row"
            >
              <td>
                {{ showDate(index) ? formatDate(item.businessDate) : "" }}
              </td>
              <td class="cash-book-table__desc">{{ item.description }}</td>
              <td class="num-cell">
                {{
                  item.channel === Channel.Cash &&
                  item.direction === TransactionDirection.Out
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td class="num-cell">
                {{
                  item.channel === Channel.Cash &&
                  item.direction === TransactionDirection.In
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td class="num-cell sheet-total">
                {{ formatSheetNumber(item.balance) }}
              </td>
              <td class="num-cell">
                {{
                  item.savingsBookOut
                    ? formatSheetNumber(item.savingsBookOut)
                    : ""
                }}
              </td>
              <td class="num-cell">
                {{
                  item.savingsBookIn
                    ? formatSheetNumber(item.savingsBookIn)
                    : ""
                }}
              </td>
              <td class="num-cell sheet-total">
                {{
                  cashPlusSavings.has(item.id)
                    ? formatSheetNumber(cashPlusSavings.get(item.id)!)
                    : ""
                }}
              </td>
              <td class="cash-book-table__actions">
                <button
                  class="cash-book-table__detail"
                  type="button"
                  :aria-label="`Detail ${item.description}`"
                  @click.stop="pendingDetail = item"
                >
                  Detail
                </button>
                <button
                  class="cash-book-table__delete"
                  type="button"
                  :aria-label="`Hapus ${item.description}`"
                  @click.stop="pendingDelete = item"
                >
                  Hapus
                </button>
              </td>
            </tr>
            <tr
              v-if="
                !loadingBook && debouncedQuery.trim() && !filteredRows.length
              "
            >
              <td colspan="9">
                Tidak ada transaksi yang cocok dengan pencarian.
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td colspan="2">
                {{
                  hasRange
                    ? `JUMLAH ${rangeLabel}`
                    : debouncedQuery.trim()
                      ? "JUMLAH TAHUNAN"
                      : "JUMLAH"
                }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashTotals.out) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashTotals.in) }}
              </td>
              <td class="num-cell sheet-total">
                {{ formatSheetNumber(cashTotals.closing) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(savingsBookTotals.out) }}
              </td>
              <td class="num-cell">
                {{
                  formatSheetNumber((openingBank ?? 0) + savingsBookTotals.in)
                }}
              </td>
              <td class="num-cell sheet-total">
                {{
                  formatSheetNumber(
                    cashTotals.closing +
                      (openingBank ?? 0) +
                      savingsBookTotals.in -
                      savingsBookTotals.out,
                  )
                }}
              </td>
              <td class="cash-book-table__actions"></td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>

    <CashEntryDialog ref="entryDialog" :page-state="pageState" />

    <UiModal
      :open="Boolean(pendingDetail) && pageState.active.value"
      title="Detail transaksi"
      @close="pendingDetail = null"
      ><div v-if="pendingDetail" class="transaction-detail">
        <div class="transaction-detail__head">
          <div>
            <strong>{{ pendingDetail.description }}</strong
            ><span
              >{{ formatDate(pendingDetail.businessDate) }}
              <template v-if="pendingDetail.memberNumber">
                · {{ pendingDetail.memberNumber }}
                {{ pendingDetail.memberName }}</template
              ></span
            >
          </div>
          <div class="transaction-detail__total">
            <strong>{{ formatCurrency(pendingDetail.amount) }}</strong
            ><span>{{
              pendingDetail.direction === TransactionDirection.In
                ? "Masuk"
                : "Keluar"
            }}</span>
          </div>
        </div>
        <ul class="transaction-detail__items">
          <li v-for="(item, index) in detailItems" :key="index">
            <div>
              <span>{{ item.label }}</span>
              <small v-if="item.loanLabel">{{ item.loanLabel }}</small>
            </div>
            <strong>{{ formatCurrency(item.amount) }}</strong>
          </li>
        </ul>
        <div class="modal-actions">
          <button class="button" @click="pendingDetail = null">Tutup</button>
        </div>
      </div>
    </UiModal>

    <UiModal
      :open="Boolean(pendingDelete) && pageState.active.value"
      title="Hapus transaksi?"
      @close="pendingDelete = null"
      ><div v-if="pendingDelete" class="transaction-detail">
        <div class="transaction-detail__head">
          <div>
            <strong>{{ pendingDelete.description }}</strong
            ><span>{{ formatDate(pendingDelete.businessDate) }}</span>
          </div>
          <strong>{{ formatCurrency(pendingDelete.amount) }}</strong>
        </div>
        <template v-if="deleteAlso.length">
          <p class="transaction-detail__also">
            Ikut terhapus: pembayaran pinjaman/simpanan yang tercatat dari baris
            ini ({{ deleteGroup?.label }}).
          </p>
          <ul class="transaction-detail__items">
            <li v-for="transaction in deleteAlso" :key="transaction.id">
              <div>
                <span>{{ transaction.title }}</span>
                <small v-if="transaction.detail">{{
                  transaction.detail
                }}</small>
                <small>{{ formatDate(transaction.businessDate) }}</small>
              </div>
              <strong>{{ formatCurrency(transaction.amount) }}</strong>
            </li>
          </ul>
        </template>
        <div class="modal-actions">
          <button class="button" @click="pendingDelete = null">Batal</button
          ><button class="button button--danger" @click="deletePending">
            <Trash2 :size="17" />
            Hapus
          </button>
        </div>
      </div>
    </UiModal>
  </div>
</template>

<style scoped>
.button--danger {
  color: var(--red);
  background: var(--red-soft);
}
.year-empty-state--inline {
  border: 0;
  border-radius: 0;
}
.cash-book-table .cash-book-table__desc {
  min-width: 280px;
  max-width: 420px;
  white-space: normal;
  text-align: left;
}
.cash-book-table .cash-book-table__actions {
  width: 1%;
  white-space: nowrap;
  padding: 2px 6px;
  text-align: center;
}
.cash-book-table__detail {
  padding: 2px 6px;
  border: 0;
  border-radius: 6px;
  color: var(--primary);
  background: transparent;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.cash-book-table__detail:hover {
  background: rgba(1, 82, 156, 0.06);
}
.cash-book-table__delete {
  padding: 2px 6px;
  border: 0;
  border-radius: 6px;
  color: var(--red);
  background: transparent;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}
.cash-book-table__delete:hover {
  background: var(--red-soft);
}
.transaction-detail__head {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--line);
}
.transaction-detail__head > div {
  display: grid;
  gap: 4px;
}
.transaction-detail__head small,
.transaction-detail__head span {
  color: var(--soft);
  font-size: 9px;
}
.transaction-detail__total {
  justify-items: end;
}
.transaction-detail__items {
  display: grid;
  margin: 0;
  padding: 0;
  list-style: none;
}
.transaction-detail__items li {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 20px;
  padding: 10px 0;
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}
.transaction-detail__items li > div {
  display: grid;
  gap: 2px;
}
.transaction-detail__also {
  margin: 12px 0 0;
  color: var(--muted);
  font-size: 11px;
}
.transaction-detail__items small {
  color: var(--soft);
  font-size: 10px;
}
.deleted-table__desc,
.deleted-table__meta {
  display: block;
}
.deleted-table__meta {
  margin-top: 2px;
  color: var(--soft);
  font-size: 10.5px;
}
</style>
