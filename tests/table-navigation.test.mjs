import assert from "node:assert/strict";
import { test } from "node:test";
import { scrollTableCellIntoView } from "../src/directives/tableNavigation.ts";

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
