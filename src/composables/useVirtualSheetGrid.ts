import { computed, nextTick, type Ref } from "vue";
import { useVirtualSheet } from "./useVirtualSheet";
import { useVirtualSheetColumns } from "./useVirtualSheetColumns";
import type { useWorkspacePageState } from "./useWorkspacePageState";

/** A monthly sheet virtualized both ways: rows, and whole month groups of
 * columns. Keyboard navigation scrolls the column into view before the row. */
export function useVirtualSheetGrid<R, G extends { period: string }>(options: {
  pageState: ReturnType<typeof useWorkspacePageState>;
  rows: Ref<readonly R[]>;
  rowKey: (row: R) => string;
  columnSpans: (index: number) => number[];
  groups: Ref<readonly G[]>;
  groupColumns: (group: G) => number;
  leadingWidths: readonly number[];
}) {
  const { pageState } = options;

  const {
    scroller,
    table,
    renderedRows,
    paddingTop,
    paddingBottom,
    navigation: rowNavigation,
  } = useVirtualSheet(
    options.rows,
    options.rowKey,
    pageState.tabId,
    pageState.active,
    options.columnSpans,
  );

  const {
    renderedGroups,
    leftColumns,
    rightColumns,
    widths: columnWidths,
    totalWidth,
    scrollToColumn,
  } = useVirtualSheetColumns(
    options.groups,
    options.groupColumns,
    options.leadingWidths,
    scroller,
    pageState.active,
  );

  const renderedColumnCount = computed(() => columnWidths.value.length);

  const navigation = computed(() => ({
    ...rowNavigation.value,
    async scrollToRow(index: number, column = 0) {
      scrollToColumn(column);
      await rowNavigation.value.scrollToRow(index);
      await nextTick();
    },
  }));

  return {
    scroller,
    table,
    renderedRows,
    paddingTop,
    paddingBottom,
    renderedGroups,
    leftColumns,
    rightColumns,
    columnWidths,
    totalWidth,
    renderedColumnCount,
    navigation,
  };
}
