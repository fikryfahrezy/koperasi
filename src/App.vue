<script setup lang="ts">
import { onMounted, ref } from "vue";
import { RouterView, useRouter } from "vue-router";
import { Bell, CalendarDays, ChevronDown, Menu, Search } from "lucide-vue-next";
import Sidebar from "./components/Sidebar.vue";
import ToastHost from "./components/ToastHost.vue";
import UpdateChecker from "./components/UpdateChecker.vue";
import { clearRuntimeError, runtimeError } from "./runtime-error";
import { useKoperasiStore } from "./store/koperasi";

const router = useRouter();
const mobileOpen = ref(false);
const collapsed = ref(false);
const search = ref("");
const { initialize, loading, backendError } = useKoperasiStore();

onMounted(initialize);
function runSearch() {
  if (search.value.trim())
    router.push({ path: "/members", query: { q: search.value.trim() } });
}
</script>

<template>
  <div class="app-frame">
    <section v-if="runtimeError" class="runtime-banner" role="alert">
      <div>
        <strong>Terjadi kesalahan aplikasi</strong>
        <p>{{ runtimeError }}</p>
      </div>
      <button @click="clearRuntimeError">Tutup</button>
    </section>
    <div :class="['app-shell', { 'app-shell--collapsed': collapsed }]">
      <button
        v-if="mobileOpen"
        class="sidebar-overlay"
        aria-label="Tutup menu"
        @click="mobileOpen = false"
      ></button>
      <div :class="['app-shell__sidebar', { 'is-open': mobileOpen }]">
        <Sidebar
          :collapsed="collapsed"
          @close="mobileOpen = false"
          @toggle="collapsed = !collapsed"
        />
      </div>
      <div class="app-shell__body">
        <header class="topbar">
          <button
            class="topbar__menu icon-button"
            aria-label="Buka menu"
            @click="mobileOpen = true"
          >
            <Menu :size="21" />
          </button>
          <form class="global-search" @submit.prevent="runSearch">
            <Search :size="18" /><input
              v-model="search"
              placeholder="Cari anggota, transaksi, pinjaman..."
            /><kbd>⌘ K</kbd>
          </form>
          <div class="topbar__tools">
            <button class="period-control">
              <CalendarDays :size="17" /><span>Sep 2026</span
              ><ChevronDown :size="15" /></button
            ><button
              class="icon-button notification-button"
              aria-label="Notifikasi"
            >
              <Bell :size="19" /><i></i></button
            ><UpdateChecker />
          </div>
        </header>
        <main class="app-content">
          <section v-if="loading" class="backend-state panel">
            <span class="backend-state__spinner"></span>
            <strong>Menyiapkan database koperasi…</strong>
            <p>Memuat anggota, pinjaman, simpanan, dan buku kas 2026.</p>
          </section>
          <section v-else-if="backendError" class="backend-state panel">
            <strong>Backend desktop tidak tersambung</strong>
            <p>{{ backendError }}</p>
            <button class="button button--primary" @click="initialize">
              Coba lagi
            </button>
          </section>
          <RouterView v-else />
        </main>
      </div>
    </div>
    <ToastHost />
  </div>
</template>
