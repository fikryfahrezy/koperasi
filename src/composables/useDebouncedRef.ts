import { onScopeDispose, readonly, shallowRef, watch, type Ref } from "vue";

/** Keep input responsive while delaying updates to search results. */
export function useDebouncedRef(
  source: Ref<string>,
  context: Ref<string>,
  delay = 300,
) {
  const debounced = shallowRef(source.value);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const cancel = () => {
    clearTimeout(timer);
    timer = undefined;
  };

  watch(
    () => [source.value, context.value] as const,
    ([value, tabId], [, previousTabId]) => {
      cancel();
      if (!value.trim() || tabId !== previousTabId) {
        debounced.value = value;
        return;
      }
      timer = setTimeout(() => {
        debounced.value = value;
        timer = undefined;
      }, delay);
    },
    { flush: "sync" },
  );
  onScopeDispose(cancel);
  return readonly(debounced);
}
