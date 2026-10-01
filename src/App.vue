<script setup lang="ts">
import { computed, onMounted } from "vue";
import { RouterView } from "vue-router";
import { Building2, CalendarDays } from "lucide-vue-next";
import logo from "./assets/logo.png";
import AppMenu from "./components/AppMenu.vue";
import ToastHost from "./components/ToastHost.vue";
import UiSelect from "./components/UiSelect.vue";
import UpdateChecker from "./components/UpdateChecker.vue";
import WorkspaceTabs from "./components/WorkspaceTabs.vue";
import { clearRuntimeError, runtimeError } from "./runtime-error";
import { useKoperasiStore } from "./store/koperasi";
import { activeWorkspaceTabId } from "./workspace-tabs";

const {
  initialize,
  loading,
  backendError,
  selectedYear,
  yearOptions,
  companies,
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

onMounted(initialize);
</script>

<template>
  <div class="app-frame">
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
          <RouterView v-else v-slot="{ Component }">
            <KeepAlive>
              <component
                :is="Component"
                :key="`${activeWorkspaceTabId}:${$route.path}`"
              />
            </KeepAlive>
          </RouterView>
        </main>
        <AppMenu />
      </div>
    </div>
    <ToastHost />
  </div>
</template>
