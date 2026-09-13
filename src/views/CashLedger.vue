<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ArrowDownLeft,
  ArrowUpRight,
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
  type Transaction,
  useKoperasiStore,
} from "../store/koperasi";

const { transactions, totals, reverseTransaction, notify } = useKoperasiStore();
const query = ref("");
const selected = ref<Transaction | null>(null);
const filtered = computed(() =>
  transactions.filter((item) =>
    `${item.description} ${item.memberName} ${item.reference}`
      .toLowerCase()
      .includes(query.value.toLowerCase()),
  ),
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
    <PageHeader
      eyebrow="Ledger otomatis"
      title="Buku kas"
      description="Setiap kas masuk dan keluar terhubung ke transaksi sumber dan komponen sub-ledger."
      ><template #actions
        ><button
          class="button button--secondary"
          @click="
            notify(
              'Ekspor disiapkan',
              'Buku kas September akan diekspor.',
              'info',
            )
          "
        >
          <Download :size="18" /> Ekspor buku kas
        </button></template
      ></PageHeader
    >
    <section class="balance-band">
      <div><span>Saldo awal September</span><strong>Rp43.674.064</strong></div>
      <i></i>
      <div>
        <span>Kas masuk</span><strong class="amount-in">+Rp180.325.747</strong>
      </div>
      <i></i>
      <div>
        <span>Kas keluar</span><strong class="amount-out">−Rp88.318.000</strong>
      </div>
      <i></i>
      <div class="balance-band__ending">
        <span>Saldo berjalan</span
        ><strong>{{ formatCurrency(totals.cash.value) }}</strong>
      </div>
    </section>
    <section class="panel table-panel">
      <div class="toolbar">
        <label class="search-field"
          ><Search :size="18" /><input
            v-model="query"
            placeholder="Cari uraian, anggota, atau referensi..." /></label
        ><select class="select-control">
          <option>September 2026</option>
          <option>Agustus 2026</option>
        </select>
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
                  <span :class="item.direction === 'Masuk' ? 'is-in' : 'is-out'"
                    ><ArrowDownLeft
                      v-if="item.direction === 'Masuk'"
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
                  item.direction === "Masuk" ? formatCurrency(item.amount) : "—"
                }}
              </td>
              <td class="num-cell amount-out">
                {{
                  item.direction === "Keluar"
                    ? formatCurrency(item.amount)
                    : "—"
                }}
              </td>
              <td>
                <StatusPill
                  :label="item.status"
                  :tone="
                    item.status === 'Terposting'
                      ? 'success'
                      : item.status === 'Dibalik'
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
            :disabled="selected.status !== 'Terposting'"
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
