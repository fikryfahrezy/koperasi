<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { CalendarX2, Plus, Search, Undo2 } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import UiModal from "../components/UiModal.vue";
import CashEntryDialog from "../components/CashEntryDialog.vue";
import {
  Channel,
  formatCurrency,
  formatSheetNumber,
  type CashBookRow,
  TransactionDirection,
  TransactionStatus,
  useKoperasiStore,
  useCashBook,
} from "../store/koperasi";

const pageState = useWorkspacePageState("/cash-ledger");

const { selectedYear, loadFinancialParameters, reverseTransaction, refresh } =
  useKoperasiStore();

// --- Cash ledger (following the daily cash ledger spreadsheet) ----------------------------
const { cashBook, loadingBook } = useCashBook(Channel.Cash);
const selected = pageState.field<CashBookRow | null>("selected", () => null);

onMounted(() => {
  void loadFinancialParameters();
});

const rows = computed(() => cashBook.value?.rows ?? []);
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
const filteredRows = computed(() => {
  const search = debouncedQuery.value.trim().toLowerCase();
  return rows.value.filter(
    (row) =>
      row.description.toLowerCase().includes(search) ||
      (search !== "" && row.memberNumber.toLowerCase().includes(search)),
  );
});
// As in the spreadsheet, the date appears only on the first row of each day.
const showDate = (index: number) =>
  index === 0 ||
  filteredRows.value[index - 1].date !== filteredRows.value[index].date;
// KAS BUKU TABUNGAN (the bank savings book), as on the sheet: Keluar = Tambahan
// Kas dari Tabungan, Masuk = the opening balance and Transfer ke Tabungan. The last column is
// cash + bank on the opening row. No running bank balance: the bank also receives
// money that never passes through Buku Kas.
const savingsBookOpening = computed(
  () => cashBook.value?.savingsBookOpeningBalance ?? null,
);
const savingsBookTotals = computed(() =>
  rows.value
    .filter((row) => !isExcluded(row))
    .reduce(
      (sum, row) => ({
        out: sum.out + row.savingsBookOut,
        in: sum.in + row.savingsBookIn,
      }),
      { out: 0, in: 0 },
    ),
);
const isExcluded = (item: CashBookRow) =>
  item.status === TransactionStatus.Reversed || item.isReversal;

const entryDialog = ref<InstanceType<typeof CashEntryDialog> | null>(null);

async function reverseSelected() {
  if (!selected.value) return;
  const submittedSelection = selected.forTab(pageState.tabId.value);
  const reversed = await reverseTransaction(submittedSelection.value!.id);
  if (reversed) submittedSelection.value = null;
}
async function refreshPage() {
  await refresh();
  await loadFinancialParameters();
}
</script>

<template>
  <div class="page-stack page-stack--sheet">
    <PageHeader title="Buku kas harian" :refresh="refreshPage">
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
      </template>
      <template #actions>
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
      <div v-else class="data-table-wrap sheet-wrap">
        <table v-table-navigation class="sheet-table cash-book-table">
          <thead>
            <tr>
              <th rowspan="2">Tanggal</th>
              <th rowspan="2" class="cash-book-table__desc">Uraian</th>
              <th colspan="2" class="sheet-group">Mutasi</th>
              <th rowspan="2" class="sheet-total">Jumlah Saldo Kas</th>
              <th colspan="2" class="sheet-group">KAS BUKU TABUNGAN</th>
              <th rowspan="2" class="sheet-total">Kas + Tabungan</th>
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
                Saldo Kas Fisik Neraca Posisi 31 Desember {{ selectedYear - 1 }}
              </td>
              <td></td>
              <td></td>
              <td class="num-cell sheet-total">
                <strong>{{
                  formatSheetNumber(cashBook?.openingBalance ?? 0)
                }}</strong>
              </td>
              <td></td>
              <td class="num-cell">
                <strong v-if="savingsBookOpening !== null">{{
                  formatSheetNumber(savingsBookOpening)
                }}</strong>
              </td>
              <td class="num-cell sheet-total">
                <strong v-if="savingsBookOpening !== null">{{
                  formatSheetNumber(
                    (cashBook?.openingBalance ?? 0) + savingsBookOpening,
                  )
                }}</strong>
              </td>
            </tr>
            <tr
              v-for="(item, index) in filteredRows"
              :key="item.id"
              class="cash-book-table__row"
              :class="{ 'is-excluded': isExcluded(item) }"
              @click="selected = item"
            >
              <td>{{ showDate(index) ? item.date : "" }}</td>
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
              <td class="sheet-total"></td>
            </tr>
            <tr
              v-if="
                !loadingBook && debouncedQuery.trim() && !filteredRows.length
              "
            >
              <td colspan="8">
                Tidak ada transaksi yang cocok dengan pencarian.
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td colspan="2">
                {{ debouncedQuery.trim() ? "JUMLAH TAHUNAN" : "JUMLAH" }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashBook?.totalOut ?? 0) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(cashBook?.totalIn ?? 0) }}
              </td>
              <td class="num-cell sheet-total">
                {{ formatSheetNumber(cashBook?.closingBalance ?? 0) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(savingsBookTotals.out) }}
              </td>
              <td class="num-cell">
                {{ formatSheetNumber(savingsBookTotals.in) }}
              </td>
              <td class="sheet-total"></td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>

    <CashEntryDialog ref="entryDialog" :page-state="pageState" />

    <UiModal
      :open="Boolean(selected) && pageState.active.value"
      title="Detail transaksi"
      @close="selected = null"
      ><div v-if="selected" class="transaction-detail">
        <div class="transaction-detail__head">
          <div>
            <strong>{{ selected.description }}</strong
            ><span>{{ selected.date }}</span>
          </div>
          <strong>{{ formatCurrency(selected.amount) }}</strong>
        </div>
        <div class="component-list">
          <div v-for="(component, index) in selected.components" :key="index">
            <span>{{ component.label }}</span
            ><strong>{{ formatCurrency(component.amount) }}</strong>
          </div>
        </div>
        <div v-if="isExcluded(selected)" class="audit-note">
          {{
            selected.isReversal
              ? "Baris ini adalah reversal."
              : "Transaksi ini sudah dibalik."
          }}
        </div>
        <div class="modal-actions">
          <button
            class="button button--danger"
            :disabled="isExcluded(selected)"
            @click="reverseSelected"
          >
            <Undo2 :size="17" /> Buat reversal</button
          ><button class="button button--primary" @click="selected = null">
            Selesai
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
.cash-book-table__row {
  cursor: pointer;
}
.cash-book-table tr.is-excluded td {
  color: var(--soft);
  text-decoration: line-through;
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
.transaction-detail .audit-note {
  margin-top: 16px;
}
.audit-note {
  color: var(--muted);
  background: #f3f6f4;
}
</style>
