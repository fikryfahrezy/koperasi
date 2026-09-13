<script setup lang="ts">
import {
  ArrowDownLeft,
  ArrowRight,
  ArrowUpRight,
  Banknote,
  CircleDollarSign,
  Landmark,
  PiggyBank,
  Scale,
  ShieldCheck,
  TriangleAlert,
  Users,
} from "lucide-vue-next";
import { useRouter } from "vue-router";
import StatusPill from "../components/StatusPill.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const router = useRouter();
const { transactions, totals } = useKoperasiStore();
const cashflow = [
  { month: "Apr", masuk: 110.4, keluar: 98.7 },
  { month: "Mei", masuk: 230.1, keluar: 238.7 },
  { month: "Jun", masuk: 133.1, keluar: 135.8 },
  { month: "Jul", masuk: 95.9, keluar: 89.4 },
  { month: "Agu", masuk: 145.7, keluar: 140.4 },
  { month: "Sep", masuk: 180.3, keluar: 88.3 },
];
</script>

<template>
  <div class="page-stack">
    <section class="welcome-band">
      <div>
        <p class="eyebrow eyebrow--light">Ringkasan 13 September 2026</p>
        <h1>Ringkasan keuangan koperasi.</h1>
        <p>
          Semua transaksi hari ini sudah terposting. Ada 6 item yang perlu
          ditinjau sebelum tutup periode.
        </p>
      </div>
      <div class="welcome-band__status">
        <ShieldCheck :size="20" /><span>Periode Sep 2026</span
        ><strong>Terbuka</strong>
      </div>
    </section>

    <section class="metric-grid">
      <article class="metric-card">
        <div class="metric-card__top">
          <span class="metric-icon metric-icon--green"
            ><Banknote :size="20" /></span
          ><span class="trend trend--up"><ArrowUpRight :size="14" /> 8,4%</span>
        </div>
        <p>Saldo kas</p>
        <strong>{{ formatCurrency(totals.cash.value) }}</strong
        ><small>Terakhir diperbarui 14:32</small>
      </article>
      <article class="metric-card">
        <div class="metric-card__top">
          <span class="metric-icon metric-icon--blue"
            ><Landmark :size="20" /></span
          ><span class="metric-label">145 kontrak</span>
        </div>
        <p>Portofolio pinjaman</p>
        <strong>{{ formatCurrency(totals.loanPortfolio.value, true) }}</strong
        ><small>2 data perlu dilengkapi</small>
      </article>
      <article class="metric-card">
        <div class="metric-card__top">
          <span class="metric-icon metric-icon--amber"
            ><PiggyBank :size="20" /></span
          ><span class="metric-label">3 jenis</span>
        </div>
        <p>Total simpanan</p>
        <strong>{{ formatCurrency(totals.savings.value, true) }}</strong
        ><small>Pokok, wajib & manasuka</small>
      </article>
      <article class="metric-card">
        <div class="metric-card__top">
          <span class="metric-icon metric-icon--violet"
            ><Users :size="20" /></span
          ><span class="trend trend--up"
            ><ArrowUpRight :size="14" /> 3 baru</span
          >
        </div>
        <p>Anggota aktif</p>
        <strong>{{ totals.members.value }}</strong
        ><small>98,6% data terverifikasi</small>
      </article>
    </section>

    <section class="dashboard-grid">
      <article class="panel panel--chart">
        <header class="panel__header">
          <div>
            <p class="eyebrow">Arus kas</p>
            <h2>Pergerakan enam bulan</h2>
          </div>
          <div class="legend">
            <span><i class="legend__dot legend__dot--in"></i>Masuk</span
            ><span><i class="legend__dot legend__dot--out"></i>Keluar</span>
          </div>
        </header>
        <div class="cash-chart">
          <div
            v-for="item in cashflow"
            :key="item.month"
            class="cash-chart__group"
          >
            <div class="cash-chart__bars">
              <span
                class="cash-chart__bar cash-chart__bar--in"
                :style="{ height: `${item.masuk / 2.6}px` }"
                :title="`${item.masuk} juta`"
              ></span
              ><span
                class="cash-chart__bar cash-chart__bar--out"
                :style="{ height: `${item.keluar / 2.6}px` }"
                :title="`${item.keluar} juta`"
              ></span>
            </div>
            <small>{{ item.month }}</small>
          </div>
        </div>
        <div class="chart-summary">
          <div>
            <ArrowDownLeft :size="18" /><span>Kas masuk Sep</span
            ><strong>Rp180,3 jt</strong>
          </div>
          <div>
            <ArrowUpRight :size="18" /><span>Kas keluar Sep</span
            ><strong>Rp88,3 jt</strong>
          </div>
          <div>
            <Scale :size="18" /><span>Kas net</span><strong>+Rp92,0 jt</strong>
          </div>
        </div>
      </article>

      <aside class="panel action-panel">
        <header class="panel__header">
          <div>
            <p class="eyebrow">Aksi cepat</p>
            <h2>Mulai transaksi</h2>
          </div>
        </header>
        <button
          class="quick-action quick-action--primary"
          type="button"
          @click="router.push('/transactions')"
        >
          <span><CircleDollarSign :size="21" /></span>
          <div>
            <strong>Terima pembayaran</strong
            ><small>Split pinjaman & simpanan</small>
          </div>
          <ArrowRight :size="18" />
        </button>
        <button
          class="quick-action"
          type="button"
          @click="router.push('/loans')"
        >
          <span><Landmark :size="21" /></span>
          <div>
            <strong>Buat pinjaman</strong
            ><small>Preview jadwal & provisi</small>
          </div>
          <ArrowRight :size="18" />
        </button>
        <button
          class="quick-action"
          type="button"
          @click="router.push('/members')"
        >
          <span><Users :size="21" /></span>
          <div>
            <strong>Tambah anggota</strong
            ><small>Aktifkan rekening simpanan</small>
          </div>
          <ArrowRight :size="18" />
        </button>
      </aside>
    </section>

    <section class="dashboard-grid dashboard-grid--bottom">
      <article class="panel">
        <header class="panel__header">
          <div>
            <p class="eyebrow">Aktivitas</p>
            <h2>Transaksi terbaru</h2>
          </div>
          <button
            class="text-button"
            type="button"
            @click="router.push('/cash-ledger')"
          >
            Lihat buku kas <ArrowRight :size="16" />
          </button>
        </header>
        <div class="activity-list">
          <div
            v-for="transaction in transactions.slice(0, 4)"
            :key="transaction.id"
            class="activity-row"
          >
            <span
              :class="[
                'activity-row__icon',
                transaction.direction === 'Masuk' ? 'is-in' : 'is-out',
              ]"
              ><ArrowDownLeft
                v-if="transaction.direction === 'Masuk'"
                :size="18" /><ArrowUpRight v-else :size="18"
            /></span>
            <div class="activity-row__main">
              <strong>{{ transaction.description }}</strong
              ><span
                >{{ transaction.memberName }} · {{ transaction.time }}</span
              >
            </div>
            <div class="activity-row__amount">
              <strong
                :class="
                  transaction.direction === 'Masuk' ? 'amount-in' : 'amount-out'
                "
                >{{ transaction.direction === "Masuk" ? "+" : "-"
                }}{{ formatCurrency(transaction.amount) }}</strong
              ><span>{{ transaction.reference }}</span>
            </div>
          </div>
        </div>
      </article>
      <aside class="panel review-panel">
        <header class="panel__header">
          <div>
            <p class="eyebrow">Kontrol</p>
            <h2>Perlu perhatian</h2>
          </div>
          <span class="count-badge">6</span>
        </header>
        <button class="review-item" @click="router.push('/reconciliation')">
          <span class="review-item__icon"><TriangleAlert :size="19" /></span>
          <div>
            <strong>Selisih rekonsiliasi</strong
            ><small>September · Rp42.427.177</small>
          </div>
          <ArrowRight :size="17" />
        </button>
        <button class="review-item" @click="router.push('/migration')">
          <span class="review-item__icon review-item__icon--blue"
            ><Users :size="19"
          /></span>
          <div>
            <strong>Data migrasi ambigu</strong
            ><small>4 anggota perlu dipadankan</small>
          </div>
          <ArrowRight :size="17" />
        </button>
        <button class="review-item" @click="router.push('/loans')">
          <span class="review-item__icon review-item__icon--violet"
            ><Landmark :size="19"
          /></span>
          <div>
            <strong>Plafond belum valid</strong
            ><small>2 kontrak perlu dilengkapi</small>
          </div>
          <ArrowRight :size="17" />
        </button>
      </aside>
    </section>
  </div>
</template>
