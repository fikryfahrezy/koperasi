<script setup lang="ts">
import {
  CheckCircle2,
  LockKeyhole,
  Scale,
  TriangleAlert,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const { notify } = useKoperasiStore();
const months = [
  ["Sep 2026", 180325747, 88318000, 49580570, 92007747, 42427177, "Review"],
  ["Agu 2026", 145664638, 140388100, 23819768, 5276538, -18543230, "Selesai"],
  ["Jul 2026", 95897816, 89400000, 8352746, 6497816, -1854930, "Selesai"],
  ["Jun 2026", 133097000, 135763000, -23900250, -2666000, 21234250, "Selesai"],
  ["Mei 2026", 230100404, 238663500, -58291199, -8563096, 49728103, "Selesai"],
];
</script>
<template>
  <div class="page-stack">
    <PageHeader
      eyebrow="Kontrol finansial"
      title="Rekonsiliasi"
      description="Bandingkan kas dengan pergerakan sub-ledger dan jelaskan setiap selisih operasional."
      ><template #actions
        ><button
          class="button button--primary"
          @click="
            notify(
              'Belum dapat ditutup',
              'Selesaikan 6 exception aktif sebelum mengunci September.',
              'warning',
            )
          "
        >
          <LockKeyhole :size="18" /> Tutup periode
        </button></template
      ></PageHeader
    >
    <section class="recon-hero">
      <div>
        <span class="metric-icon metric-icon--green"><Scale :size="21" /></span>
        <p>September 2026</p>
        <h2>Rekonsiliasi belum seimbang</h2>
        <small
          >Kas net dan sub-ledger net masih memiliki selisih operasional yang
          perlu diklasifikasikan.</small
        >
      </div>
      <dl>
        <div>
          <dt>Kas net</dt>
          <dd>Rp92.007.747</dd>
        </div>
        <div>
          <dt>Sub-ledger net</dt>
          <dd>Rp49.580.570</dd>
        </div>
        <div class="is-difference">
          <dt>Selisih</dt>
          <dd>Rp42.427.177</dd>
        </div>
      </dl>
    </section>
    <section class="recon-grid">
      <article class="panel">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Exception queue</p>
            <h2>6 item perlu review</h2>
          </div>
          <span class="count-badge count-badge--warning">6</span>
        </div>
        <div class="exception-list">
          <div>
            <span><TriangleAlert :size="18" /></span>
            <div>
              <strong>Biaya operasional belum diklasifikasi</strong
              ><small>4 transaksi · Rp3.686.500</small>
            </div>
            <button>Review</button>
          </div>
          <div>
            <span><TriangleAlert :size="18" /></span>
            <div>
              <strong>Pencairan tanpa kontrak terpadan</strong
              ><small>1 transaksi · Rp5.000.000</small>
            </div>
            <button>Review</button>
          </div>
          <div>
            <span><TriangleAlert :size="18" /></span>
            <div>
              <strong>Selisih migrasi buku kas</strong
              ><small>1 kelompok sumber · Rp33.740.677</small>
            </div>
            <button>Review</button>
          </div>
        </div>
      </article>
      <aside class="panel control-checklist">
        <p class="eyebrow">Checklist tutup bulan</p>
        <h2>3 dari 5 selesai</h2>
        <div>
          <p class="done">
            <CheckCircle2 :size="18" /> Semua transaksi sudah diposting
          </p>
          <p class="done">
            <CheckCircle2 :size="18" /> Jadwal Oktober sudah dibuat
          </p>
          <p class="done">
            <CheckCircle2 :size="18" /> Backup database berhasil
          </p>
          <p><span>4</span> Seluruh exception terselesaikan</p>
          <p><span>5</span> Persetujuan reviewer</p>
        </div>
        <div class="progress"><i style="width: 60%"></i></div>
      </aside>
    </section>
    <section class="panel table-panel">
      <div class="panel__header panel__header--padded">
        <div>
          <p class="eyebrow">Riwayat</p>
          <h2>Rekonsiliasi per bulan</h2>
        </div>
      </div>
      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>Periode</th>
              <th>Kas masuk</th>
              <th>Kas keluar</th>
              <th>Sub-ledger net</th>
              <th>Kas net</th>
              <th>Operasional/lainnya</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in months" :key="String(row[0])">
              <td>
                <strong>{{ row[0] }}</strong>
              </td>
              <td class="num-cell">{{ formatCurrency(Number(row[1])) }}</td>
              <td class="num-cell">{{ formatCurrency(Number(row[2])) }}</td>
              <td class="num-cell">{{ formatCurrency(Number(row[3])) }}</td>
              <td class="num-cell">{{ formatCurrency(Number(row[4])) }}</td>
              <td
                class="num-cell"
                :class="Number(row[5]) !== 0 ? 'text-warning' : ''"
              >
                {{ formatCurrency(Number(row[5])) }}
              </td>
              <td>
                <StatusPill
                  :label="String(row[6])"
                  :tone="row[6] === 'Selesai' ? 'success' : 'warning'"
                />
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
