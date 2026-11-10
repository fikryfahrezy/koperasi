<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { PencilLine, Plus, X } from "lucide-vue-next";
import BalanceSheetListPanel from "../components/BalanceSheetListPanel.vue";
import SheetAmountInput from "../components/SheetAmountInput.vue";
import PageHeader from "../components/PageHeader.vue";
import UiSelect from "../components/UiSelect.vue";
import { useWorkspacePageState } from "../composables/useWorkspacePageState";
import {
  BalanceSheetItem,
  type LineEdit,
  LineEditKind,
  balanceSheetAt,
  sheetRows,
} from "../balance-sheet";
import {
  type BalanceSheetDetail,
  Channel,
  formatPeriod,
  formatSheetNumber,
  useBalanceSheetDetails,
  useBalanceSheetItems,
  useCashBook,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const pageState = useWorkspacePageState("/balance-sheet");
const { loans, selectedYear, refresh, setBalanceSheetItems } =
  useKoperasiStore();
const { monthlyLedger, monthlyLedgerLoading } = useMonthlyLedger();
const { cashBook } = useCashBook(Channel.Cash);
const { balanceSheetItems } = useBalanceSheetItems();
// The first column is 31 Desember of the year before, as in the sheet.
const { monthlyLedger: previousLedger } = useMonthlyLedger(-1);
const { cashBook: previousCashBook } = useCashBook(Channel.Cash, -1);
const { balanceSheetItems: previousItems } = useBalanceSheetItems(-1);
const { balanceSheetDetails } = useBalanceSheetDetails();
const { balanceSheetDetails: previousDetails } = useBalanceSheetDetails(-1);
// Catatan "Piutang Lainnya (Bon Sementara) terdiri": the rincian per person.
const otherLoansByName = computed(() => [
  ...(balanceSheetDetails.value[BalanceSheetItem.OtherLoans] ?? [])
    .reduce(
      (totals, detail) =>
        totals.set(detail.name, (totals.get(detail.name) ?? 0) + detail.amount),
      new Map<string, number>(),
    )
    .entries(),
]);
// The Catatan list being edited (Piutang Lainnya), if any.
const editing = ref<BalanceSheetItem | null>(null);
// Typed cells belong to the book year of their column: 31 Desember of the
// year before, then the year shown.
const columnYears = computed(() => [
  selectedYear.value - 1,
  selectedYear.value,
]);
const columnDetails = computed(() => [
  withDraftLists(selectedYear.value - 1, previousDetails.value),
  withDraftLists(selectedYear.value, balanceSheetDetails.value),
]);
async function commitCell(edit: LineEdit, column: number, value: number) {
  const year = columnYears.value[column];
  if (edit.kind === LineEditKind.Item) {
    draftItems.value = {
      ...draftItems.value,
      [year]: {
        ...draftItems.value[year],
        [edit.key]: value - (edit.offset ?? 0),
      },
    };
    return;
  }
  const list = columnDetails.value[column][edit.key] ?? [];
  // Typed as shown: a negative amount (e.g. Akum. Penyusutan) with "-".
  const saved = list.some((detail) => detail.name === edit.name)
    ? list.map((detail) =>
        detail.name === edit.name ? { ...detail, amount: value } : detail,
      )
    : [...list, { name: edit.name, amount: value, loanDate: null, note: "" }];
  saveList(year, edit.key, saved);
}

function saveList(
  year: number,
  key: BalanceSheetItem,
  list: BalanceSheetDetail[],
) {
  draftLists.value = {
    ...draftLists.value,
    [year]: { ...draftLists.value[year], [key]: list },
  };
}

const savingChanges = ref(false);
async function saveChanges() {
  savingChanges.value = true;
  try {
    const years = new Set(
      [...Object.keys(draftItems.value), ...Object.keys(draftLists.value)].map(
        Number,
      ),
    );
    for (const year of years) {
      const saved = await setBalanceSheetItems(
        year,
        draftItems.value[year] ?? {},
        draftLists.value[year] ?? {},
      );
      if (!saved) return;
      const { [year]: _items, ...items } = draftItems.value;
      const { [year]: _lists, ...lists } = draftLists.value;
      draftItems.value = items;
      draftLists.value = lists;
    }
  } finally {
    savingChanges.value = false;
  }
}
function discardChanges() {
  draftItems.value = {};
  draftLists.value = {};
}

// A pos of a group's list in the Neraca: renamed in both years shown so
// the row stays one row; added and removed in the year shown.
function renamePos(key: BalanceSheetItem, from: string, to: string) {
  const name = to.trim();
  if (!name || name === from) return;
  for (const [column, year] of columnYears.value.entries()) {
    const list = columnDetails.value[column][key] ?? [];
    if (!list.some((detail) => detail.name === from)) continue;
    saveList(
      year,
      key,
      list.map((detail) =>
        detail.name === from ? { ...detail, name } : detail,
      ),
    );
  }
}
function addPos(key: BalanceSheetItem) {
  const list = columnDetails.value[1][key] ?? [];
  const names = new Set(
    columnDetails.value.flatMap((details) =>
      (details[key] ?? []).map((detail) => detail.name),
    ),
  );
  let name = "Pos baru";
  for (let count = 2; names.has(name); count++) name = `Pos baru ${count}`;
  saveList(selectedYear.value, key, [
    ...list,
    { name, amount: 0, loanDate: null, note: "" },
  ]);
}
// Nothing is stored until Simpan, so removing needs no extra confirmation.
function removePos(key: BalanceSheetItem, name: string) {
  const list = columnDetails.value[1][key] ?? [];
  saveList(
    selectedYear.value,
    key,
    list.filter((detail) => detail.name !== name),
  );
}
const sumOf = (entries: { amount: number }[] | [string, number][]) =>
  entries.reduce(
    (total, entry) => total + (Array.isArray(entry) ? entry[1] : entry.amount),
    0,
  );

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

// Changes typed in the Neraca stay here, per book year, until Simpan; the
// Neraca shows them already so the totals can be checked first.
const draftItems = ref<Record<number, Record<string, number>>>({});
const draftLists = ref<Record<number, Record<string, BalanceSheetDetail[]>>>(
  {},
);
const hasChanges = computed(
  () =>
    Object.keys(draftItems.value).length > 0 ||
    Object.keys(draftLists.value).length > 0,
);
const withDraftItems = (year: number, saved: Record<string, number>) => ({
  ...saved,
  ...draftItems.value[year],
});
const withDraftLists = (
  year: number,
  saved: Record<string, BalanceSheetDetail[]>,
) => ({ ...saved, ...draftLists.value[year] });

const current = computed(() =>
  balanceSheetAt(
    selectedPeriod.value,
    monthlyLedger.value,
    cashBook.value,
    loans,
    withDraftItems(selectedYear.value, balanceSheetItems.value),
    withDraftLists(selectedYear.value, balanceSheetDetails.value),
  ),
);
const previous = computed(() =>
  balanceSheetAt(
    previousPeriod.value,
    previousLedger.value,
    previousCashBook.value,
    loans,
    withDraftItems(selectedYear.value - 1, {
      ...previousItems.value,
      // Without a typed balance, this year's opening Kas Buku Tabungan is the
      // bank at 31 Desember.
      [BalanceSheetItem.BankBalance]:
        previousItems.value[BalanceSheetItem.BankBalance] ||
        (cashBook.value?.savingsBookOpeningBalance ?? 0),
    }),
    withDraftLists(selectedYear.value - 1, previousDetails.value),
  ),
);
const rows = computed(() => sheetRows([previous.value, current.value]));
const differences = computed(() => [
  previous.value.difference,
  current.value.difference,
]);

const lastDay = (value: string) => {
  const [year, month] = value.split("-").map(Number);
  if (!year) return "";
  return `${new Date(year, month, 0).getDate()} ${formatPeriod(value)}`;
};
const columnLabel = (value: string) => {
  const [year, month] = value.split("-").map(Number);
  return new Date(year, month - 1, 1)
    .toLocaleDateString("en-US", {
      month: "short",
      year: "2-digit",
    })
    .replace(" ", "-");
};
const columns = computed(() => [
  columnLabel(previousPeriod.value),
  columnLabel(selectedPeriod.value),
]);
const hasEstimate = computed(() =>
  rows.value.some((row) =>
    [row.left, row.right].some(
      (cell) => cell.kind === "line" && cell.estimated.some(Boolean),
    ),
  ),
);
</script>

<template>
  <div class="page-stack balance-page">
    <PageHeader title="Neraca" :refresh="refresh">
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
    </PageHeader>

    <section
      v-if="monthlyLedgerLoading && !periods.length"
      class="panel balance-empty"
    >
      Memuat data…
    </section>
    <section v-else-if="!periods.length" class="panel balance-empty">
      <strong>Belum ada data untuk {{ selectedYear }}</strong>
    </section>

    <template v-else>
      <p v-if="current.difference !== 0" class="balance-check" role="status">
        Belum seimbang: aktiva − pasiva =
        <strong>{{ formatSheetNumber(current.difference) }}</strong> per
        {{ lastDay(selectedPeriod) }}. Periksa pos neraca {{ selectedYear }}
        dan kategori biaya di Buku Kas.
      </p>

      <section class="panel balance-sheet">
        <header class="balance-sheet__title">
          <h2>KOPERASI PENSIUNAN BRI BINA SEJAHTERA</h2>
          <strong>NERACA</strong>
          <u>POSISI {{ lastDay(selectedPeriod).toUpperCase() }}</u>
        </header>
        <div class="balance-sheet__scroll">
          <table class="balance-table">
            <colgroup>
              <col class="balance-table__label" />
              <col class="balance-table__amount" />
              <col class="balance-table__amount" />
              <col class="balance-table__label" />
              <col class="balance-table__amount" />
              <col class="balance-table__amount" />
            </colgroup>
            <thead>
              <tr>
                <th colspan="3">AKTIVA</th>
                <th colspan="3">PASIVA</th>
              </tr>
              <tr>
                <th>URAIAN</th>
                <th v-for="label in columns" :key="`a-${label}`">
                  {{ label }}
                </th>
                <th>URAIAN</th>
                <th v-for="label in columns" :key="`p-${label}`">
                  {{ label }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, index) in rows"
                :key="index"
                :class="{
                  'balance-table__grand': index === rows.length - 1,
                }"
              >
                <template
                  v-for="(cell, side) in [row.left, row.right]"
                  :key="side"
                >
                  <template v-if="cell.kind === 'heading'">
                    <td class="balance-table__heading">
                      {{ cell.label }}
                      <!-- Each group takes pos added by hand. -->
                      <button
                        class="balance-notes__edit"
                        type="button"
                        :aria-label="`Tambah pos ${cell.label}`"
                        @click="addPos(cell.listKey)"
                      >
                        <Plus :size="13" /> Pos
                      </button>
                    </td>
                    <td></td>
                    <td></td>
                  </template>
                  <template v-else-if="cell.kind === 'line'">
                    <td v-if="cell.listKey" class="balance-table__pos">
                      <span>{{ cell.number }}.</span>
                      <input
                        class="balance-table__pos-name"
                        :value="cell.name"
                        aria-label="Nama pos"
                        title="Klik untuk mengganti nama"
                        @keydown.enter.prevent="
                          ($event.target as HTMLInputElement).blur()
                        "
                        @keydown.esc.prevent="
                          ($event.target as HTMLInputElement).value = cell.name;
                          ($event.target as HTMLInputElement).blur();
                        "
                        @change="
                          renamePos(
                            cell.listKey,
                            cell.name,
                            ($event.target as HTMLInputElement).value,
                          )
                        "
                      />
                      <button
                        class="balance-table__pos-remove"
                        type="button"
                        :aria-label="`Hapus pos ${cell.name}`"
                        @click="removePos(cell.listKey, cell.name)"
                      >
                        <X :size="12" />
                      </button>
                    </td>
                    <td v-else>{{ cell.label }}</td>
                    <td
                      v-for="(amount, column) in cell.amounts"
                      :key="column"
                      class="num-cell"
                    >
                      <SheetAmountInput
                        v-if="cell.edits[column]"
                        :value="amount"
                        :label="`${cell.label} ${columnYears[column]}`"
                        @commit="
                          (value) =>
                            commitCell(cell.edits[column]!, column, value)
                        "
                      />
                      <template v-else>{{ formatSheetNumber(amount) }}</template
                      ><sup v-if="cell.estimated[column]">*</sup>
                    </td>
                  </template>
                  <template v-else-if="cell.kind === 'total'">
                    <td class="balance-table__total">{{ cell.label }}</td>
                    <td
                      v-for="(amount, column) in cell.amounts"
                      :key="column"
                      class="num-cell balance-table__total"
                    >
                      {{ formatSheetNumber(amount) }}
                    </td>
                  </template>
                  <template v-else>
                    <td></td>
                    <td></td>
                    <td></td>
                  </template>
                </template>
              </tr>
              <tr class="balance-table__difference">
                <td colspan="3"></td>
                <td>Selisih aktiva − pasiva</td>
                <td
                  v-for="(amount, column) in differences"
                  :key="column"
                  class="num-cell"
                >
                  {{ amount === 0 ? "0" : formatSheetNumber(amount) }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="hasChanges" class="balance-changes" role="status">
          <span>Ada perubahan yang belum disimpan.</span>
          <button
            class="button"
            type="button"
            :disabled="savingChanges"
            @click="discardChanges"
          >
            Batal
          </button>
          <button
            class="button button--primary"
            type="button"
            :disabled="savingChanges"
            @click="saveChanges"
          >
            {{ savingChanges ? "Menyimpan..." : "Simpan" }}
          </button>
        </div>
        <p class="field-hint">
          Angka bergaris bawah putus-putus bisa diklik lalu diubah langsung (Esc
          untuk batal). Perubahan baru tersimpan setelah klik Simpan. Kolom 31
          Desember menyimpan ke tahun buku {{ selectedYear - 1 }}, kolom bulan
          ke tahun buku {{ selectedYear }}. Kas yang diubah disimpan sebagai
          Penyesuaian kas.
        </p>
        <div class="balance-notes">
          <strong>Catatan :</strong>
          <table
            v-if="current.nonMemberByRecommender.length"
            class="balance-notes__table"
          >
            <caption>
              Piutang Non Anggota (Rekomendasi)
            </caption>
            <tbody>
              <tr
                v-for="(entry, index) in current.nonMemberByRecommender"
                :key="entry.recommender"
              >
                <td>
                  {{ index + 1 }}.
                  {{ entry.recommender || "Belum ada rekomendasi" }}
                </td>
                <td class="num-cell">{{ formatSheetNumber(entry.amount) }}</td>
              </tr>
            </tbody>
            <tfoot>
              <tr>
                <td></td>
                <td class="num-cell">
                  {{ formatSheetNumber(sumOf(current.nonMemberByRecommender)) }}
                </td>
              </tr>
            </tfoot>
          </table>
          <BalanceSheetListPanel
            v-if="editing === BalanceSheetItem.OtherLoans"
            :item-key="BalanceSheetItem.OtherLoans"
            title="Piutang Lainnya"
            @done="editing = null"
          />
          <table v-else class="balance-notes__table">
            <caption>
              Piutang Lainnya (Bon Sementara) terdiri :
              <button
                class="balance-notes__edit"
                type="button"
                @click="editing = BalanceSheetItem.OtherLoans"
              >
                <PencilLine :size="13" /> Ubah
              </button>
            </caption>
            <tbody>
              <tr
                v-for="([name, amount], index) in otherLoansByName"
                :key="name"
              >
                <td>{{ index + 1 }}. {{ name }}</td>
                <td class="num-cell">{{ formatSheetNumber(amount) }}</td>
              </tr>
              <tr v-if="!otherLoansByName.length">
                <td colspan="2" class="balance-notes__empty">Belum diisi</td>
              </tr>
            </tbody>
            <tfoot>
              <tr>
                <td></td>
                <td class="num-cell">
                  {{ formatSheetNumber(sumOf(otherLoansByName)) }}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
        <p v-if="hasEstimate" class="field-hint">
          * Bank dihitung dari Kas Buku Tabungan dan belum tentu sama dengan
          saldo bank. Ketik saldo bank 31 Desember di selnya pada bulan
          Desember.
        </p>
      </section>

      <section class="panel balance-income">
        <h2>Laba Bersih s/d {{ formatPeriod(selectedPeriod) }}</h2>
        <table class="balance-income__table">
          <tbody>
            <tr>
              <td>Pendapatan bunga pinjaman</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.interest) }}
              </td>
            </tr>
            <tr>
              <td>Pendapatan provisi pinjaman</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.provision) }}
              </td>
            </tr>
            <tr v-for="expense in current.income.expenses" :key="expense.label">
              <td>{{ expense.label }}</td>
              <td class="num-cell">
                {{ formatSheetNumber(-expense.amount) }}
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td>Laba Bersih</td>
              <td class="num-cell">
                {{ formatSheetNumber(current.income.netIncome) }}
              </td>
            </tr>
          </tfoot>
        </table>
        <p class="field-hint">
          Biaya diambil dari baris Buku Kas yang diberi kategori biaya. Baris
          tanpa kategori tidak ikut dihitung.
        </p>
      </section>
    </template>
  </div>
</template>

<style scoped>
.balance-page {
  gap: 12px;
  padding-bottom: 16px;
}
.balance-empty {
  padding: 32px;
  color: var(--muted);
  text-align: center;
}
.balance-check {
  margin: 0;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-left: 3px solid var(--amber);
  background: var(--amber-soft);
  font-size: 12px;
}
.balance-sheet {
  padding: 18px 20px;
  min-width: 0;
}
.balance-sheet__title {
  display: grid;
  justify-items: center;
  gap: 4px;
  margin-bottom: 14px;
  font-size: 13px;
}
.balance-sheet__title h2 {
  margin: 0;
  color: var(--green);
  font-size: 17px;
  font-weight: 600;
}
.balance-sheet__scroll {
  overflow-x: auto;
}
.balance-table {
  width: 100%;
  min-width: 900px;
  border-collapse: collapse;
  font-size: 12px;
}
.balance-table__label {
  width: 26%;
}
.balance-table__amount {
  width: 12%;
}
.balance-table th,
.balance-table td {
  padding: 5px 7px;
  border: 1px solid var(--line-strong);
}
.balance-table thead th {
  font-style: italic;
  font-weight: 800;
  text-align: center;
}
.balance-table td:nth-child(4) {
  border-left: 2px solid var(--ink);
}
.balance-table__heading {
  font-style: italic;
  font-weight: 800;
  text-decoration: underline;
}
.balance-table__total {
  font-style: italic;
  font-weight: 800;
}
.balance-table__grand td {
  border-top: 2px solid var(--ink);
  border-bottom: 3px double var(--ink);
}
.balance-table__difference td {
  border: 0;
  color: var(--muted);
  font-style: italic;
}
.balance-table sup {
  color: var(--amber);
}
.balance-notes {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: 8px 48px;
  margin-top: 16px;
  font-size: 12px;
}
.balance-notes > strong {
  flex-basis: 100%;
}
.balance-notes__table {
  width: min(420px, 100%);
  border-collapse: collapse;
}
.balance-notes__table caption {
  padding-bottom: 4px;
  font-style: italic;
  font-weight: 700;
  text-align: left;
}
.balance-table__pos {
  display: flex;
  align-items: center;
  gap: 4px;
}
.balance-table__pos-name {
  flex: 1;
  min-width: 0;
  padding: 0;
  border: 0;
  border-bottom: 1px dashed var(--line-strong);
  background: transparent;
  color: inherit;
  font: inherit;
}
.balance-table__pos-name:focus {
  outline: 2px solid var(--green);
  outline-offset: 1px;
  background: #fff;
}
.balance-table__pos-remove {
  display: inline-grid;
  place-items: center;
  padding: 2px;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
}
.balance-table__pos-remove:hover {
  color: var(--red, #b42318);
}
.balance-changes {
  position: sticky;
  bottom: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  padding: 8px 12px;
  border: 1px solid #f0c36d;
  border-radius: 6px;
  background: #fff8e6;
}
.balance-changes span {
  flex: 1;
  font-weight: 600;
}
.balance-notes__edit {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  margin-left: 8px;
  padding: 1px 6px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: #fff;
  color: var(--muted);
  font-size: 11px;
  font-style: normal;
  font-weight: 600;
}
.balance-notes__edit:hover {
  border-color: var(--line-strong);
  color: var(--green);
}
.balance-notes__empty {
  color: var(--muted);
  font-style: italic;
}
.balance-notes__table td {
  padding: 3px 4px;
}
.balance-notes__table tfoot td {
  border-top: 1px solid var(--line-strong);
  font-weight: 800;
}
.balance-income {
  padding: 15px 17px;
}
.balance-income h2 {
  margin: 0 0 8px;
  font-size: 14px;
}
.balance-income__table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.balance-income__table td {
  padding: 6px 4px;
  border-bottom: 1px solid var(--line);
}
.balance-income__table tfoot td {
  padding-top: 10px;
  border-top: 2px solid var(--line-strong);
  border-bottom: 0;
  font-weight: 800;
}
.field-hint {
  margin: 8px 0 0;
  color: var(--soft);
  font-size: 10px;
}
</style>
