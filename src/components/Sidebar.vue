<script setup lang="ts">
import { RouterLink } from "vue-router";
import logo from "../assets/logo.png";
import {
  BookOpen,
  ChevronLeft,
  CircleDollarSign,
  FileSpreadsheet,
  Gauge,
  Landmark,
  PiggyBank,
  Settings2,
  ShieldCheck,
  Users,
  X,
} from "lucide-vue-next";

defineProps<{ collapsed?: boolean }>();
const emit = defineEmits<{ close: []; toggle: [] }>();
const mainNav = [
  { to: "/dashboard", label: "Ringkasan", icon: Gauge },
  { to: "/transactions", label: "Transaksi", icon: CircleDollarSign },
  { to: "/members", label: "Anggota", icon: Users },
  { to: "/savings", label: "Simpanan", icon: PiggyBank },
  { to: "/loans", label: "Pinjaman", icon: Landmark },
  { to: "/cash-ledger", label: "Buku kas", icon: BookOpen },
];
const controlNav = [
  {
    to: "/reconciliation",
    label: "Rekonsiliasi",
    icon: ShieldCheck,
    badge: "6",
  },
  { to: "/migration", label: "Migrasi data", icon: FileSpreadsheet },
  { to: "/admin", label: "Administrasi", icon: Settings2 },
];
</script>
<template>
  <aside :class="['sidebar', { 'sidebar--collapsed': collapsed }]">
    <header class="sidebar__brand">
      <div class="sidebar__logo"><img :src="logo" alt="Logo koperasi" /></div>
      <div class="sidebar__brand-copy">
        <strong>Koperasi Pensiunan BRI Kuningan</strong>
        <span>Bina Sejahtera</span>
      </div>
      <button class="sidebar__mobile-close" @click="emit('close')">
        <X :size="20" />
      </button>
    </header>
    <nav class="sidebar__nav">
      <p>Operasional</p>
      <RouterLink
        v-for="item in mainNav"
        :key="item.to"
        :to="item.to"
        class="sidebar__link"
        active-class="sidebar__link--active"
        :aria-label="collapsed ? item.label : undefined"
        :title="collapsed ? item.label : undefined"
        @click="emit('close')"
        ><component :is="item.icon" :size="19" /><span>{{
          item.label
        }}</span></RouterLink
      >
      <p>Kontrol & sistem</p>
      <RouterLink
        v-for="item in controlNav"
        :key="item.to"
        :to="item.to"
        class="sidebar__link"
        active-class="sidebar__link--active"
        :aria-label="collapsed ? item.label : undefined"
        :title="collapsed ? item.label : undefined"
        @click="emit('close')"
        ><component :is="item.icon" :size="19" /><span>{{ item.label }}</span
        ><b v-if="item.badge">{{ item.badge }}</b></RouterLink
      >
    </nav>
    <footer class="sidebar__footer">
      <button
        class="sidebar__collapse"
        :aria-label="collapsed ? 'Perluas sidebar' : 'Perkecil sidebar'"
        :title="collapsed ? 'Perluas sidebar' : 'Perkecil sidebar'"
        @click="emit('toggle')"
      >
        <ChevronLeft :size="18" />
      </button>
    </footer>
  </aside>
</template>
