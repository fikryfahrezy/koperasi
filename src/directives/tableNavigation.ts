import type { ObjectDirective } from "vue";

type CellPosition = { cell: HTMLTableCellElement; column: number };
type TableState = { selected: HTMLTableCellElement | null; column: number };
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
      return Array.from(row.cells).map((cell) => {
        const position = { cell, column };
        column += cell.colSpan;
        return position;
      });
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
  cell.focus({ preventScroll: true });
  scrollTableCellIntoView(table, cell);
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
  const visible = Array.from(tables.keys()).filter(
    (table) => table.isConnected && table.getClientRects().length > 0,
  );
  const focusedTable =
    target instanceof Element ? target.closest("table") : null;
  const table =
    visible.find((candidate) => candidate === focusedTable) ??
    visible.find((candidate) => candidate === activeTable) ??
    visible[0];
  if (!table) return;
  const grid = rows(table);
  const state = tables.get(table)!;
  const rowIndex = grid.findIndex((row) =>
    row.some(({ cell }) => cell === state.selected),
  );
  if (!grid.length) return;
  event.preventDefault();
  if (rowIndex < 0 || !(target instanceof Node) || !table.contains(target)) {
    const cell = rowIndex < 0 ? grid[0]![0]!.cell : state.selected!;
    if (rowIndex < 0) state.column = 0;
    select(table, cell, true);
    return;
  }
  const row = grid[rowIndex]!;
  const cellIndex = row.findIndex(({ cell }) => cell === state.selected);
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
    select(table, next.cell, true);
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
    select(table, next.cell, true);
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
  const position = rows(table)
    .flat()
    .find((position) => position.cell === cell);
  if (!position) return;
  if (event.type === "focusin" && tables.get(table)!.selected === cell) return;
  tables.get(table)!.column = position.column;
  select(
    table,
    cell,
    event.type === "click" &&
      !target.closest("a, button, " + nativeArrowControl),
  );
}

function sync(table: HTMLTableElement) {
  const state = tables.get(table)!;
  const cells = rows(table).flat();
  if (!cells.some(({ cell }) => cell === state.selected)) {
    state.selected = cells[0]?.cell ?? null;
    state.column = 0;
  }
  for (const { cell } of cells)
    cell.tabIndex = cell === state.selected ? 0 : -1;
}

export const tableNavigation: ObjectDirective<HTMLTableElement> = {
  mounted(table) {
    if (!tables.size) document.addEventListener("keydown", onKeydown);
    tables.set(table, { selected: null, column: 0 });
    table.addEventListener("click", onCellFocus);
    table.addEventListener("focusin", onCellFocus);
    sync(table);
  },
  updated: sync,
  unmounted(table) {
    table.removeEventListener("click", onCellFocus);
    table.removeEventListener("focusin", onCellFocus);
    tables.delete(table);
    if (activeTable === table) activeTable = null;
    if (!tables.size) document.removeEventListener("keydown", onKeydown);
  },
};
