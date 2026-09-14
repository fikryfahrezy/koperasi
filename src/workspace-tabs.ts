import { ref } from "vue";
import type { Router } from "vue-router";

export type WorkspaceTab = {
  id: string;
  fullPath: string;
  titleKey: string;
};

let tabSequence = 0;
const makeId = () => `tab-${Date.now()}-${tabSequence++}`;
const blankTab = (): WorkspaceTab => ({
  id: makeId(),
  fullPath: "/new-tab",
  titleKey: "workspaceTabs.newTab",
});

export const workspaceTabs = ref<WorkspaceTab[]>([blankTab()]);
export const activeWorkspaceTabId = ref(workspaceTabs.value[0].id);

export function activeWorkspaceTab() {
  return workspaceTabs.value.find(
    (tab) => tab.id === activeWorkspaceTabId.value,
  );
}

export function syncActiveWorkspaceTab(fullPath: string, titleKey: string) {
  const tab = activeWorkspaceTab();
  if (!tab) return;
  tab.fullPath = fullPath;
  tab.titleKey = titleKey;
}

export function navigateActiveWorkspaceTab(path: string, router: Router) {
  const resolved = router.resolve(path);
  syncActiveWorkspaceTab(
    resolved.fullPath,
    String(resolved.meta.titleKey ?? "common.page"),
  );
}

export function openBlankWorkspaceTab(router: Router) {
  const tab = blankTab();
  workspaceTabs.value.push(tab);
  activeWorkspaceTabId.value = tab.id;
  router.push(tab.fullPath);
}
