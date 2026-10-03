import assert from "node:assert/strict";
import { test } from "node:test";
import { createRenderer, h, KeepAlive, nextTick, ref } from "vue";
import { useProgressiveRows } from "../src/composables/useProgressiveRows.ts";

test("sheets yield between batches, restart on filters, and pause in cached tabs", async (t) => {
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
      4,
      "hidden tabs stop rendering",
    );
    visible.value = true;
    await nextTick();
    t.mock.timers.tick(16);
    assert.equal(
      progress.renderedRowCount.value,
      8,
      "returning resumes the cached sheet",
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
      25,
      "closing a tab cancels pending work",
    );
  } finally {
    app.unmount();
  }
});
