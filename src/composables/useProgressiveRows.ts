import {
  computed,
  onActivated,
  onDeactivated,
  onScopeDispose,
  ref,
  watch,
} from "vue";

/** Yield between small DOM updates so navigation and input can be processed. */
export function useProgressiveRows(
  rows: () => readonly unknown[],
  columns: () => number,
) {
  const renderedRowCount = ref(0);
  const totalRowCount = computed(() => rows().length);
  const renderingRows = computed(
    () => renderedRowCount.value < totalRowCount.value,
  );
  let active = true;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function cancel() {
    clearTimeout(timer);
    timer = undefined;
  }

  function schedule() {
    cancel();
    if (!active || !renderingRows.value) return;
    // A timer gives the page header/loading state a chance to paint first.
    timer = setTimeout(() => {
      timer = undefined;
      const batchSize = Math.max(
        1,
        Math.min(25, Math.floor(800 / Math.max(1, columns()))),
      );
      renderedRowCount.value = Math.min(
        totalRowCount.value,
        renderedRowCount.value + batchSize,
      );
      schedule();
    }, 16);
  }

  watch(
    [rows, columns],
    () => {
      renderedRowCount.value = 0;
      schedule();
    },
    { immediate: true },
  );

  onDeactivated(() => {
    active = false;
    cancel();
    // Clear the rendered rows while hidden so returning does not attach the
    // entire cached table before the loading state can appear.
    renderedRowCount.value = 0;
  });
  onActivated(() => {
    active = true;
    renderedRowCount.value = 0;
    schedule();
  });
  onScopeDispose(cancel);

  return { renderedRowCount, totalRowCount, renderingRows };
}
