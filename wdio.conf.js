import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { once } from "node:events";
import { browser } from "@wdio/globals";
import { SevereServiceError } from "webdriverio";

const root = fileURLToPath(new URL(".", import.meta.url));
const target = join(root, "src-tauri", "target", "e2e");
const application = join(
  target,
  "debug",
  process.platform === "win32" ? "koperasi.exe" : "koperasi",
);
const port = Number(process.env.KOPERASI_E2E_PORT ?? 4444);
let driver;
let dataDir;
let driverError;

async function cleanup() {
  if (driver && driver.exitCode === null && driver.signalCode === null) {
    const exited = once(driver, "exit").catch(() => {});
    driver.kill();
    await exited;
  }
  driver = undefined;
  if (dataDir) rmSync(dataDir, { recursive: true, force: true });
  dataDir = undefined;
}

export const config = {
  runner: "local",
  hostname: "127.0.0.1",
  port,
  path: "/",
  specs: ["./e2e/specs/**/*.e2e.js"],
  maxInstances: 1,
  capabilities: [{ "tauri:options": { application } }],
  framework: "mocha",
  reporters: ["spec"],
  outputDir: "./e2e/artifacts",
  logLevel: "warn",
  waitforTimeout: 15000,
  connectionRetryTimeout: 30000,
  connectionRetryCount: 1,
  mochaOpts: { timeout: 60000 },

  async onPrepare() {
    if (!["linux", "win32"].includes(process.platform)) {
      throw new SevereServiceError(
        "Native E2E tests require Linux or Windows. Run the E2E GitHub Actions workflow from macOS. The embedded macOS driver requires Tauri 2.10, incompatible with this app's Tauri 2.5.1 Windows 7 pin.",
      );
    }
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw new SevereServiceError(
        "KOPERASI_E2E_PORT must be a valid TCP port",
      );
    }
    const driverPath = process.env.TAURI_DRIVER ?? "tauri-driver";
    // tauri-driver 2.0.6 rejects --version; --help exits successfully.
    const probe = spawnSync(driverPath, ["--help"], {
      encoding: "utf8",
      timeout: 5000,
    });
    if (probe.error || probe.status !== 0) {
      const detail =
        probe.error?.message ||
        probe.stderr?.trim() ||
        `exit status ${probe.status}, signal ${probe.signal ?? "none"}`;
      throw new SevereServiceError(
        `Cannot run ${driverPath} --help: ${detail}\nInstall the driver with: cargo +stable install tauri-driver --version 2.0.6 --locked (or set TAURI_DRIVER to its executable).`,
      );
    }
    try {
      dataDir = mkdtempSync(join(tmpdir(), "koperasi-e2e-"));
      const result = spawnSync(
        process.execPath,
        [
          join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"),
          "build",
          "--debug",
          "--no-bundle",
          "--features",
          "e2e",
          "--config",
          join(root, "e2e", "tauri.e2e.conf.json"),
        ],
        {
          cwd: root,
          stdio: "inherit",
          env: { ...process.env, CARGO_TARGET_DIR: target, VITE_E2E: "true" },
        },
      );
      if (result.error || result.status !== 0) {
        throw result.error ?? new Error(`E2E build failed (${result.status})`);
      }
      driverError = undefined;
      driver = spawn(driverPath, ["--port", String(port)], {
        stdio: "inherit",
        env: {
          ...process.env,
          KOPERASI_E2E_DATA_DIR: dataDir,
          TAURI_WEBVIEW_AUTOMATION: "true",
        },
      });
      driver.on("error", (error) => {
        driverError = error;
      });
      driver.on("exit", (code) => {
        driverError = new Error(`tauri-driver exited (${code})`);
      });
      const deadline = Date.now() + 15000;
      while (Date.now() < deadline) {
        if (driverError) throw driverError;
        try {
          const response = await fetch(`http://127.0.0.1:${port}/status`, {
            signal: AbortSignal.timeout(1000),
          });
          if (response.ok) return;
        } catch {
          // Driver startup is asynchronous; wait for its readiness endpoint.
        }
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      throw new Error("tauri-driver did not become ready within 15 seconds");
    } catch (error) {
      await cleanup();
      throw new SevereServiceError(error.message);
    }
  },

  async afterTest(_test, _context, { passed }) {
    if (!passed) {
      const directory = join(root, "e2e", "artifacts");
      mkdirSync(directory, { recursive: true });
      await browser.saveScreenshot(
        join(directory, `failure-${Date.now()}.png`),
      );
    }
  },
  onComplete: cleanup,
};
