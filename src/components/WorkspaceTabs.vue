<script setup lang="ts">
import { watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { Plus, X } from "lucide-vue-next";
import {
  activeWorkspaceTabId,
  openBlankWorkspaceTab,
  syncActiveWorkspaceTab,
  workspaceTabs,
  type WorkspaceTab,
} from "../workspace-tabs";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();

watch(
  () => route.fullPath,
  () => {
    syncActiveWorkspaceTab(
      route.fullPath,
      String(route.meta.titleKey ?? "common.page"),
    );
  },
  { immediate: true },
);

function activate(tab: WorkspaceTab) {
  activeWorkspaceTabId.value = tab.id;
  if (tab.fullPath !== route.fullPath) router.push(tab.fullPath);
}

function closeTab(tab: WorkspaceTab) {
  const index = workspaceTabs.value.findIndex((item) => item.id === tab.id);
  const wasActive = tab.id === activeWorkspaceTabId.value;
  workspaceTabs.value.splice(index, 1);

  if (workspaceTabs.value.length === 0) {
    openBlankWorkspaceTab(router);
    return;
  }

  if (wasActive) {
    const nextTab =
      workspaceTabs.value[Math.min(index, workspaceTabs.value.length - 1)];
    activeWorkspaceTabId.value = nextTab.id;
    router.push(nextTab.fullPath);
  }
}
</script>

<template>
  <nav class="workspace-tabs" :aria-label="t('workspaceTabs.openPages')">
    <div class="workspace-tabs__list" role="tablist">
      <div
        v-for="tab in workspaceTabs"
        :key="tab.id"
        :class="[
          'workspace-tab',
          { 'workspace-tab--active': tab.id === activeWorkspaceTabId },
        ]"
        role="tab"
        tabindex="0"
        :aria-selected="tab.id === activeWorkspaceTabId"
        :title="t(tab.titleKey)"
        @click="activate(tab)"
        @keydown.enter="activate(tab)"
        @keydown.space.prevent="activate(tab)"
      >
        <span>{{ t(tab.titleKey) }}</span>
        <button
          class="workspace-tab__close"
          :aria-label="t('workspaceTabs.closeTab', { title: t(tab.titleKey) })"
          @click.stop="closeTab(tab)"
        >
          <X :size="13" />
        </button>
      </div>
      <button
        class="workspace-tabs__new"
        :aria-label="t('workspaceTabs.openNewTab')"
        :title="t('workspaceTabs.newTab')"
        @click="openBlankWorkspaceTab(router)"
      >
        <Plus :size="16" />
      </button>
    </div>
  </nav>
</template>
