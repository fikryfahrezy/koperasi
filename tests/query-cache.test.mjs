import assert from "node:assert/strict";
import { test } from "node:test";
import { createRenderer, nextTick } from "vue";
import { VueQueryPlugin } from "@tanstack/vue-query";
import { createServer } from "vite";

// Exercise the real store and query observers without requiring a native Tauri window.
test("cached reports, company isolation, mutations, and manual refresh", async () => {
  const server = await createServer({
    server: { middlewareMode: true, hmr: false, ws: false },
  });
  const calls = [];
  let revision = 0;
  let failReports = false;
  const snapshot = (companyId) => ({
    members: [{ id: companyId, name: companyId }],
    loans: [],
    transactions: [],
    savingsBalances: [],
    totals: { cash: revision, savings: 0, loanPortfolio: 0, members: 1 },
  });
  const originalWindow = globalThis.window;
  globalThis.window = {
    setTimeout: () => 0,
    __TAURI_INTERNALS__: {
      invoke: async (command, args = {}) => {
        calls.push({ command, ...args });
        if (command === "list_companies") return [{ id: "a" }, { id: "b" }];
        if (command === "get_app_snapshot") return snapshot(args.companyId);
        if (command === "get_financial_parameters")
          return { principalSavings: 100 };
        if (command === "post_cash_entry") {
          revision++;
          return snapshot(args.companyId);
        }
        if (failReports) throw new Error("Report unavailable");
        if (command === "get_monthly_ledger") {
          return { periods: [String(args.year)], savings: [], loans: [] };
        }
        if (command === "get_cash_book") {
          return { rows: [], year: args.year, closingBalance: revision };
        }
        throw new Error(`Unexpected command: ${command}`);
      },
    },
  };
  const renderer = createRenderer({
    createComment: () => ({}),
    insert() {},
    remove() {},
    parentNode: () => null,
    nextSibling: () => null,
  });
  const apps = [];
  let queryClient;
  try {
    const clientModule = await server.ssrLoadModule("/src/query-client.ts");
    queryClient = clientModule.queryClient;
    const { queryKeys } = clientModule;
    const { useKoperasiStore, useMonthlyLedger, useCashBook, Channel } =
      await server.ssrLoadModule("/src/store/koperasi.ts");
    const store = useKoperasiStore();
    await store.initialize();
    const count = (command) =>
      calls.filter((call) => call.command === command).length;
    const settle = async () => {
      for (let attempt = 0; attempt < 100; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 1));
        await nextTick();
        if (!queryClient.isFetching()) return;
      }
      assert.fail("Queries did not settle");
    };
    const mountReports = () => {
      let reports;
      const app = renderer.createApp({
        setup() {
          reports = { ...useMonthlyLedger(), ...useCashBook(Channel.Cash) };
          return () => null;
        },
      });
      app.use(VueQueryPlugin, { queryClient });
      app.mount({});
      apps.push(app);
      return reports;
    };
    const reports = mountReports();
    await settle();
    mountReports();
    await settle();
    assert.equal(count("get_monthly_ledger"), 1, "tabs share monthly requests");
    assert.equal(count("get_cash_book"), 1, "tabs share cash requests");
    assert.equal(reports.loadingBook.value, false);

    const year = store.selectedYear.value;
    store.selectedYear.value = year - 1;
    await settle();
    assert.equal(reports.cashBook.value.year, year - 1);
    store.selectedYear.value = year;
    await settle();
    assert.equal(
      count("get_cash_book"),
      2,
      "returning to a year uses its cache",
    );

    await store.selectCompany("b");
    await settle();
    await store.selectCompany("a");
    await settle();
    assert.equal(
      count("get_app_snapshot"),
      2,
      "returning to a company uses its cache",
    );
    assert.equal(count("get_cash_book"), 3);
    assert.equal(store.members[0].id, "a");

    await store.postCashEntry({
      direction: "Masuk",
      category: "",
      description: "Test",
      amount: 1,
      reference: "",
    });
    await settle();
    assert.equal(
      reports.cashBook.value.closingBalance,
      1,
      "saves refresh reports",
    );
    assert.equal(
      queryClient.getQueryData(queryKeys.snapshot("a")).totals.cash,
      1,
    );
    assert.equal(
      queryClient.getQueryData(queryKeys.snapshot("b")).totals.cash,
      0,
    );
    const beforeRefresh = count("get_cash_book");
    assert.equal(await store.refresh(), true);
    await settle();
    assert.equal(
      count("get_cash_book"),
      beforeRefresh + 1,
      "refresh bypasses cache",
    );

    failReports = true;
    await store.refresh();
    await settle();
    assert.equal(
      reports.cashBook.value.closingBalance,
      1,
      "failed refresh retains cached rows",
    );
    assert.equal(
      reports.loadingBook.value,
      false,
      "background refresh does not reset loading",
    );
  } finally {
    for (const app of apps) app.unmount();
    queryClient?.clear();
    globalThis.window = originalWindow;
    await server.close();
  }
});
