<script setup lang="ts">
import { ref } from "vue";
import {
  CheckCircle2,
  FileSpreadsheet,
  Loader2,
  UploadCloud,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const { totals, loans, importWorkbook } = useKoperasiStore();
const importing = ref(false);

async function handleImport() {
  importing.value = true;
  try {
    await importWorkbook();
  } finally {
    importing.value = false;
  }
}
</script>
<template>
  <div class="page-stack">
    <PageHeader
      eyebrow="Data staging"
      title="Migrasi Excel"
      description="Impor anggota, pinjaman, simpanan, dan buku kas langsung dari workbook Excel (.xlsm) ke database lokal."
      ><template #actions
        ><button
          class="button button--primary"
          :disabled="importing"
          @click="handleImport"
        >
          <Loader2 v-if="importing" :size="18" class="spin" />
          <UploadCloud v-else :size="18" />
          {{ importing ? "Mengimpor..." : "Impor workbook" }}
        </button></template
      ></PageHeader
    >
    <section class="migration-source panel">
      <span class="migration-source__icon"><FileSpreadsheet :size="28" /></span>
      <div>
        <p class="eyebrow">Sumber data</p>
        <h2>Workbook Excel (.xlsm)</h2>
        <span
          >Sheet yang dibaca: MASTER_SIMPANAN, SIMPANAN_2026, MASTER_PINJAMAN,
          PINJAMAN_2026, KAS_2026</span
        >
      </div>
      <StatusPill
        :label="totals.members.value > 0 ? 'Data tersedia' : 'Belum ada data'"
        :tone="totals.members.value > 0 ? 'success' : 'warning'"
      />
    </section>
    <section v-if="totals.members.value === 0" class="panel empty-import">
      <CheckCircle2 :size="22" style="opacity: 0.35" />
      <p>
        Database masih kosong. Pilih file workbook (.xlsm) melalui tombol
        <strong>Impor workbook</strong> di atas untuk memuat data anggota,
        pinjaman, simpanan, dan buku kas.
      </p>
    </section>
    <section v-else class="migration-metrics">
      <article>
        <strong>{{ totals.members.value }}</strong
        ><span>Anggota aktif</span><small>Tersimpan di database lokal</small>
      </article>
      <article>
        <strong>{{ loans.length }}</strong
        ><span>Pinjaman</span><small>Seluruh status</small>
      </article>
      <article>
        <strong>{{ formatCurrency(totals.savings.value, true) }}</strong
        ><span>Total simpanan</span><small>Pokok, wajib, dan manasuka</small>
      </article>
      <article>
        <strong>{{ formatCurrency(totals.cash.value, true) }}</strong
        ><span>Saldo kas</span><small>Hasil rekonstruksi buku kas</small>
      </article>
    </section>
  </div>
</template>
<style scoped>
.empty-import {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  color: var(--muted, #6b7280);
}
.spin {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
