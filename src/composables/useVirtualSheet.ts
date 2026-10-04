import {
  computed,
  nextTick,
  onActivated,
  onScopeDispose,
  ref,
  watch,
  type Ref,
} from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import type { VirtualTableNavigation } from "../directives/tableNavigation";

export const SHEET_ROW_HEIGHT = 32;

/** Native table spacers preserve sticky headers, totals and horizontal scrolling. */
export function useVirtualSheet<T>(
  rows: Ref<readonly T[]>,
  key: (row: T) => string,
  context: Ref<string>,
  active: Ref<boolean>,
  columnSpans: (index: number) => number[],
) {
  const scroller = ref<HTMLElement | null>(null);
  const table = ref<HTMLTableElement | null>(null);
  const headerHeight = ref(100);
  const footerHeight = ref(SHEET_ROW_HEIGHT);
  const virtualizer = useVirtualizer<HTMLElement, HTMLTableRowElement>(
    computed(() => ({
      count: rows.value.length,
      getScrollElement: () => scroller.value,
      estimateSize: () => SHEET_ROW_HEIGHT,
      getItemKey: (index: number) => key(rows.value[index]!),
      overscan: 5,
      enabled: active.value,
      scrollMargin: headerHeight.value,
      scrollPaddingStart: headerHeight.value,
      scrollPaddingEnd: footerHeight.value,
      // Chromium 88 does not implement scrollend.
      useScrollendEvent: false,
    })),
  );
  let observer: ResizeObserver | undefined;
  const measureChrome = () => {
    if (!active.value || !table.value) return;
    headerHeight.value = table.value.tHead?.offsetHeight ?? 0;
    footerHeight.value = table.value.tFoot?.offsetHeight ?? SHEET_ROW_HEIGHT;
  };
  watch(
    table,
    async () => {
      observer?.disconnect();
      await nextTick();
      measureChrome();
      if (typeof ResizeObserver !== "undefined" && table.value) {
        observer = new ResizeObserver(measureChrome);
        if (table.value.tHead) observer.observe(table.value.tHead);
        if (table.value.tFoot) observer.observe(table.value.tFoot);
      }
    },
    { flush: "post" },
  );
  watch(active, async () => {
    await nextTick();
    measureChrome();
  });
  onActivated(async () => {
    await nextTick();
    measureChrome();
  });
  onScopeDispose(() => observer?.disconnect());
  // Filters can shrink a report while the viewport is near the bottom.
  watch(rows, async (_, previous) => {
    const tab = context.value;
    await nextTick();
    if (context.value !== tab || !active.value || !scroller.value || !previous)
      return;
    const maxOffset = Math.max(
      0,
      headerHeight.value +
        virtualizer.value.getTotalSize() +
        footerHeight.value -
        scroller.value.clientHeight,
    );
    if (scroller.value.scrollTop > maxOffset)
      scroller.value.scrollTop = maxOffset;
  });
  const items = computed(() => virtualizer.value.getVirtualItems());
  const renderedRows = computed(() =>
    items.value.map((item) => ({
      index: item.index,
      row: rows.value[item.index]!,
    })),
  );
  const paddingTop = computed(() =>
    Math.max(
      0,
      (items.value[0]?.start ?? headerHeight.value) - headerHeight.value,
    ),
  );
  const paddingBottom = computed(() =>
    Math.max(
      0,
      virtualizer.value.getTotalSize() -
        ((items.value[items.value.length - 1]?.end ?? headerHeight.value) -
          headerHeight.value),
    ),
  );
  const navigation = computed<VirtualTableNavigation>(() => ({
    count: rows.value.length + 1, // Include the always-mounted grand total row.
    context: context.value,
    rows: rows.value,
    columnSpans,
    async scrollToRow(index) {
      if (index < rows.value.length)
        virtualizer.value.scrollToIndex(index, {
          align: "auto",
          behavior: "auto",
        });
      else
        virtualizer.value.scrollToOffset(
          headerHeight.value + virtualizer.value.getTotalSize(),
          { behavior: "auto" },
        );
      await nextTick();
    },
  }));
  return {
    scroller,
    table,
    renderedRows,
    paddingTop,
    paddingBottom,
    navigation,
  };
}
