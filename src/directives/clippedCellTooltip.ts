let sequence = 0;

/** One tooltip per table, measured only when a cell is hovered or focused. */
export function createClippedCellTooltip(table: HTMLTableElement) {
  let owner: HTMLTableCellElement | null = null;
  let tooltip: HTMLDivElement | null = null;
  const scroller = table.closest<HTMLElement>(".data-table-wrap");

  function hide() {
    if (owner && tooltip) {
      const ids = (owner.getAttribute("aria-describedby") ?? "")
        .split(/\s+/)
        .filter((id) => id && id !== tooltip!.id);
      if (ids.length) owner.setAttribute("aria-describedby", ids.join(" "));
      else owner.removeAttribute("aria-describedby");
    }
    tooltip?.remove();
    tooltip = null;
    owner = null;
  }

  function show(cell: HTMLTableCellElement) {
    hide();
    const text = cell.textContent?.trim();
    if (!text || !(cell.scrollWidth > cell.clientWidth)) return;
    const rect = cell.getBoundingClientRect();
    const viewport = scroller?.getBoundingClientRect();
    if (
      viewport &&
      (rect.bottom <= viewport.top ||
        rect.top >= viewport.bottom ||
        rect.right <= viewport.left ||
        rect.left >= viewport.right)
    )
      return;
    owner = cell;
    tooltip = document.createElement("div");
    tooltip.id = `sheet-cell-tooltip-${++sequence}`;
    tooltip.className = "sheet-cell-tooltip";
    tooltip.setAttribute("role", "tooltip");
    tooltip.textContent = text;
    document.body.appendChild(tooltip);
    const previous = cell.getAttribute("aria-describedby");
    cell.setAttribute(
      "aria-describedby",
      [previous, tooltip.id].filter(Boolean).join(" "),
    );
    const size = tooltip.getBoundingClientRect();
    tooltip.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - size.width - 8))}px`;
    tooltip.style.top = `${Math.max(
      8,
      rect.bottom + size.height + 6 <= window.innerHeight - 8
        ? rect.bottom + 6
        : rect.top - size.height - 6,
    )}px`;
  }

  function hover(event: Event) {
    const cell = (event.target as Element).closest<HTMLTableCellElement>("td");
    if (cell && table.contains(cell)) show(cell);
  }
  function leave() {
    if (owner && !owner.contains(document.activeElement)) hide();
  }
  function dismiss(event: KeyboardEvent) {
    if (event.key === "Escape") hide();
  }
  function scroll() {
    const cell = owner;
    if (cell && cell.contains(document.activeElement)) show(cell);
    else hide();
  }
  table.addEventListener("mouseover", hover);
  table.addEventListener("mouseout", leave);
  table.addEventListener("focusout", hide);
  table.addEventListener("keydown", dismiss);
  scroller?.addEventListener("scroll", scroll);
  return {
    show,
    hide,
    destroy() {
      hide();
      table.removeEventListener("mouseover", hover);
      table.removeEventListener("mouseout", leave);
      table.removeEventListener("focusout", hide);
      table.removeEventListener("keydown", dismiss);
      scroller?.removeEventListener("scroll", scroll);
    },
  };
}
