import type { ObjectDirective } from "vue";
import { createClippedCellTooltip } from "./clippedCellTooltip.ts";

export type VirtualTableNavigation = {
  count: number;
  context: string;
  rows: readonly unknown[];
  columnSpans: (index: number) => number[];
  scrollToRow: (index: number, column?: number) => Promise<void>;
};

type CellPosition = { cell: HTMLTableCellElement; column: number };
type TableState = {
  selected: HTMLTableCellElement | null;
  virtual?: VirtualTableNavigation;
  logicalRow: number | null;
  revision: number;
  column: number;
  grid: CellPosition[][];
  positions: Map<HTMLTableCellElement, { row: number; cell: number }>;
  frame: number | null;
  tooltip: ReturnType<typeof createClippedCellTooltip>;
};
const tables = new Map<HTMLTableElement, TableState>();
let activeTable: HTMLTableElement | null = null;
const arrowKeys = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"]);
const nativeArrowControl =
  'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="combobox"], [role="listbox"], [role="menu"], [role="tablist"], [role="slider"], [role="spinbutton"]';

function rows(table: HTMLTableElement): CellPosition[][] {
  return Array.from(table.rows)
    .filter((row) => row.parentElement?.tagName !== "THEAD")
    .filter(
      (row) =>
        !row.querySelector(".sheet-empty") &&
        !(row.cells.length === 1 && row.cells[0]!.colSpan > 1),
    )
    .map((row) => {
      let column = 0;
      const cells: CellPosition[] = [];
      for (const cell of Array.from(row.cells)) {
        const span = Number(cell.dataset?.virtualSpan) || cell.colSpan;
        if (cell.dataset?.virtualGap === undefined) {
          cells.push({ cell, column });
          if (cell.dataset) cell.dataset.virtualColumn = String(column);
          cell.setAttribute?.("aria-colindex", String(column + 1));
        }
        column += span;
      }
      return cells;
    })
    .filter((row) => row.length > 0);
}

function select(
  table: HTMLTableElement,
  cell: HTMLTableCellElement,
  focus: boolean,
) {
  const state = tables.get(table)!;
  if (state.selected && state.selected !== cell) state.selected.tabIndex = -1;
  state.selected = cell;
  cell.tabIndex = 0;
  activeTable = table;
  if (!focus) return;
  if (state.frame !== null) return;
  const origin = document.activeElement;
  // Keep every logical arrow step, but only focus and measure the final cell
  // once per frame. Slow devices never accumulate a queue of scroll work.
  state.frame = requestAnimationFrame(() => {
    state.frame = null;
    const selected = state.selected;
    if (
      !selected ||
      !table.isConnected ||
      activeTable !== table ||
      document.activeElement !== origin
    )
      return;
    selected.focus({ preventScroll: true });
    scrollTableCellIntoView(table, selected);
  });
}

function cancelFocus(state: TableState) {
  if (state.frame !== null) cancelAnimationFrame(state.frame);
  state.frame = null;
}

export function scrollTableCellIntoView(
  table: HTMLTableElement,
  cell: HTMLTableCellElement,
) {
  cell.scrollIntoView({ block: "nearest", inline: "nearest" });

  // Account for the sheet's sticky headers, totals, and leading columns.
  const scroller = table.closest<HTMLElement>(".data-table-wrap");
  if (!scroller) return;
  const viewport = scroller.getBoundingClientRect();
  const viewportTop = viewport.top + scroller.clientTop;
  const viewportBottom = viewportTop + scroller.clientHeight;
  const rect = cell.getBoundingClientRect();
  const top = Math.max(
    viewportTop,
    table.tHead?.getBoundingClientRect().bottom ?? viewportTop,
  );
  // The footer cells are sticky; the tfoot itself remains in normal flow.
  const footerTops = Array.from(
    table.tFoot?.querySelectorAll<HTMLTableCellElement>("td") ?? [],
  )
    .map((footerCell) => footerCell.getBoundingClientRect())
    .filter(
      (footerRect) =>
        footerRect.bottom > viewportTop && footerRect.top < viewportBottom,
    )
    .map((footerRect) => footerRect.top);
  const bottom =
    cell.parentElement?.parentElement?.tagName === "TFOOT"
      ? viewportBottom
      : Math.min(viewportBottom, ...footerTops);
  const stickyCells = Array.from(cell.parentElement!.children).filter(
    (sibling) => sibling.classList.contains("sheet-sticky"),
  );
  const left = cell.classList.contains("sheet-sticky")
    ? viewport.left
    : Math.max(
        viewport.left,
        ...stickyCells.map((sibling) => sibling.getBoundingClientRect().right),
      );
  scroller.scrollBy({
    top:
      rect.top < top
        ? rect.top - top
        : rect.bottom > bottom
          ? rect.bottom - bottom
          : 0,
    left: rect.left < left ? rect.left - left : 0,
  });
}

function virtualRowIndex(cell: HTMLTableCellElement | null) {
  const value = cell?.parentElement?.dataset?.virtualRow;
  return value === undefined ? null : Number(value);
}

function queueVirtualFocus(table: HTMLTableElement, state: TableState) {
  if (state.frame !== null) return;
  const origin = document.activeElement;
  const revision = state.revision;
  state.frame = requestAnimationFrame(async () => {
    state.frame = null;
    const index = state.logicalRow;
    if (index === null || !state.virtual || revision !== state.revision) return;
    const column = state.column;
    await state.virtual.scrollToRow(index, column);
    if (
      revision !== state.revision ||
      index !== state.logicalRow ||
      column !== state.column ||
      !table.isConnected ||
      activeTable !== table ||
      (document.activeElement !== origin &&
        !(
          origin &&
          !origin.isConnected &&
          document.activeElement === document.body
        ))
    )
      return;
    const row = state.grid.find(
      (cells) => virtualRowIndex(cells[0]?.cell ?? null) === index,
    );
    const next = row?.find(
      ({ cell, column: start }) =>
        column >= start &&
        column < start + (Number(cell.dataset?.virtualSpan) || cell.colSpan),
    );
    if (!next) return;
    select(table, next.cell, false);
    next.cell.focus({ preventScroll: true });
    scrollTableCellIntoView(table, next.cell);
  });
}

function navigateVirtual(
  table: HTMLTableElement,
  state: TableState,
  event: KeyboardEvent,
) {
  const virtual = state.virtual!;
  if (!virtual.count) return;
  event.preventDefault();
  activeTable = table;
  if (state.logicalRow === null) {
    state.logicalRow = virtualRowIndex(state.grid[0]?.[0]?.cell ?? null) ?? 0;
    state.column = 0;
  } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    state.logicalRow = Math.max(
      0,
      Math.min(
        virtual.count - 1,
        state.logicalRow + (event.key === "ArrowDown" ? 1 : -1),
      ),
    );
  } else {
    const spans = virtual.columnSpans(state.logicalRow);
    const starts: number[] = [];
    let column = 0;
    for (const span of spans) {
      starts.push(column);
      column += span;
    }
    const current = starts.findIndex(
      (start, index) =>
        state.column >= start && state.column < start + spans[index]!,
    );
    state.column =
      starts[
        Math.max(
          0,
          Math.min(
            starts.length - 1,
            current + (event.key === "ArrowRight" ? 1 : -1),
          ),
        )
      ] ?? 0;
  }
  queueVirtualFocus(table, state);
}

function onKeydown(event: KeyboardEvent) {
  if (
    !arrowKeys.has(event.key) ||
    event.defaultPrevented ||
    event.isComposing ||
    event.altKey ||
    event.ctrlKey ||
    event.metaKey ||
    event.shiftKey
  )
    return;
  const target = event.target;
  if (target instanceof Element && target.closest(nativeArrowControl)) return;
  if (
    document.querySelector(
      '[aria-modal="true"], [role="listbox"], [role="menu"]',
    )
  )
    return;
  const focusedTable =
    target instanceof Element
      ? target.closest<HTMLTableElement>("table")
      : null;
  const isVisible = (candidate: HTMLTableElement) =>
    candidate.isConnected && candidate.getClientRects().length > 0;
  // A focused table is already visible. Avoid layout reads on the repeat path.
  const table =
    focusedTable && tables.has(focusedTable)
      ? focusedTable
      : activeTable && isVisible(activeTable)
        ? activeTable
        : Array.from(tables.keys()).find(isVisible);
  if (!table) return;
  const state = tables.get(table)!;
  if (state.virtual) {
    navigateVirtual(table, state, event);
    return;
  }
  const grid = state.grid;
  const position = state.selected ? state.positions.get(state.selected) : null;
  const rowIndex = position?.row ?? -1;
  if (!grid.length) return;
  event.preventDefault();
  if (
    rowIndex < 0 ||
    (state.frame === null &&
      (!(target instanceof Node) || !table.contains(target)))
  ) {
    const cell = rowIndex < 0 ? grid[0]![0]!.cell : state.selected!;
    if (rowIndex < 0) state.column = 0;
    select(table, cell, true);
    return;
  }
  const row = grid[rowIndex]!;
  const cellIndex = position!.cell;
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    const next =
      row[
        Math.max(
          0,
          Math.min(
            row.length - 1,
            cellIndex + (event.key === "ArrowLeft" ? -1 : 1),
          ),
        )
      ]!;
    state.column = next.column;
    if (next.cell !== state.selected) select(table, next.cell, true);
  } else {
    const nextRow =
      grid[
        Math.max(
          0,
          Math.min(
            grid.length - 1,
            rowIndex + (event.key === "ArrowUp" ? -1 : 1),
          ),
        )
      ]!;
    const next =
      nextRow.find(
        ({ cell, column }) =>
          state.column >= column && state.column < column + cell.colSpan,
      ) ?? nextRow[nextRow.length - 1]!;
    if (next.cell !== state.selected) select(table, next.cell, true);
  }
}

function onCellFocus(event: Event) {
  const target = event.target;
  if (!(target instanceof Element)) return;
  const table = event.currentTarget as HTMLTableElement;
  const cell =
    target instanceof Element
      ? target.closest<HTMLTableCellElement>("td")
      : null;
  if (!cell) return;
  const state = tables.get(table)!;
  state.tooltip.show(cell);
  if (
    event.type === "focusin" &&
    state.selected === cell &&
    (!state.virtual || state.logicalRow === virtualRowIndex(cell))
  )
    return;
  const position = state.positions.get(cell);
  if (!position) return;
  cancelFocus(state);
  state.logicalRow = virtualRowIndex(cell);
  state.column = state.grid[position.row]![position.cell]!.column;
  select(
    table,
    cell,
    event.type === "click" &&
      !target.closest("a, button, " + nativeArrowControl),
  );
}

function sync(table: HTMLTableElement) {
  const state = tables.get(table)!;
  state.grid = rows(table);
  state.positions.clear();
  state.grid.forEach((row, rowIndex) => {
    row.forEach(({ cell }, cellIndex) => {
      state.positions.set(cell, { row: rowIndex, cell: cellIndex });
    });
  });
  const cells = state.grid.flat();
  if (
    !state.selected ||
    !state.positions.has(state.selected) ||
    (state.virtual && state.logicalRow === null)
  ) {
    if (!state.virtual) cancelFocus(state);
    state.selected = cells[0]?.cell ?? null;
    if (!state.virtual) state.column = 0;
  }
  for (const { cell } of cells) {
    const tabIndex = cell === state.selected ? 0 : -1;
    if (cell.tabIndex !== tabIndex) cell.tabIndex = tabIndex;
  }
}

export const tableNavigation: ObjectDirective<
  HTMLTableElement,
  VirtualTableNavigation | undefined
> = {
  mounted(table, binding) {
    if (!tables.size) document.addEventListener("keydown", onKeydown);
    tables.set(table, {
      selected: null,
      virtual: binding?.value,
      logicalRow: null,
      revision: 0,
      column: 0,
      grid: [],
      positions: new Map(),
      frame: null,
      tooltip: createClippedCellTooltip(table),
    });
    table.addEventListener("click", onCellFocus);
    table.addEventListener("focusin", onCellFocus);
    sync(table);
  },
  updated(table, binding) {
    const state = tables.get(table)!;
    const value = binding?.value;
    if (
      state.virtual?.context !== value?.context ||
      state.virtual?.rows !== value?.rows
    ) {
      cancelFocus(state);
      state.revision++;
      state.logicalRow = null;
      state.selected = null;
      state.column = 0;
      state.tooltip.hide();
    }
    state.virtual = value;
    sync(table);
  },
  unmounted(table) {
    tables.get(table)!.tooltip.destroy();
    tables.get(table)!.revision++;
    cancelFocus(tables.get(table)!);
    table.removeEventListener("click", onCellFocus);
    table.removeEventListener("focusin", onCellFocus);
    tables.delete(table);
    if (activeTable === table) activeTable = null;
    if (!tables.size) document.removeEventListener("keydown", onKeydown);
  },
};
