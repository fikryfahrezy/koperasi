<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Building2, CalendarDays } from "lucide-vue-next";
import logo from "./assets/logo.png";
import AppMenu from "./components/AppMenu.vue";
import ToastHost from "./components/ToastHost.vue";
import UiSelect from "./components/UiSelect.vue";
import UpdateChecker from "./components/UpdateChecker.vue";
import WorkspaceTabs from "./components/WorkspaceTabs.vue";
import WorkspaceTabPages from "./components/WorkspaceTabPages.vue";
import { clearRuntimeError, runtimeError } from "./runtime-error";
import { useKoperasiStore } from "./store/koperasi";
import { activeWorkspaceTab } from "./workspace-tabs";

const {
  initialize,
  loading,
  backendError,
  selectedYear,
  yearOptions,
  companies,
  configuredCompanyId,
  selectedCompanyId,
  selectCompany,
} = useKoperasiStore();

const companyOptions = computed(() =>
  companies.value.map((company) => ({
    value: company.id,
    label: company.name,
  })),
);
const reportYearOptions = computed(() =>
  yearOptions.value.map((year) => ({ value: year, label: String(year) })),
);
const route = useRoute();
const router = useRouter();
// Menu clicks switch the tab's page before the router settles, so follow the
// tab. Apply it post-flush: cached pages save scroll before the layout changes
// and restore it after.
const fitViewport = ref(false);
watch(
  () =>
    Boolean(
      router.resolve(activeWorkspaceTab()?.fullPath ?? route.fullPath).meta
        .fitViewport,
    ),
  (value) => {
    fitViewport.value = value;
  },
  { immediate: true, flush: "post" },
);

onMounted(initialize);
</script>

<template>
  <div class="app-frame" :class="{ 'app-frame--sheet': fitViewport }">
    <section v-if="runtimeError" class="runtime-banner" role="alert">
      <div>
        <strong>{{ $t("app.errorTitle") }}</strong>
        <p>{{ runtimeError }}</p>
      </div>
      <button @click="clearRuntimeError">{{ $t("common.close") }}</button>
    </section>
    <div class="app-shell">
      <div class="app-shell__body">
        <div class="app-shell__chrome">
          <header class="topbar">
            <div class="topbar__brand">
              <img :src="logo" :alt="$t('sidebar.logoAlt')" />
              <div>
                <strong>{{ $t("sidebar.cooperativeName") }}</strong>
                <span>{{ $t("sidebar.cooperativeShortName") }}</span>
              </div>
            </div>
            <div class="topbar__tools">
              <UiSelect
                v-if="!configuredCompanyId"
                class="company-control"
                :model-value="selectedCompanyId"
                :options="companyOptions"
                aria-label="Perusahaan"
                variant="toolbar"
                @update:model-value="selectCompany(String($event))"
              >
                <template #icon><Building2 :size="16" /></template>
              </UiSelect>
              <UiSelect
                :model-value="selectedYear"
                :options="reportYearOptions"
                aria-label="Tahun laporan"
                variant="toolbar"
                @update:model-value="selectedYear = Number($event)"
              >
                <template #icon><CalendarDays :size="16" /></template>
              </UiSelect>
              <UpdateChecker />
            </div>
          </header>
          <WorkspaceTabs />
        </div>
        <div class="app-stage">
          <!-- Page dialogs render here so tabs and the menu stay usable. -->
          <div id="page-overlay"></div>
          <main class="app-content">
            <section v-if="loading" class="backend-state panel">
              <span class="backend-state__spinner"></span>
              <strong>{{ $t("app.loadingTitle") }}</strong>
              <p>{{ $t("app.loadingDescription") }}</p>
            </section>
            <section v-else-if="backendError" class="backend-state panel">
              <strong>{{ $t("app.backendTitle") }}</strong>
              <p>{{ backendError }}</p>
              <button class="button button--primary" @click="initialize">
                {{ $t("common.retry") }}
              </button>
            </section>
            <WorkspaceTabPages v-else />
          </main>
        </div>
        <AppMenu />
      </div>
    </div>
    <ToastHost />
  </div>
</template>

<style scoped>
.app-frame {
  /* Chromium 88 (Windows 7) ignores dvh, so keep vh as the fallback. */
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
}
.app-shell {
  height: 100%;
}
.app-shell__body {
  min-width: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
}
.app-stage {
  position: relative;
  flex: 1 1 0%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.app-shell__chrome {
  flex-shrink: 0;
  z-index: 20;
  background: #fff;
}
.topbar {
  height: 58px;
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 0 var(--shell-gutter);
  border-bottom: 1px solid var(--line);
  background: #fff;
}
.topbar__brand {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 196px;
}
.topbar__brand img {
  width: 34px;
  height: 34px;
  object-fit: cover;
}
.topbar__brand > div {
  display: grid;
  gap: 1px;
}
.topbar__brand strong {
  color: var(--ink);
  font-size: 12px;
}
.topbar__brand span {
  color: var(--soft);
  font-size: 9.5px;
}
.topbar__tools {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 9px;
}
.company-control {
  max-width: 240px;
}
.runtime-banner {
  position: fixed;
  z-index: 300;
  left: 20px;
  right: 20px;
  bottom: 20px;
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding: 16px;
  border: 1px solid #e1aaa5;
  border-radius: 11px;
  color: #803430;
  background: #fff6f5;
}
.runtime-banner p {
  max-width: 800px;
  margin: 4px 0 0;
  font: 10px/1.4 monospace;
  white-space: pre-wrap;
}
.runtime-banner button {
  color: var(--red);
  background: transparent;
}
@media (max-width: 780px) {
  .topbar {
    height: 54px;
    padding: 0 var(--shell-gutter);
  }
  .topbar__brand {
    min-width: auto;
  }
  .topbar__brand > div {
    display: none;
  }
}
@media (max-width: 540px) {
  .topbar__brand {
    display: none;
  }
}
</style>
