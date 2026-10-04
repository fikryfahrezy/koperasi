import { computed, nextTick, watch, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";

/** Virtualize whole month groups so native rowspan/colspan headers stay intact. */
export function useVirtualSheetColumns<T extends { period: string }>(
  groups: Ref<readonly T[]>,
  groupColumns: (group: T) => number,
  leadingWidths: readonly number[],
  scroller: Ref<HTMLElement | null>,
  active: Ref<boolean>,
) {
  const leadingWidth = leadingWidths.reduce((sum, width) => sum + width, 0);
  const groupWidth = 110;
  const virtualizer = useVirtualizer<HTMLElement, HTMLTableCellElement>(
    computed(() => ({
      horizontal: true,
      count: groups.value.length,
      getScrollElement: () => scroller.value,
      estimateSize: (index: number) =>
        groupColumns(groups.value[index]!) * groupWidth,
      getItemKey: (index: number) => groups.value[index]!.period,
      scrollMargin: leadingWidth,
      overscan: 0,
      enabled: active.value,
      useScrollendEvent: false,
    })),
  );
  const items = computed(() => virtualizer.value.getVirtualItems());
  const renderedGroups = computed(() =>
    items.value.map((item) => groups.value[item.index]!),
  );
  const leftColumns = computed(() =>
    groups.value
      .slice(0, items.value[0]?.index ?? 0)
      .reduce((count, group) => count + groupColumns(group), 0),
  );
  const rightColumns = computed(() =>
    groups.value
      .slice((items.value[items.value.length - 1]?.index ?? -1) + 1)
      .reduce((count, group) => count + groupColumns(group), 0),
  );
  const widths = computed(() => [
    ...leadingWidths,
    ...(leftColumns.value ? [leftColumns.value * groupWidth] : []),
    ...renderedGroups.value.flatMap((group) =>
      Array<number>(groupColumns(group)).fill(groupWidth),
    ),
    ...(rightColumns.value ? [rightColumns.value * groupWidth] : []),
  ]);
  const fullWidths = computed(() => [
    ...leadingWidths,
    ...groups.value.flatMap((group) =>
      Array<number>(groupColumns(group)).fill(groupWidth),
    ),
  ]);
  const totalWidth = computed(() =>
    fullWidths.value.reduce((sum, width) => sum + width, 0),
  );
  watch(groups, async () => {
    await nextTick();
    if (active.value && scroller.value) {
      const max = Math.max(0, totalWidth.value - scroller.value.clientWidth);
      if (scroller.value.scrollLeft > max) scroller.value.scrollLeft = max;
    }
  });
  function scrollToColumn(column: number) {
    const element = scroller.value;
    // The first two columns are sticky and remain visible at every offset.
    if (!element || column < 2) return;
    const start = fullWidths.value
      .slice(0, column)
      .reduce((sum, width) => sum + width, 0);
    const end = start + (fullWidths.value[column] ?? 0);
    const pinnedWidth = leadingWidths[0]! + leadingWidths[1]!;
    const previousOffset = element.scrollLeft;
    if (start < element.scrollLeft + pinnedWidth)
      element.scrollLeft = Math.max(0, start - pinnedWidth);
    else if (end > element.scrollLeft + element.clientWidth)
      element.scrollLeft = end - element.clientWidth;
    // Native scroll events arrive later; update the range before focusing a cell.
    if (element.scrollLeft !== previousOffset)
      element.dispatchEvent(new Event("scroll"));
  }
  return {
    renderedGroups,
    leftColumns,
    rightColumns,
    widths,
    totalWidth,
    scrollToColumn,
  };
}
