<script setup lang="ts">
import { computed, ref } from "vue";
import { RouterView, useRoute } from "vue-router";
import { Menu } from "lucide-vue-next";
import Sidebar from "./components/Sidebar.vue";
import { clearRuntimeError, runtimeError } from "./runtime-error";

const route = useRoute();
const isLogin = computed(() => route.path === "/login");
const isSidebarOpen = ref(false);

const closeSidebar = () => {
  isSidebarOpen.value = false;
};

const toggleSidebar = () => {
  isSidebarOpen.value = !isSidebarOpen.value;
};
</script>

<template>
  <div class="app-frame">
    <section v-if="runtimeError" class="runtime-banner" role="alert">
      <div class="runtime-banner__content">
        <div>
          <p class="runtime-banner__eyebrow">Runtime error</p>
          <p class="runtime-banner__description">
            Unhandled app errors are mirrored here when the webview console is
            not visible.
          </p>
        </div>
        <button
          class="runtime-banner__dismiss"
          type="button"
          @click="clearRuntimeError"
        >
          Dismiss
        </button>
      </div>
      <pre class="runtime-banner__body">{{ runtimeError }}</pre>
    </section>

    <RouterView v-if="isLogin" />

    <div v-else class="dashboard-shell">
      <button
        v-if="isSidebarOpen"
        class="dashboard-shell__overlay"
        type="button"
        aria-label="Close sidebar"
        @click="closeSidebar"
      ></button>

      <div
        :class="[
          'dashboard-shell__sidebar',
          { 'dashboard-shell__sidebar--open': isSidebarOpen },
        ]"
      >
        <Sidebar @close="closeSidebar" />
      </div>

      <div class="dashboard-shell__main">
        <header class="dashboard-header">
          <div class="dashboard-header__group">
            <button
              class="dashboard-header__menu"
              type="button"
              aria-label="Toggle sidebar"
              @click="toggleSidebar"
            >
              <Menu class="dashboard-header__menu-icon" />
            </button>

            <div>
              <p class="dashboard-header__eyebrow">Koperasi</p>
              <h1 class="dashboard-header__title">Koperasi Sejahtera</h1>
            </div>
          </div>

          <div class="dashboard-header__user">Admin</div>
        </header>

        <main class="dashboard-content">
          <RouterView />
        </main>
      </div>
    </div>
  </div>
</template>
