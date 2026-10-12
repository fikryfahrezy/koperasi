import type { ObjectDirective } from "vue";

type HeaderState = { destroy: () => void };
const headers = new WeakMap<HTMLTableElement, HeaderState>();

/** Chromium 88 supports sticky cells, but not sticky table sections. */
export const stickyHeader: ObjectDirective<HTMLTableElement> = {
  mounted(table) {
    const head = table.tHead;
    if (!head) return;
    let frame: number | null = null;
    const observedRows = new Set<HTMLTableRowElement>();
    const measure = () => {
      frame = null;
      // Cached workspace pages can be hidden; retain their last valid offsets.
      if (!head.getClientRects().length || !head.offsetHeight) return;
      const rows = Array.from(head.rows);
      for (const row of observedRows) {
        if (!rows.includes(row)) {
          resize?.unobserve(row);
          observedRows.delete(row);
        }
      }
      let top = 0;
      for (const row of rows) {
        if (!observedRows.has(row)) {
          observedRows.add(row);
          resize?.observe(row);
        }
        const offset = `${top}px`;
        for (const cell of Array.from(row.cells)) {
          if (cell.style.top !== offset) cell.style.top = offset;
        }
        // Row heights also account for merged cells, wrapping, and font changes.
        top += row.getBoundingClientRect().height;
      }
    };
    const schedule = () => {
      if (frame === null) frame = requestAnimationFrame(measure);
    };
    const resize =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(schedule)
        : undefined;
    resize?.observe(head);
    const mutations = new MutationObserver(schedule);
    mutations.observe(head, {
      childList: true,
      subtree: true,
      characterData: true,
    });
    window.addEventListener("resize", schedule);
    measure();
    headers.set(table, {
      destroy() {
        if (frame !== null) cancelAnimationFrame(frame);
        resize?.disconnect();
        mutations.disconnect();
        window.removeEventListener("resize", schedule);
      },
    });
  },
  unmounted(table) {
    headers.get(table)?.destroy();
    headers.delete(table);
  },
};
