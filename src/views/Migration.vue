<script setup lang="ts">
import { ref } from "vue";
import {
  CheckCircle2,
  FileSpreadsheet,
  Link2,
  TriangleAlert,
  UploadCloud,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import { useKoperasiStore } from "../store/koperasi";

const { notify } = useKoperasiStore();
const progress = ref(100);
const issues = ref([
  {
    source: "MASTER_PINJAMAN · baris 9",
    record: "Aah Rohayati Nursalim",
    issue: "Plafond sumber tidak valid",
    status: "Review finansial",
  },
  {
    source: "MASTER_PINJAMAN · baris 77",
    record: "Nama anggota ambigu",
    issue: "Tidak ada member match unik",
    status: "Padankan anggota",
  },
  {
    source: "PINJAMAN_2026 · L002",
    record: "Aam Faozah Hajah",
    issue: "Provisi aktual berbeda Rp11.000",
    status: "Review finansial",
  },
  {
    source: "KAS_2026 · 484 baris",
    record: "Transaksi operasional",
    issue: "Belum memiliki component mapping",
    status: "Review finansial",
  },
]);
function resolve(index: number) {
  issues.value.splice(index, 1);
  notify(
    "Issue ditandai selesai",
    "Keputusan review tersimpan di audit trail.",
  );
}
</script>
<template>
  <div class="page-stack">
    <PageHeader
      eyebrow="Data staging"
      title="Migrasi Excel"
      description="Validasi, padankan, dan rekonsiliasi data sumber sebelum masuk ke ledger produksi."
      ><template #actions
        ><button
          class="button button--primary"
          @click="
            notify(
              'Snapshot sudah terbaru',
              'Workbook 2026 telah dimuat ke staging.',
              'info',
            )
          "
        >
          <UploadCloud :size="18" /> Impor workbook
        </button></template
      ></PageHeader
    >
    <section class="migration-source panel">
      <span class="migration-source__icon"><FileSpreadsheet :size="28" /></span>
      <div>
        <p class="eyebrow">Sumber aktif</p>
        <h2>KOPERASI_BINA_SEJAHTERA_2026_DIBERSIHKAN.xlsm</h2>
        <span>8 sheet · snapshot 13 September 2026 · 3.560 baris data</span>
      </div>
      <StatusPill label="Staging selesai" tone="success" />
    </section>
    <section class="migration-steps">
      <article class="done">
        <span><CheckCircle2 :size="21" /></span>
        <div>
          <strong>1. Unggah sumber</strong
          ><small>Workbook dan metadata tersimpan</small>
        </div>
      </article>
      <i></i>
      <article class="done">
        <span><CheckCircle2 :size="21" /></span>
        <div>
          <strong>2. Validasi struktur</strong><small>8 sheet dikenali</small>
        </div>
      </article>
      <i></i>
      <article class="active">
        <span><TriangleAlert :size="21" /></span>
        <div>
          <strong>3. Review exception</strong
          ><small>{{ issues.length }} keputusan tersisa</small>
        </div>
      </article>
      <i></i>
      <article>
        <span>4</span>
        <div>
          <strong>Commit snapshot</strong><small>Menunggu review</small>
        </div>
      </article>
    </section>
    <section class="migration-metrics">
      <article>
        <strong>2.593</strong><span>Auto import</span
        ><small>Record valid & konsisten</small>
      </article>
      <article>
        <strong>{{ issues.length }}</strong
        ><span>Perlu keputusan</span><small>Ditahan dari produksi</small>
      </article>
      <article>
        <strong>144</strong><span>Anggota dipadankan</span
        ><small>98,6% confidence tinggi</small>
      </article>
      <article>
        <strong>{{ progress }}%</strong><span>Struktur tervalidasi</span
        ><small>8 dari 8 sheet</small>
      </article>
    </section>
    <section class="panel table-panel">
      <div class="panel__header panel__header--padded">
        <div>
          <p class="eyebrow">Review queue</p>
          <h2>Exception migrasi</h2>
        </div>
        <span class="count-badge count-badge--warning">{{
          issues.length
        }}</span>
      </div>
      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Lokasi sumber</th>
              <th>Record</th>
              <th>Masalah</th>
              <th>Status</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(issue, index) in issues" :key="issue.source">
              <td>
                <code>{{ issue.source }}</code>
              </td>
              <td>
                <strong>{{ issue.record }}</strong>
              </td>
              <td>{{ issue.issue }}</td>
              <td><StatusPill :label="issue.status" tone="warning" /></td>
              <td>
                <button class="row-action" @click="resolve(index)">
                  <Link2 :size="15" /> Review
                </button>
              </td>
            </tr>
            <tr v-if="issues.length === 0">
              <td colspan="5">
                <div class="empty-inline">
                  <CheckCircle2 :size="22" /> Semua exception sudah
                  diselesaikan.
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
