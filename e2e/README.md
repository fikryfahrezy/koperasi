# Native desktop E2E tests

These WebdriverIO tests launch the real Tauri app and call the real Rust/SQLite
backend. The member test creates a member with Rp50.000 principal savings,
restarts the app, and checks that the member and savings persisted.

## Run on Linux

Install the normal Tauri build prerequisites, plus `webkit2gtk-driver` and
`xvfb` on Debian/Ubuntu. Install the driver separately from the app's pinned
Rust toolchain:

```sh
rustup toolchain install stable --profile minimal
cargo +stable install tauri-driver --version 2.0.6 --locked
pnpm install --frozen-lockfile
xvfb-run -a pnpm test:e2e
```

With a graphical desktop already available, `pnpm test:e2e` also works without
Xvfb. Ensure `~/.cargo/bin` is in PATH. Set `TAURI_DRIVER` if the executable is
elsewhere; `KOPERASI_E2E_PORT` overrides the default port 4444.

## Run on Windows

Install the normal Tauri Windows build prerequisites, `tauri-driver` as above,
and `msedgedriver.exe` in PATH. The Edge driver must match the WebView2 runtime
actually used by the debug app (normally the system WebView2 runtime).
Then run `pnpm test:e2e`. The Linux CI job is the automated baseline; Windows
execution still needs validation on a Windows machine.

## macOS and CI

The official external `tauri-driver` supports Linux and Windows. The current
embedded driver supports macOS but all published versions require Tauri 2.10,
while Koperasi pins Tauri 2.5.1 for Windows 7 compatibility. This setup preserves
those pins and reports an explanatory error on macOS. Run the **E2E** workflow
in GitHub Actions to test from a Mac; it also runs for pull requests and pushes
to `main`.

References: [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/),
[manual driver setup](https://v2.tauri.app/develop/tests/webdriver/manual/).

## Isolation and debugging

The runner builds automatically with the `e2e` Cargo feature and `VITE_E2E=true`.
It uses `src-tauri/target/e2e` for build output, a separate application identifier,
and a fresh temporary directory for SQLite on every run. The temporary database
is retained across the restart inside the test and deleted when the suite ends.
E2E builds refuse to start without `KOPERASI_E2E_DATA_DIR`, so they cannot fall
back to the normal database. Updater, single-instance, and window-state plugins
are omitted, and automatic frontend update checks are disabled in test builds.

Production builds do not enable the `e2e` feature or `VITE_E2E`; use the normal
`pnpm tauri build` command for those. WebdriverIO records logs and failure
screenshots under `e2e/artifacts/`, which CI uploads when a run fails.

Keep each additional spec independent: the current runner shares one temporary
database for the suite, so use unique member names and avoid relying on earlier
tests. Reset fixtures explicitly if a new test needs a completely empty ledger.
