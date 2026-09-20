<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ArrowDownLeft,
  ArrowUpRight,
  CalendarX2,
  Download,
  MoreHorizontal,
  Search,
  Undo2,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import UiModal from "../components/UiModal.vue";
import {
  formatCurrency,
  TransactionDirection,
  TransactionStatus,
  type Transaction,
  useKoperasiStore,
} from "../store/koperasi";

const {
  selectedYear,
  yearTransactions,
  yearTotals,
  reverseTransaction,
  notify,
  refresh,
} = useKoperasiStore();
const query = ref("");
const selected = ref<Transaction | null>(null);
const cashHasData = computed(() => yearTransactions.value.length > 0);
const filtered = computed(() =>
  yearTransactions.value.filter((item) =>
    `${item.description} ${item.memberName} ${item.reference}`
      .toLowerCase()
      .includes(query.value.toLowerCase()),
  ),
);
const cashIn = computed(() =>
  yearTransactions.value
    .filter((item) => item.direction === TransactionDirection.In)
    .reduce((sum, item) => sum + item.amount, 0),
);
const cashOut = computed(() =>
  yearTransactions.value
    .filter((item) => item.direction === TransactionDirection.Out)
    .reduce((sum, item) => sum + item.amount, 0),
);
async function reverseSelected() {
  if (!selected.value) return;
  const reversed = await reverseTransaction(selected.value.id);
  if (!reversed) return;
  selected.value = null;
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Buku kas" :refresh="refresh"
      ><template #actions
        ><button
          class="button button--secondary"
          @click="
            notify(
              'Ekspor disiapkan',
              `Buku kas ${selectedYear} akan diekspor.`,
              'info',
            )
          "
        >
          <Download :size="18" /> Ekspor buku kas
        </button></template
      ></PageHeader
    >
    <section v-if="!cashHasData" class="panel year-empty-state">
      <CalendarX2 :size="36" />
      <strong>Belum ada transaksi kas untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain untuk melihat mutasi buku kas.</p>
    </section>
    <template v-else>
      <section class="balance-band">
        <div>
          <span>Kas masuk {{ selectedYear }}</span
          ><strong class="amount-in">+{{ formatCurrency(cashIn) }}</strong>
        </div>
        <i></i>
        <div>
          <span>Kas keluar {{ selectedYear }}</span
          ><strong class="amount-out">−{{ formatCurrency(cashOut) }}</strong>
        </div>
        <i></i>
        <div class="balance-band__ending">
          <span>Kas net</span
          ><strong>{{ formatCurrency(yearTotals.cash) }}</strong>
        </div>
      </section>
      <section class="panel table-panel">
        <div class="toolbar">
          <label class="search-field"
            ><Search :size="18" /><input
              v-model="query"
              placeholder="Cari uraian, anggota, atau referensi..."
          /></label>
          <div class="toolbar__meta">Tahun {{ selectedYear }}</div>
        </div>
        <div class="data-table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>Tanggal</th>
                <th>Transaksi</th>
                <th>Referensi</th>
                <th>Kas masuk</th>
                <th>Kas keluar</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in filtered" :key="item.id">
                <td>
                  {{ item.date }}<small class="cell-sub">{{ item.time }}</small>
                </td>
                <td>
                  <div class="ledger-description">
                    <span
                      :class="
                        item.direction === TransactionDirection.In
                          ? 'is-in'
                          : 'is-out'
                      "
                      ><ArrowDownLeft
                        v-if="item.direction === TransactionDirection.In"
                        :size="17" /><ArrowUpRight v-else :size="17"
                    /></span>
                    <div>
                      <strong>{{ item.description }}</strong
                      ><small>{{ item.memberName }} · {{ item.id }}</small>
                    </div>
                  </div>
                </td>
                <td>
                  <code>{{ item.reference }}</code>
                </td>
                <td class="num-cell amount-in">
                  {{
                    item.direction === TransactionDirection.In
                      ? formatCurrency(item.amount)
                      : "—"
                  }}
                </td>
                <td class="num-cell amount-out">
                  {{
                    item.direction === TransactionDirection.Out
                      ? formatCurrency(item.amount)
                      : "—"
                  }}
                </td>
                <td>
                  <StatusPill
                    :label="item.status"
                    :tone="
                      item.status === TransactionStatus.Posted
                        ? 'success'
                        : item.status === TransactionStatus.Reversed
                          ? 'neutral'
                          : 'warning'
                    "
                  />
                </td>
                <td>
                  <button class="icon-button" @click="selected = item">
                    <MoreHorizontal :size="18" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
    <UiModal
      :open="Boolean(selected)"
      title="Detail transaksi"
      description="Jejak sumber dan split komponen transaksi."
      @close="selected = null"
      ><div v-if="selected" class="transaction-detail">
        <div class="transaction-detail__head">
          <div>
            <small>{{ selected.id }}</small
            ><strong>{{ selected.description }}</strong
            ><span
              >{{ selected.memberName }} · {{ selected.date }},
              {{ selected.time }}</span
            >
          </div>
          <strong>{{ formatCurrency(selected.amount) }}</strong>
        </div>
        <div class="component-list">
          <div v-for="component in selected.components" :key="component.label">
            <span>{{ component.label }}</span
            ><strong>{{ formatCurrency(component.amount) }}</strong>
          </div>
          <div class="is-total">
            <span>Total {{ selected.direction.toLowerCase() }}</span
            ><strong>{{ formatCurrency(selected.amount) }}</strong>
          </div>
        </div>
        <div class="audit-note">
          Sumber posting: {{ selected.actor }} · Riwayat finansial tidak dapat
          diedit langsung.
        </div>
        <div class="modal-actions">
          <button
            class="button button--danger"
            :disabled="selected.status !== TransactionStatus.Posted"
            @click="reverseSelected"
          >
            <Undo2 :size="17" /> Buat reversal</button
          ><button class="button button--primary" @click="selected = null">
            Selesai
          </button>
        </div>
      </div></UiModal
    >
  </div>
</template>
