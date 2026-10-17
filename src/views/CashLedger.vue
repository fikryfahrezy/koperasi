<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { CalendarX2, History, Plus, Search, Trash2 } from "lucide-vue-next";
import { useDebouncedRef } from "../composables/useDebouncedRef";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import PageHeader from "../components/PageHeader.vue";
import UiModal from "../components/UiModal.vue";
import CashEntryDialog from "../components/CashEntryDialog.vue";
import ExportButton from "../components/ExportButton.vue";
import {
  Channel,
  type Transaction,
  type TransactionGroup,
  formatCurrency,
  formatLoanLabel,
  formatSheetNumber,
  type CashBookRow,
  TransactionDirection,
  TransactionStatus,
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
    .map((transaction) => ({ ...transaction, ...plainWords(transaction) })),
);
const itemWords: Record<string, string> = {
  LOAN_PRINCIPAL: "pokok",
  LOAN_INTEREST: "bunga",
  LOAN_PROVISION: "provisi",
  LOAN_DISBURSEMENT: "pencairan",
  SAVINGS_DEPOSIT: "setor",
  SAVINGS_WITHDRAWAL: "ambil",
  SAVINGS_SHU: "SHU",
};
const accountWords: Record<string, string> = {
  POKOK: "pokok",
  WAJIB: "wajib",
  MANASUKA: "manasuka",
};
/** "Pembayaran pinjaman Rp 13.900.000" with "pokok 10.007.331 · bunga 200.147"
 * for a loan or savings movement; other rows keep their description. */
function plainWords(transaction: Transaction) {
  const items = transaction.components;
  const loan = loans.find((candidate) =>
    items.some((item) => item.loanId === candidate.id),
  );
  const parts = items.map((item) =>
    item.accountType
      ? `${accountWords[item.accountType] ?? item.accountType} ${formatSheetNumber(item.amount)}`
      : `${itemWords[item.componentType] ?? item.label} ${formatSheetNumber(item.amount)}`,
  );
  const imported = transaction.reference.includes("-MIG");
  if (items.some((item) => item.loanId)) {
    const disbursed = items.some(
      (item) => item.componentType === "LOAN_DISBURSEMENT",
    );
    return {
      title: `${disbursed ? "Pencairan" : "Pembayaran"} pinjaman${loan ? ` ${formatCurrency(loan.plafond)}` : ""}`,
      detail: [...parts, imported ? "dari sheet PINJAMAN" : ""]
        .filter(Boolean)
        .join(" · "),
    };
  }
  if (items.some((item) => item.accountType)) {
    const withdrawn = items.some(
      (item) => item.componentType === "SAVINGS_WITHDRAWAL",
    );
    return {
      title: withdrawn ? "Pengambilan simpanan" : "Setoran simpanan",
      detail: [...parts, imported ? "dari sheet SIMPANAN" : ""]
        .filter(Boolean)
        .join(" · "),
    };
  }
  return { title: transaction.description, detail: "" };
}

onMounted(() => {
  void loadFinancialParameters();
});

const rows = computed(() => cashBook.value?.rows ?? []);
const query = pageState.field("query", () => "");
const debouncedQuery = useDebouncedRef(query, pageState.tabId);
// Users see a reversal as a deletion: the reversed row and its reversal are never shown.
const filteredRows = computed(() => {
  const search = debouncedQuery.value.trim().toLowerCase();
  return rows.value.filter(
    (row) =>
      !isExcluded(row) &&
      (row.description.toLowerCase().includes(search) ||
        (search !== "" && row.memberNumber.toLowerCase().includes(search))),
  );
});
// "Riwayat hapus" swaps the sheet for every deleted row of the year, newest
// deletion first. The reversal row carries when it was deleted; its reference
// extends the original's.
const showDeleted = pageState.field("showDeleted", () => false);
const deletedRows = computed(() => {
  const reversals = rows.value.filter((row) => row.isReversal);
  return rows.value
    .filter((row) => row.status === TransactionStatus.Reversed)
    .map((row) => {
      const reversal = reversals.find((candidate) =>
        candidate.reference.startsWith(`${row.reference}/REV/`),
      );
      return {
        ...row,
        deletedAt: reversal
          ? [reversal.date, reversal.time].filter(Boolean).join(" ")
          : "",
        deletedOrder: reversal ? `${reversal.businessDate} ${reversal.id}` : "",
      };
    })
    .sort((a, b) => b.deletedOrder.localeCompare(a.deletedOrder));
});
const filteredDeletedRows = computed(() => {
  const search = debouncedQuery.value.trim().toLowerCase();
  return deletedRows.value.filter(
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
          :class="[
            'button',
            showDeleted ? 'button--active' : 'button--secondary',
          ]"
          type="button"
          :aria-pressed="showDeleted"
          @click="showDeleted = !showDeleted"
        >
          <History :size="16" /> Riwayat hapus
          <span v-if="deletedRows.length" class="deleted-count">{{
            deletedRows.length
          }}</span>
        </button>
        <ExportButton kind="cash-book" />
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
              <th>Anggota</th>
              <th>Debet</th>
              <th>Kredit</th>
              <th>Dihapus</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="item in filteredDeletedRows"
              :key="item.id"
              class="cash-book-table__row"
            >
              <td>{{ item.date }}</td>
              <td class="cash-book-table__desc">{{ item.description }}</td>
              <td>
                {{
                  [item.memberNumber, item.memberName].filter(Boolean).join(" ")
                }}
              </td>
              <td class="num-cell">
                {{
                  item.direction === TransactionDirection.Out
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td class="num-cell">
                {{
                  item.direction === TransactionDirection.In
                    ? formatSheetNumber(item.amount)
                    : ""
                }}
              </td>
              <td>{{ item.deletedAt }}</td>
            </tr>
            <tr v-if="!filteredDeletedRows.length">
              <td colspan="6">
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
              <td class="cash-book-table__actions"></td>
            </tr>
            <tr
              v-for="(item, index) in filteredRows"
              :key="item.id"
              class="cash-book-table__row"
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
              >{{ pendingDetail.date }}
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
            ><span>{{ pendingDelete.date }}</span>
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
                <small>{{ transaction.date }}</small>
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
.button--active {
  border: 1px solid var(--green);
  color: var(--green);
  background: var(--green-soft);
}
.deleted-count {
  min-width: 18px;
  padding: 1px 5px;
  color: var(--red);
  background: var(--red-soft);
  font-size: 10.5px;
}
</style>
