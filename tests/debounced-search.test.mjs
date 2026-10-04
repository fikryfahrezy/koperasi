import assert from "node:assert/strict";
import { test } from "node:test";
import { computed, effectScope, ref } from "vue";
import { useDebouncedRef } from "../src/composables/useDebouncedRef.ts";

test("search waits for a typing pause, clears immediately, and cancels on disposal", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const scope = effectScope();
  const query = ref("initial");
  const tabId = ref("first");
  const search = scope.run(() => useDebouncedRef(query, tabId));
  try {
    assert.equal(search.value, "initial");
    query.value = "a";
    assert.equal(query.value, "a", "input updates immediately");
    t.mock.timers.tick(200);
    query.value = "andi";
    t.mock.timers.tick(299);
    assert.equal(search.value, "initial");
    t.mock.timers.tick(1);
    assert.equal(search.value, "andi");

    query.value = "pending";
    query.value = "";
    assert.equal(search.value, "");
    t.mock.timers.tick(300);
    assert.equal(search.value, "", "clearing cancels the previous search");

    query.value = "discarded";
    scope.stop();
    t.mock.timers.tick(300);
    assert.equal(search.value, "");
  } finally {
    scope.stop();
  }
});

test("switching tabs restores the current query and cancels the previous tab's timer", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const scope = effectScope();
  const tabId = ref("first");
  const queries = ref({ first: "first member", second: "second member" });
  const query = computed({
    get: () => queries.value[tabId.value],
    set: (value) => {
      queries.value[tabId.value] = value;
    },
  });
  const search = scope.run(() => useDebouncedRef(query, tabId));
  try {
    query.value = "pending first member";
    t.mock.timers.tick(100);
    tabId.value = "second";
    assert.equal(search.value, "second member");
    t.mock.timers.tick(300);
    assert.equal(search.value, "second member");
    tabId.value = "first";
    assert.equal(search.value, "pending first member");
  } finally {
    scope.stop();
  }
});
