import assert from "node:assert/strict";
import { test } from "node:test";
import {
  scrollTableCellIntoView,
  tableNavigation,
} from "../src/directives/tableNavigation.ts";

function fixture({
  cellTop = 450,
  cellBottom = 480,
  footerTop = 450,
  inFooter = false,
} = {}) {
  let scroll;
  const scroller = {
    // The border box includes a 15px horizontal scrollbar and 1px borders.
    getBoundingClientRect: () => ({ top: 100, bottom: 501, left: 0 }),
    clientTop: 1,
    clientHeight: 384,
    scrollBy: (value) => {
      scroll = value;
    },
  };
  const table = {
    closest: () => scroller,
    tHead: { getBoundingClientRect: () => ({ bottom: 200 }) },
    tFoot: {
      // Before reaching the bottom, the section is below the viewport.
      getBoundingClientRect: () => ({ top: 1000 }),
      querySelectorAll: () => [
        {
          getBoundingClientRect: () => ({
            top: footerTop,
            bottom: footerTop + 35,
          }),
        },
      ],
    },
  };
  const cell = {
    scrollIntoView() {},
    getBoundingClientRect: () => ({
      top: cellTop,
      bottom: cellBottom,
      left: 20,
    }),
    classList: { contains: () => false },
    parentElement: {
      children: [],
      parentElement: { tagName: inFooter ? "TFOOT" : "TBODY" },
    },
  };
  scrollTableCellIntoView(table, cell);
  return scroll;
}

test("reveals a bottom row covered by sticky totals even when tfoot is outside the viewport", () => {
  assert.equal(fixture().top, 30);
});

test("does not scroll a row already fully visible above the totals", () => {
  assert.equal(fixture({ cellTop: 415, cellBottom: 450 }).top, 0);
});

test("excludes the horizontal scrollbar when revealing a footer cell", () => {
  assert.equal(
    fixture({ cellTop: 470, cellBottom: 500, inFooter: true }).top,
    15,
  );
});

test("ignores a footer outside the viewport and reveals the row above the scrollbar", () => {
  assert.equal(
    fixture({ cellTop: 460, cellBottom: 490, footerTop: 1000 }).top,
    5,
  );
});

test("keeps rows below the sticky header when moving upward", () => {
  assert.equal(fixture({ cellTop: 185, cellBottom: 215 }).top, -15);
});

function navigationFixture(t, spans, virtual) {
  const listeners = new Map();
  const frames = new Map();
  const counts = { scans: 0, focuses: 0, scrolls: 0, visibility: 0 };
  let frameId = 0;
  class Element {
    closest() {
      return null;
    }
  }
  const document = {
    activeElement: new Element(),
    querySelector: () => null,
    addEventListener: (name, listener) => listeners.set(name, listener),
    removeEventListener: (name) => listeners.delete(name),
  };
  const globals = {
    Element,
    Node: Element,
    document,
    requestAnimationFrame(callback) {
      frames.set(++frameId, callback);
      return frameId;
    },
    cancelAnimationFrame: (id) => frames.delete(id),
  };
  const originals = Object.fromEntries(
    Object.keys(globals).map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key),
    ]),
  );
  Object.assign(globalThis, globals);
  const tableListeners = new Map();
  const table = new Element();
  let mounted = true;
  const body = { tagName: "TBODY" };
  const grid = spans.map((rowSpans, index) => {
    const row = {
      parentElement: body,
      dataset: { virtualRow: String(index) },
      querySelector: () => null,
      cells: [],
    };
    row.cells = rowSpans.map((colSpan) =>
      Object.assign(new Element(), {
        colSpan,
        tabIndex: -1,
        parentElement: row,
        classList: { contains: () => false },
        focus() {
          counts.focuses++;
          document.activeElement = this;
          tableListeners.get("focusin")({
            target: this,
            currentTarget: table,
            type: "focusin",
          });
        },
        scrollIntoView() {
          counts.scrolls++;
        },
      }),
    );
    // Resolve each cell through its own closest(), like the browser DOM.
    for (const cell of row.cells) {
      cell.closest = (selector) =>
        selector === "table" ? table : selector === "td" ? cell : null;
    }
    return row;
  });
  Object.assign(table, {
    isConnected: true,
    closest: () => null,
    contains: (target) => target.closest?.("table") === table,
    getClientRects() {
      counts.visibility++;
      return [{}];
    },
    addEventListener: (name, listener) => tableListeners.set(name, listener),
    removeEventListener: (name) => tableListeners.delete(name),
  });
  Object.defineProperty(table, "rows", {
    get() {
      counts.scans++;
      return grid;
    },
  });
  tableNavigation.mounted(table, { value: virtual });
  t.after(() => {
    if (mounted) tableNavigation.unmounted(table);
    for (const [key, descriptor] of Object.entries(originals)) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor);
      else delete globalThis[key];
    }
  });
  return {
    table,
    grid,
    counts,
    frames,
    document,
    unmount() {
      tableNavigation.unmounted(table);
      mounted = false;
    },
    press(key) {
      let prevented = false;
      listeners.get("keydown")({
        key,
        target: document.activeElement,
        preventDefault: () => {
          prevented = true;
        },
      });
      return prevented;
    },
    flush() {
      const callbacks = Array.from(frames.values());
      frames.clear();
      return Promise.all(callbacks.map((callback) => callback()));
    },
  };
}

test("arrow bursts preserve every step with one focus and scroll per frame and no table rescans", (t) => {
  const nav = navigationFixture(
    t,
    Array.from({ length: 100 }, () => Array(20).fill(1)),
  );
  nav.press("ArrowDown"); // Enter the table first.
  nav.flush();
  for (let i = 0; i < 80; i++) assert.equal(nav.press("ArrowDown"), true);
  assert.equal(nav.frames.size, 1);
  assert.equal(nav.counts.focuses, 1);
  nav.flush();
  assert.equal(nav.document.activeElement, nav.grid[80].cells[0]);
  assert.equal(nav.counts.focuses, 2);
  assert.equal(nav.counts.scrolls, 2);
  assert.equal(nav.counts.scans, 1);
  assert.equal(nav.counts.visibility, 1);
});

test("batched navigation preserves the preferred column through merged cells and stops at edges", (t) => {
  const nav = navigationFixture(t, [
    [1, 1, 1],
    [2, 1],
    [1, 1, 1],
  ]);
  nav.press("ArrowRight");
  nav.press("ArrowRight"); // Advance even before the initial focus frame.
  nav.press("ArrowDown");
  nav.press("ArrowDown");
  nav.flush();
  assert.equal(nav.document.activeElement, nav.grid[2].cells[1]);
  for (let i = 0; i < 30; i++) nav.press("ArrowDown");
  assert.equal(nav.frames.size, 0);
});

test("refreshes cached positions after table updates and cancels work on unmount", (t) => {
  const nav = navigationFixture(t, [[1], [1]]);
  nav.press("ArrowDown");
  nav.flush();
  nav.grid.shift();
  nav.document.activeElement = {};
  tableNavigation.updated(nav.table);
  nav.press("ArrowDown");
  assert.equal(nav.frames.size, 1);
  nav.flush();
  assert.equal(nav.document.activeElement, nav.grid[0].cells[0]);
  nav.press("ArrowRight"); // Re-enter when focus is outside the table.
  nav.document.activeElement = {};
  nav.press("ArrowDown");
  nav.unmount();
  assert.equal(nav.frames.size, 0);
});

test("pending navigation does not steal focus after another control receives it", (t) => {
  const nav = navigationFixture(t, [[1, 1]]);
  nav.press("ArrowRight");
  const control = {};
  nav.document.activeElement = control;
  nav.flush();
  assert.equal(nav.document.activeElement, control);
  assert.equal(nav.counts.focuses, 0);
});

test("virtual arrow bursts cross unmounted rows and preserve columns through merged rows", async (t) => {
  let nav;
  let cells;
  const virtual = {
    count: 1001,
    context: "tab-1",
    rows: [],
    columnSpans: (index) => (index === 80 ? [2, 1] : [1, 1, 1]),
    async scrollToRow(index) {
      const row = nav.grid[0];
      cells ??= [...row.cells];
      row.cells = virtual.columnSpans(index).map((span, cellIndex) => {
        cells[cellIndex].colSpan = span;
        return cells[cellIndex];
      });
      row.dataset.virtualRow = String(index);
      nav.grid.splice(0, nav.grid.length, row);
      tableNavigation.updated(nav.table, { value: virtual });
    },
  };
  nav = navigationFixture(t, [[1, 1, 1]], virtual);
  nav.press("ArrowDown");
  await nav.flush();
  nav.press("ArrowRight");
  for (let i = 0; i < 80; i++) nav.press("ArrowDown");
  assert.equal(nav.frames.size, 1);
  await nav.flush();
  assert.equal(
    nav.document.activeElement.parentElement.dataset.virtualRow,
    "80",
  );
  assert.equal(nav.document.activeElement, nav.grid[0].cells[0]);
  nav.press("ArrowDown");
  await nav.flush();
  assert.equal(
    nav.document.activeElement.parentElement.dataset.virtualRow,
    "81",
  );
  assert.equal(nav.counts.focuses, 3);
});

test("virtual focus waits for rendering and is cancelled when the tab changes", async (t) => {
  let resolveScroll;
  const virtual = {
    count: 100,
    context: "first",
    rows: [],
    columnSpans: () => [1],
    scrollToRow: () =>
      new Promise((resolve) => {
        resolveScroll = resolve;
      }),
  };
  const nav = navigationFixture(t, [[1]], virtual);
  nav.press("ArrowDown");
  const pending = nav.flush();
  tableNavigation.updated(nav.table, {
    value: { ...virtual, context: "second" },
  });
  resolveScroll();
  await pending;
  assert.equal(nav.counts.focuses, 0);
});

test("horizontal virtual navigation skips gap cells and focuses the logical destination", async (t) => {
  let nav;
  let requestedColumn;
  const virtual = {
    count: 1,
    context: "tab",
    rows: [],
    columnSpans: () => Array(10).fill(1),
    async scrollToRow(_, column) {
      requestedColumn = column;
      nav.grid[0].cells[1].dataset.virtualSpan = String(column - 1);
      tableNavigation.updated(nav.table, { value: virtual });
    },
  };
  nav = navigationFixture(t, [[1, 1, 1]], virtual);
  const cells = nav.grid[0].cells;
  cells[0].dataset = {};
  cells[1].dataset = { virtualGap: "", virtualSpan: "8" };
  cells[2].dataset = {};
  tableNavigation.updated(nav.table, { value: virtual });
  assert.equal(cells[2].dataset.virtualColumn, "9");
  cells[2].focus();
  nav.press("ArrowLeft");
  await nav.flush();
  assert.equal(requestedColumn, 8);
  assert.equal(cells[2].dataset.virtualColumn, "8");
  assert.equal(nav.document.activeElement, cells[2]);
  assert.equal(
    cells[1].tabIndex,
    -1,
    "spacers never become navigation targets",
  );
});

test("virtual reports have one Tab entry and arrows advance from the focused entry", async (t) => {
  const virtual = {
    count: 3,
    context: "first",
    rows: [],
    columnSpans: () => [1, 1],
    async scrollToRow() {},
  };
  const nav = navigationFixture(
    t,
    [
      [1, 1],
      [1, 1],
      [1, 1],
    ],
    virtual,
  );
  const tabbable = () =>
    nav.grid.flatMap((row) => row.cells).filter((cell) => cell.tabIndex === 0);
  assert.deepEqual(tabbable(), [nav.grid[0].cells[0]]);
  nav.grid[0].cells[0].focus();
  nav.press("ArrowDown");
  await nav.flush();
  assert.equal(nav.document.activeElement, nav.grid[1].cells[0]);
  assert.deepEqual(tabbable(), [nav.grid[1].cells[0]]);

  // Scrolling unmounts the selection; the next mounted row becomes the entry.
  nav.grid.splice(0, 2);
  nav.document.activeElement = {};
  tableNavigation.updated(nav.table, { value: virtual });
  assert.deepEqual(tabbable(), [nav.grid[0].cells[0]]);
  nav.grid[0].cells[0].focus();
  nav.press("ArrowRight");
  await nav.flush();
  assert.equal(nav.document.activeElement, nav.grid[0].cells[1]);

  // Filters and tab changes must also restore a single entry point.
  tableNavigation.updated(nav.table, { value: { ...virtual, rows: [{}] } });
  assert.deepEqual(tabbable(), [nav.grid[0].cells[0]]);
  tableNavigation.updated(nav.table, {
    value: { ...virtual, context: "second" },
  });
  assert.deepEqual(tabbable(), [nav.grid[0].cells[0]]);
});

test("the Tab entry moves from initial totals to data rows when they finish mounting", (t) => {
  const virtual = {
    count: 2,
    context: "first",
    rows: [],
    columnSpans: () => [1],
    async scrollToRow() {},
  };
  const nav = navigationFixture(t, [[1], [1]], virtual);
  const firstRow = nav.grid.shift();
  tableNavigation.updated(nav.table, { value: virtual });
  assert.equal(nav.grid[0].cells[0].tabIndex, 0);
  nav.grid.unshift(firstRow);
  tableNavigation.updated(nav.table, { value: virtual });
  assert.equal(firstRow.cells[0].tabIndex, 0);
  assert.equal(nav.grid[1].cells[0].tabIndex, -1);
});
