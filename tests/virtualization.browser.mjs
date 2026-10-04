import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { once } from "node:events";
import { createServer as createNetServer } from "node:net";
import assert from "node:assert/strict";
const root = fileURLToPath(new URL("../", import.meta.url));
const chromeBinary =
  process.env.CHROME_BINARY ||
  (process.platform === "darwin"
    ? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    : process.platform === "win32"
      ? join(
          process.env.PROGRAMFILES || "C:\\Program Files",
          "Google",
          "Chrome",
          "Application",
          "chrome.exe",
        )
      : "/usr/bin/google-chrome");
if (!existsSync(chromeBinary))
  throw new Error(
    "Set CHROME_BINARY to an installed Chrome/Chromium executable.",
  );
const profile = await mkdtemp(join(tmpdir(), "koperasi-virtual-"));
const portServer = createNetServer();
portServer.listen(0, "127.0.0.1");
await once(portServer, "listening");
const debugPort = portServer.address().port;
await new Promise((resolve) => portServer.close(resolve));
const require = createRequire(root + "/package.json");
const { createServer } = await import(pathToFileURL(require.resolve("vite")));
const vite = await createServer({
  root,
  server: { port: 0, host: "127.0.0.1", hmr: false, strictPort: false },
});
await vite.listen();
const baseUrl = `http://127.0.0.1:${vite.httpServer.address().port}`;
const chrome = spawn(
  chromeBinary,
  [
    "--headless=new",
    "--no-first-run",
    "--no-default-browser-check",
    `--remote-debugging-port=${debugPort}`,
    `--user-data-dir=${profile}`,
    "--window-size=1280,800",
    "about:blank",
  ],
  { stdio: "ignore" },
);
let ws;
try {
  let targets;
  for (let i = 0; i < 100; i++) {
    try {
      targets = await (
        await fetch(`http://127.0.0.1:${debugPort}/json`)
      ).json();
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  if (!targets) throw new Error("Chrome did not start");
  ws = new WebSocket(
    targets.find((t) => t.type === "page").webSocketDebuggerUrl,
  );
  await new Promise((r) => ws.addEventListener("open", r, { once: true }));
  let seq = 0;
  const pending = new Map();
  ws.addEventListener("message", (e) => {
    const m = JSON.parse(e.data);
    if (m.id) {
      const p = pending.get(m.id);
      pending.delete(m.id);
      m.error ? p.reject(m.error) : p.resolve(m.result);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = ++seq;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  const evaluate = async (expression) => {
    const r = await send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  };
  await send("Page.enable");
  await send("Page.addScriptToEvaluateOnNewDocument", {
    source: `
    const members=Array.from({length:1000},(_,i)=>({id:'M'+String(i+1).padStart(4,'0'),name:'Member '+String(i+1).padStart(4,'0'),joinedAt:'2026-01-01',principalSavings:50000}));
    const loans=members.slice(0,800).map((m,i)=>({id:'L'+String(i+1).padStart(4,'0'),loanGroup:i<400?'Anggota PP BRI':'Non Anggota PP BRI',memberId:m.id,memberName:m.name,plafond:10000000,balance:9000000,rate:12,tenor:24,interestType:'Menurun',loanType:'Bulanan',realizationDate:'2026-01-01',dueDate:'2028-01-01',guarantee:'TEST'}));
    const periods=Array.from({length:12},(_,i)=>'2026-'+String(i+1).padStart(2,'0'));
    const ledger={periods,savings:periods.flatMap(period=>members.map(m=>({memberId:m.id,period,transactionDate:period+'-01',principalOpening:50000,mandatoryOpening:100000,voluntaryOpening:0,principalIn:0,principalOut:0,mandatoryIn:50000,mandatoryOut:0,voluntaryIn:0,voluntaryOut:0,shu:0,principalClosing:50000,mandatoryClosing:150000,voluntaryClosing:0}))),loans:periods.flatMap(period=>loans.map(l=>({loanId:l.id,period,transactionDate:period+'-01',openingBalance:10000000,disbursed:0,principalPaid:100000,interestPaid:10000,provision:0,scheduledPrincipal:100000,scheduledInterest:10000,arrearsPrincipal:0,arrearsInterest:0,closingBalance:9900000})))};
    window.__TAURI_INTERNALS__={invoke:async(command)=>{
      if(command==='list_companies')return [{id:'default',name:'Test'}];
      if(command==='get_app_snapshot')return {members,loans,transactions:[],savingsBalances:[],totals:{cash:0,savings:200000000,loanPortfolio:7200000000,members:1000}};
      if(command==='get_monthly_ledger')return ledger;
      if(command==='get_cash_book')return {rows:[],year:2026,openingBalance:0,totalIn:0,totalOut:0,closingBalance:0};
      if(command==='get_financial_parameters')return {mandatorySavings:50000};
      if(command==='preview_loan')return {};
      if(command==='plugin:app|version')return 'test';
      return null;
    }};
  `,
  });
  await send("Page.navigate", { url: baseUrl + "/savings" });
  const waitFor = async (expr) => {
    for (let i = 0; i < 150; i++) {
      if (await evaluate(expr)) return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error("Timed out: " + expr);
  };
  await waitFor(`document.querySelector('tbody tr[data-virtual-row]')`);
  const read = () =>
    evaluate(
      `(()=>{const table=document.querySelector('.sheet-table');const wrap=table.closest('.data-table-wrap');const rows=[...table.querySelectorAll('tbody tr[data-virtual-row]')];return {count:rows.length,first:Number(rows[0]?.dataset.virtualRow),last:Number(rows.at(-1)?.dataset.virtualRow),heights:rows.map(r=>r.getBoundingClientRect().height),scrollTop:wrap.scrollTop,scrollHeight:wrap.scrollHeight,error:document.querySelector('.runtime-banner')?.textContent,footer:table.tFoot.textContent}})()`,
    );
  let result = await read();
  assert(result.count > 0 && result.count < 45, JSON.stringify(result));
  assert(
    result.heights.every((h) => h === 32),
    JSON.stringify(result.heights),
  );
  assert(!result.error, result.error);
  console.log(
    "Savings initial",
    JSON.stringify({
      count: result.count,
      first: result.first,
      last: result.last,
      scrollHeight: result.scrollHeight,
    }),
  );
  await evaluate(`document.querySelector('.sheet-wrap').scrollTop=24000`);
  await waitFor(
    `Number(document.querySelector('tbody tr[data-virtual-row]')?.dataset.virtualRow)>700`,
  );
  result = await read();
  assert(result.count < 45);
  assert(result.footer.includes("200.000.000"));
  console.log(
    "Savings scrolled",
    JSON.stringify({
      count: result.count,
      first: result.first,
      last: result.last,
    }),
  );
  await evaluate(
    `document.querySelector('tbody tr[data-virtual-row] td').click()`,
  );
  await send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key: "ArrowDown",
    code: "ArrowDown",
  });
  for (let i = 0; i < 100; i++)
    await send("Input.dispatchKeyEvent", {
      type: "keyDown",
      key: "ArrowDown",
      code: "ArrowDown",
    });
  await waitFor(
    `Number(document.activeElement?.parentElement?.dataset.virtualRow)>790`,
  );
  console.log(
    "Keyboard offscreen row",
    await evaluate(`document.activeElement.parentElement.dataset.virtualRow`),
  );
  await evaluate(`document.querySelector('.workspace-tabs__new').click()`);
  await waitFor(`location.pathname==='/new-tab'`);
  await evaluate(`document.querySelector('a[href="/savings"]').click()`);
  await waitFor(
    `location.pathname==='/savings' && document.querySelector('tbody tr[data-virtual-row]')?.dataset.virtualRow==='0'`,
  );
  assert((await read()).count < 45);
  await evaluate(`document.querySelector('.workspace-tab').click()`);
  await waitFor(
    `Number(document.querySelector('tbody tr[data-virtual-row]')?.dataset.virtualRow)>700`,
  );
  console.log(
    "Tab restoration",
    JSON.stringify({
      first: (await read()).first,
      scrollTop: (await read()).scrollTop,
    }),
  );
  await evaluate(
    `document.querySelector('input[type=search]').value='Member 1000';document.querySelector('input[type=search]').dispatchEvent(new Event('input',{bubbles:true}))`,
  );
  await waitFor(
    `document.querySelectorAll('tbody tr[data-virtual-row]').length===1`,
  );
  assert((await read()).footer.includes("200.000"));
  console.log(
    "Savings filtering",
    JSON.stringify({
      count: (await read()).count,
      scrollTop: (await read()).scrollTop,
    }),
  );
  await evaluate(`document.querySelector('a[href="/loans"]').click()`);
  await waitFor(
    `location.pathname==='/loans' && document.querySelector('tbody tr[data-virtual-row]')`,
  );
  result = await read();
  assert(result.count < 45);
  assert(
    result.heights.every((h) => h === 32),
    JSON.stringify(result.heights),
  );
  assert(result.footer.includes("8.000.000.000"));
  assert(!result.error, result.error);
  console.log(
    "Loans initial",
    JSON.stringify({
      count: result.count,
      first: result.first,
      last: result.last,
    }),
  );
  await evaluate(`document.querySelector('.sheet-wrap').scrollTop=12800`);
  await waitFor(
    `Number(document.querySelector('tbody tr[data-virtual-row]')?.dataset.virtualRow)>380`,
  );
  result = await read();
  assert(result.count < 45);
  console.log(
    "Loans section boundary",
    JSON.stringify({
      count: result.count,
      first: result.first,
      last: result.last,
      text: await evaluate(
        `document.querySelector('tbody').textContent.includes('SUB JUMLAH') && document.querySelector('tbody').textContent.includes('PINJAMAN NON ANGGOTA')`,
      ),
    }),
  );
  assert(
    await evaluate(
      `document.querySelector('tbody').textContent.includes('SUB JUMLAH') && document.querySelector('tbody').textContent.includes('PINJAMAN NON ANGGOTA')`,
    ),
  );
  assert(!result.error, result.error);
  console.log(
    "PASS: virtual reports, row height, totals, keyboard navigation, tab restore, filtering, section boundary",
  );
} finally {
  ws?.close();
  if (chrome.exitCode === null) {
    chrome.kill();
    await once(chrome, "exit");
  }
  await vite.close();
  await rm(profile, { recursive: true, force: true });
}
