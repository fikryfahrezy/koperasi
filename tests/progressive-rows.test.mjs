import assert from "node:assert/strict";
import { test } from "node:test";
import { createRenderer, h, KeepAlive, nextTick, ref } from "vue";
import { useProgressiveRows } from "../src/composables/useProgressiveRows.ts";

test("sheets yield between batches and restart on filters and every tab visit", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const rows = ref(Array.from({ length: 100 }, (_, id) => ({ id })));
  const columns = ref(200);
  const visible = ref(true);
  let progress;
  const Report = {
    setup() {
      progress = useProgressiveRows(
        () => rows.value,
        () => columns.value,
      );
      return () => null;
    },
  };
  const Blank = { render: () => null };
  const renderer = createRenderer({
    createElement: () => ({}),
    createComment: () => ({}),
    insert() {},
    remove() {},
    parentNode: () => null,
    nextSibling: () => null,
  });
  const app = renderer.createApp({
    render: () =>
      h(KeepAlive, null, { default: () => h(visible.value ? Report : Blank) }),
  });
  app.mount({});
  try {
    assert.equal(
      progress.renderedRowCount.value,
      0,
      "first render leaves room for the header to paint",
    );
    assert.equal(progress.renderingRows.value, true);
    t.mock.timers.tick(16);
    assert.equal(
      progress.renderedRowCount.value,
      4,
      "wide sheets use smaller batches",
    );

    visible.value = false;
    await nextTick();
    t.mock.timers.tick(100);
    assert.equal(
      progress.renderedRowCount.value,
      0,
      "hidden tabs clear rendered rows and stop rendering",
    );
    visible.value = true;
    await nextTick();
    assert.equal(progress.renderingRows.value, true);
    assert.equal(progress.renderedRowCount.value, 0);
    t.mock.timers.tick(16);
    assert.equal(
      progress.renderedRowCount.value,
      4,
      "returning starts a new rendering pass using cached data",
    );

    rows.value = rows.value.slice(0, 10);
    columns.value = 20;
    await nextTick();
    assert.equal(
      progress.renderedRowCount.value,
      0,
      "filters cancel the previous batch",
    );
    t.mock.timers.tick(16);
    assert.equal(progress.renderedRowCount.value, 10);
    assert.equal(progress.renderingRows.value, false);

    const cachedRows = rows.value;
    visible.value = false;
    await nextTick();
    visible.value = true;
    await nextTick();
    assert.equal(
      rows.value,
      cachedRows,
      "returning preserves the data and filters",
    );
    assert.equal(progress.renderedRowCount.value, 0);
    assert.equal(
      progress.renderingRows.value,
      true,
      "completed sheets also load on return",
    );
    t.mock.timers.tick(16);
    assert.equal(progress.renderedRowCount.value, 10);
    assert.equal(progress.renderingRows.value, false);

    rows.value = [];
    await nextTick();
    assert.equal(
      progress.renderingRows.value,
      false,
      "empty reports finish immediately",
    );
    rows.value = Array.from({ length: 100 }, (_, id) => ({ id }));
    await nextTick();
    t.mock.timers.tick(16);
    assert.equal(progress.renderedRowCount.value, 25);
    app.unmount();
    t.mock.timers.tick(100);
    assert.equal(
      progress.renderedRowCount.value,
      0,
      "closing a tab cancels pending work",
    );
  } finally {
    app.unmount();
  }
});
