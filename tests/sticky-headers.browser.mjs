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
    const members=Array.from({length:1000},(_,i)=>({id:'M'+String(i+1).padStart(4,'0'),memberNumber:'K'+String(i+1).padStart(4,'0'),name:'Member '+String(i+1).padStart(4,'0'),joinedAt:'2026-01-01',principalSavings:50000}));
    const loans=members.slice(0,800).map((m,i)=>({id:'L'+String(i+1).padStart(4,'0'),loanGroup:i<400?'Anggota PP BRI':'Non Anggota PP BRI',memberId:m.id,memberName:m.name,plafond:10000000,balance:9000000,rate:12,tenor:24,interestType:'Menurun',loanType:'Bulanan',realizationDate:'2026-01-01',dueDate:'2028-01-01',guarantee:'Sertifikat rumah dan tanah'}));
    const periods=Array.from({length:12},(_,i)=>'2026-'+String(i+1).padStart(2,'0'));
    const ledger={periods,savings:periods.flatMap(period=>members.map(m=>({memberId:m.id,period,transactionDate:period+'-01',principalOpening:50000,mandatoryOpening:100000,voluntaryOpening:0,principalIn:Number(period.slice(-2)),principalOut:0,mandatoryIn:50000,mandatoryOut:0,voluntaryIn:0,voluntaryOut:0,shu:0,principalClosing:50000,mandatoryClosing:150000,voluntaryClosing:0}))),loans:periods.flatMap(period=>loans.map(l=>({loanId:l.id,period,transactionDate:period+'-01',openingBalance:10000000,disbursed:0,principalPaid:100000,interestPaid:10000,provision:0,scheduledPrincipal:100000,scheduledInterest:10000,arrearsPrincipal:0,arrearsInterest:0,closingBalance:9900000,shown:true,nextDuePrincipal:100000,nextDueInterest:10000,recommendation:"",incomeInterest:[30000,30000,30000,30000],incomeProvision:[0,0,0,0],incentiveRaw:0,incentive:0})))};
    window.__TAURI_INTERNALS__={invoke:async(command)=>{
      if(command==='list_companies')return [{id:'default',name:'Test'}];
      if(command==='get_app_snapshot')return {members,loans,transactions:[],savingsBalances:[],totals:{cash:0,savings:200000000,loanPortfolio:7200000000,members:1000}};
      if(command==='get_monthly_ledger')return ledger;
      if(command==='get_cash_book')return {rows:Array.from({length:100},(_,i)=>({id:'C'+i,date:'2026-01-01',description:'Cash entry '+i,memberNumber:'',channel:'Cash',direction:'In',status:'Posted',isReversal:false,amount:1000,balance:1000*(i+1),savingsBookOut:0,savingsBookIn:0})),year:2026,openingBalance:1000,savingsBookOpeningBalance:0,totalIn:100000,totalOut:0,closingBalance:101000};
      if(command==='get_financial_parameters')return {mandatorySavings:50000};
      if(command==='preview_loan')return {};
      if(command==='plugin:app|version')return 'test';
      return null;
    }};
  `,
  });
  const waitFor = async (expr) => {
    for (let i = 0; i < 150; i++) {
      if (await evaluate(expr)) return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error("Timed out: " + expr);
  };
  const assertStickyHeaders = async () => {
    const check = `(()=>{
      const table=document.querySelector('.sheet-table, .data-table');
      const wrap=table.closest('.data-table-wrap');
      const start=wrap.getBoundingClientRect().top+wrap.clientTop;
      let offset=0;
      if(wrap.scrollTop<table.tHead.offsetHeight) return false;
      for(const row of table.tHead.rows){
        for(const cell of row.cells){
          if(getComputedStyle(cell).position!=='sticky' || Math.abs(cell.getBoundingClientRect().top-start-offset)>1) return false;
        }
        offset+=row.getBoundingClientRect().height;
      }
      return getComputedStyle(table.tHead).position!=='sticky';
    })()`;
    await waitFor(check);
    assert(
      await evaluate(check),
      "Every header row must remain pinned at its measured offset",
    );
  };
  for (const path of ["/savings", "/loans", "/members", "/cash-ledger"]) {
    await send("Page.navigate", { url: baseUrl + path });
    await waitFor(`document.querySelector('tbody tr:nth-child(10)')`);
    await evaluate(`document.querySelector('.data-table-wrap').scrollTop=700`);
    await assertStickyHeaders();
    if (path === "/savings" || path === "/loans") {
      await evaluate(
        `document.querySelector('.sheet-wrap').scrollLeft=${path === "/loans" ? 25000 : 19000}`,
      );
      await waitFor(
        `document.querySelector('thead th[data-period="2026-12"]')`,
      );
      await assertStickyHeaders();
      assert(
        await evaluate(`(()=>{
        const table=document.querySelector('.sheet-table');
        const wrap=table.closest('.sheet-wrap');
        const start=wrap.getBoundingClientRect().left;
        return [...table.querySelectorAll('thead .sheet-no')].every(cell=>Math.abs(cell.getBoundingClientRect().left-start)<1)
          && [...table.querySelectorAll('thead .sheet-name')].every(cell=>Math.abs(cell.getBoundingClientRect().left-start-56)<1);
      })()`),
        "Frozen header columns must stick in both directions",
      );
    }
    // Trigger a height change while scrolled, then check every merged row again.
    await evaluate(`document.querySelector('thead tr').style.height='60px'`);
    await waitFor(
      `document.querySelector('thead tr').getBoundingClientRect().height>=60`,
    );
    await assertStickyHeaders();
    await evaluate(`document.querySelector('thead tr').style.height=''`);
    await assertStickyHeaders();
    await send("Emulation.setDeviceMetricsOverride", {
      width: 1000,
      height: 700,
      deviceScaleFactor: 1,
      mobile: false,
    });
    await assertStickyHeaders();
    await send("Emulation.clearDeviceMetricsOverride");
    console.log(
      "PASS: sticky cells, row height changes, and window resize",
      path,
    );
  }
} finally {
  ws?.close();
  if (chrome.exitCode === null) {
    chrome.kill();
    await once(chrome, "exit");
  }
  await vite.close();
  await rm(profile, { recursive: true, force: true });
}
