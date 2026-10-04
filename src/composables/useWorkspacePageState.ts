import { computed, ref, watch, type Ref } from "vue";
import {
  activeWorkspaceTab,
  activeWorkspaceTabId,
  workspaceTabs,
} from "../workspace-tabs";

/** Store only UI state per tab; page components and rendered tables are shared. */
export function useWorkspacePageState(path: string) {
  const tabId = ref(activeWorkspaceTabId.value);
  const entries = new Map<string, Map<string, Ref>>();
  watch(
    () => [activeWorkspaceTabId.value, activeWorkspaceTab()?.fullPath],
    () => {
      if (activeWorkspaceTab()?.fullPath.split(/[?#]/)[0] === path) {
        tabId.value = activeWorkspaceTabId.value;
      }
    },
    { immediate: true, flush: "sync" },
  );
  watch(
    () => workspaceTabs.value.map((tab) => tab.id),
    (ids) => {
      for (const id of entries.keys())
        if (!ids.includes(id)) entries.delete(id);
    },
  );

  function field<T>(key: string, initial: () => T) {
    const forTab = (id: string): Ref<T> => {
      if (!entries.has(id)) entries.set(id, new Map());
      const entry = entries.get(id)!;
      if (!entry.has(key)) entry.set(key, ref(initial()));
      return entry.get(key)! as Ref<T>;
    };
    return Object.assign(
      computed({
        get: () => forTab(tabId.value).value,
        set: (value: T) => {
          forTab(tabId.value).value = value;
        },
      }),
      { forTab },
    );
  }
  const active = computed(
    () =>
      activeWorkspaceTabId.value === tabId.value &&
      activeWorkspaceTab()?.fullPath.split(/[?#]/)[0] === path,
  );
  return { tabId, field, active };
}
