import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { compileScript, parse } from "vue/compiler-sfc";
import ts from "typescript";
import { createRenderer, h, nextTick } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";

const workspace = await import("../src/workspace-tabs.ts");
const stateSource = readFileSync(
  new URL("../src/composables/useWorkspacePageState.ts", import.meta.url),
  "utf8",
);
const stateCode = ts
  .transpileModule(stateSource, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.ESNext,
    },
  })
  .outputText.replace(
    'from "../workspace-tabs"',
    `from "${new URL("../src/workspace-tabs.ts", import.meta.url).href}"`,
  )
  .replace('from "vue"', `from "${import.meta.resolve("vue")}"`);
const stateUrl = `data:text/javascript;base64,${Buffer.from(stateCode).toString("base64")}`;

const source = readFileSync(
  new URL("../src/components/WorkspaceTabPages.vue", import.meta.url),
  "utf8",
);
const { descriptor } = parse(source);
const script = compileScript(descriptor, {
  id: "tab-pages-test",
  inlineTemplate: true,
});
let code = ts.transpileModule(script.content, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
  },
}).outputText;
code = code.replace(
  /import (\w+) from "[^"\n]+\.vue";/g,
  (_, name) =>
    `const ${name} = { name: "${name}", setup() {
      globalThis.__tabPageMounts.push("${name}");
      const state = useWorkspacePageState("${name === "CashLedger" ? "/cash-ledger" : name === "NewTab" ? "/new-tab" : "/" + name.toLowerCase()}");
      globalThis.__tabPageFields["${name}"] = state.field("query", () => "");
      globalThis.__tabPageStates["${name}"] = state;
      return () => h("div", { ref: el => { if (el) globalThis.__tabPageRoots["${name}"] = el; } }, "${name}");
    } };`,
);
code = code.replace(
  'from "./CachedWorkspacePage"',
  `from "${new URL("../src/components/CachedWorkspacePage.ts", import.meta.url).href}"`,
);
code = code.replace(
  'from "../workspace-tabs"',
  `from "${new URL("../src/workspace-tabs.ts", import.meta.url).href}"`,
);
code =
  `import { h } from "vue";\nimport { useWorkspacePageState } from "${stateUrl}";\n` +
  code;
for (const name of ["vue", "vue-router"]) {
  code = code
    .replaceAll(`from "${name}"`, `from "${import.meta.resolve(name)}"`)
    .replaceAll(`from '${name}'`, `from "${import.meta.resolve(name)}"`);
}
const { default: WorkspaceTabPages } = await import(
  `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`
);

function renderer() {
  const node = (type, text = "") => ({
    type,
    text,
    children: [],
    parent: null,
    style: {},
    scrollTop: 0,
    scrollLeft: 0,
    closest: () => null,
    querySelectorAll() {
      return [this];
    },
  });
  return createRenderer({
    createElement: node,
    createText: (text) => node("text", text),
    createComment: (text) => node("comment", text),
    setText: (node, text) => {
      node.text = text;
    },
    setElementText: (node, text) => {
      node.text = text;
    },
    parentNode: (node) => node.parent,
    nextSibling: (node) =>
      node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
    patchProp() {},
    insert(node, parent, anchor = null) {
      if (node.parent)
        node.parent.children.splice(node.parent.children.indexOf(node), 1);
      const index = anchor ? parent.children.indexOf(anchor) : -1;
      parent.children.splice(
        index < 0 ? parent.children.length : index,
        0,
        node,
      );
      node.parent = parent;
    },
    remove(node) {
      if (node.parent)
        node.parent.children.splice(node.parent.children.indexOf(node), 1);
      node.parent = null;
    },
  });
}

test("main pages mount once across tabs, preserving filters and scroll without cache errors", async () => {
  globalThis.__tabPageMounts = [];
  globalThis.__tabPageFields = {};
  globalThis.__tabPageRoots = {};
  globalThis.__tabPageStates = {};
  const errors = [];
  const originalTabs = workspace.workspaceTabs.value;
  const originalActiveId = workspace.activeWorkspaceTabId.value;
  workspace.workspaceTabs.value = [
    { id: "first", fullPath: "/new-tab", titleKey: "new" },
  ];
  workspace.activeWorkspaceTabId.value = "first";
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ["/new-tab", "/members", "/savings", "/loans", "/cash-ledger"].map(
      (path) => ({ path, component: { render: () => null } }),
    ),
  });
  await router.push("/new-tab");
  const app = renderer().createApp({ render: () => h(WorkspaceTabPages) });
  app.use(router);
  app.config.errorHandler = (error) => errors.push(error);
  app.mount({ children: [] });
  const settle = async () => {
    await nextTick();
    await nextTick();
  };
  try {
    await settle();
    assert.equal(globalThis.__tabPageMounts.length, 5);
    workspace.workspaceTabs.value[0].fullPath = "/loans";
    await settle();
    const filter = globalThis.__tabPageFields.Loans;
    const loanRoot = globalThis.__tabPageRoots.Loans;
    const loanState = globalThis.__tabPageStates.Loans;
    const draft = loanState.field("draft", () => ({ amount: 0 }));
    draft.value.amount = 100;
    filter.value = "first borrower";
    loanRoot.scrollTop = 240;
    loanRoot.scrollLeft = 80;
    const submittedFilter = filter.forTab("first");

    workspace.workspaceTabs.value.push({
      id: "second",
      fullPath: "/new-tab",
      titleKey: "new",
    });
    workspace.activeWorkspaceTabId.value = "second";
    await settle();
    assert.equal(
      globalThis.__tabPageMounts.length,
      5,
      "new tabs do not mount additional reports",
    );
    assert.equal(
      filter.value,
      "first borrower",
      "blank tabs do not recompute hidden report state",
    );
    workspace.workspaceTabs.value[1].fullPath = "/loans";
    await settle();
    assert.equal(filter.value, "");
    assert.equal(loanState.active.value, true);
    assert.equal(draft.value.amount, 0);
    draft.value.amount = 200;
    assert.equal(loanRoot.scrollTop, 0);
    assert.equal(loanRoot.scrollLeft, 0);
    filter.value = "second borrower";
    loanRoot.scrollTop = 480;
    submittedFilter.value = "first completed";
    assert.equal(
      filter.value,
      "second borrower",
      "pending results update only their original tab",
    );

    workspace.activeWorkspaceTabId.value = "first";
    await settle();
    assert.equal(filter.value, "first completed");
    assert.equal(draft.value.amount, 100);
    assert.equal(loanRoot.scrollTop, 240);
    assert.equal(loanRoot.scrollLeft, 80);
    workspace.activeWorkspaceTabId.value = "second";
    await settle();
    assert.equal(filter.value, "second borrower");
    assert.equal(loanRoot.scrollTop, 480);
    assert.equal(draft.value.amount, 200);
    for (const fullPath of [
      "/savings",
      "/members",
      "/cash-ledger",
      "/new-tab",
      "/loans",
    ]) {
      workspace.workspaceTabs.value[1].fullPath = fullPath;
      await settle();
    }
    workspace.workspaceTabs.value.splice(0, 1);
    await settle();
    assert.equal(
      filter.forTab("first").value,
      "",
      "closed tabs release their saved state",
    );
    assert.deepEqual(errors, []);
    assert.equal(globalThis.__tabPageMounts.length, 5);
  } finally {
    app.unmount();
    workspace.workspaceTabs.value = originalTabs;
    workspace.activeWorkspaceTabId.value = originalActiveId;
    delete globalThis.__tabPageMounts;
    delete globalThis.__tabPageFields;
    delete globalThis.__tabPageRoots;
    delete globalThis.__tabPageStates;
  }
});
