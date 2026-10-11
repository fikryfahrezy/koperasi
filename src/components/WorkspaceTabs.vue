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

<style scoped>
.workspace-tabs__list {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: start;
  gap: 2px;
  padding: 5px var(--shell-gutter) 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: thin;
  scrollbar-color: #aebbb6 #e5ebe8;
}
.workspace-tabs__list::-webkit-scrollbar {
  display: block;
  height: 7px;
}
.workspace-tabs__list::-webkit-scrollbar-track {
  background: #e5ebe8;
}
.workspace-tabs__list::-webkit-scrollbar-thumb {
  border: 1px solid #e5ebe8;
  background: #aebbb6;
}
.workspace-tabs__list::-webkit-scrollbar-thumb:hover {
  background: #84948e;
}
.workspace-tab {
  flex: 0 1 190px;
  min-width: 118px;
  max-width: 210px;
  height: 34px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 8px 0 13px;
  border: 1px solid transparent;
  border-bottom: 0;
  color: #65726e;
  background: transparent;
  font-size: 11.5px;
  font-weight: 600;
  text-align: left;
}
.workspace-tab:hover {
  color: var(--ink);
  background: rgba(255, 255, 255, 0.65);
}
.workspace-tab--active {
  border-color: var(--line);
  color: var(--green);
  background: #fff;
}
.workspace-tab--active::before {
  content: "";
  width: 6px;
  height: 6px;
  flex: 0 0 6px;
  border-radius: 50%;
  background: var(--green);
}
.workspace-tab > span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.workspace-tab__close {
  width: 20px;
  height: 20px;
  flex: 0 0 20px;
  display: grid;
  place-items: center;
  padding: 0;
  color: var(--soft);
  background: transparent;
}
.workspace-tabs__new {
  width: 30px;
  height: 30px;
  flex: 0 0 30px;
  display: grid;
  place-items: center;
  margin: 2px 4px 0 2px;
  padding: 0;
  color: var(--muted);
  background: transparent;
}
.workspace-tabs__new:hover {
  color: var(--green);
  background: #e4ebe7;
}
@media (max-width: 780px) {
  .workspace-tabs__list {
    padding-inline: var(--shell-gutter);
  }
  .workspace-tab {
    min-width: 108px;
  }
}
</style>
