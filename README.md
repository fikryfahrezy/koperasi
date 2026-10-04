# Koperasi Bina Sejahtera

Aplikasi operasional desktop berbasis Vue 3 + Tauri untuk mengelola anggota,
simpanan, pinjaman, pembayaran anggota, dan buku kas. Menu utama menyediakan
empat halaman: Buku kas, Simpanan, Pinjaman, dan Anggota, dengan detail anggota
yang dapat dibuka dari daftar anggota.

## Target desktop

- Windows 7 64-bit.
- WebView2 Fixed Runtime `88.0.705.81`, dibundel bersama aplikasi.
- Rust `1.77.2` sebagai toolchain project untuk target legacy ini.
- JavaScript dan CSS dikompilasi untuk Chromium 88 melalui konfigurasi Vite.

Versi WebView2, Rust, dan Tauri tidak boleh dinaikkan secara independen tanpa
pengujian ulang pada mesin Windows 7. Workflow CI memverifikasi bahwa fixed
runtime tersedia sebelum proses packaging.

## Menjalankan aplikasi

```bash
pnpm install
pnpm tauri dev
```

Perintah tersebut menyalakan frontend Vite dan shell desktop beserta backend
Rust. `pnpm run dev` hanya menjalankan frontend sehingga command database tidak
tersedia.

Untuk menetapkan perusahaan dan menyembunyikan pilihan perusahaan di toolbar,
buat file `.env.local` di root project:

```dotenv
VITE_COMPANY_ID=default
```

Gunakan ID perusahaan yang tersedia di database. Jika variabel tidak diisi atau
dikosongkan, pilihan perusahaan tetap ditampilkan dan dapat diubah. ID yang tidak
ditemukan akan menampilkan error konfigurasi. Nilai ini dibaca Vite saat dev/build;
restart proses dev atau build ulang aplikasi setelah mengubahnya.

Untuk validasi frontend produksi dan test backend:

```bash
pnpm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
```

Test E2E aplikasi desktop menggunakan WebdriverIO tersedia melalui
`pnpm test:e2e` pada Linux/Windows dan workflow **E2E** di GitHub Actions.
Lihat [panduan E2E](e2e/README.md) untuk instalasi driver dan batasan macOS.

Database lokal dibuat otomatis pada startup pertama. Di Windows lokasinya berada
di `%APPDATA%\com.fikryfahrezy.koperasi\koperasi.db`.

## Arsitektur transaksi

- Vue hanya menangani form dan read model tampilan.
- Command Tauri di Rust memvalidasi transaksi dan periode.
- SQLite menyimpan anggota, rekening simpanan, kontrak pinjaman, komponen
  transaksi, posting kas, parameter berversi, dan audit event.
- Setoran/penarikan simpanan, pencairan + provisi, pembayaran split, serta
  reversal memakai database transaction agar seluruh posting commit atau
  rollback bersama-sama.

Aplikasi membuka tab baru untuk memilih modul dari menu utama dan tidak
memerlukan proses login. Seluruh mutasi operasional disimpan oleh backend Rust ke database
SQLite lokal di direktori data aplikasi; frontend Vue hanya memanggil command
Tauri dan menampilkan snapshot ledger. Posting pembayaran dan reversal dijalankan
secara atomik serta direkam pada audit trail. Entrypoint backend hanya melayani modul aktif. Impor workbook dan perubahan
parameter administrasi telah dihapus; data hasil impor yang sudah tersimpan
tetap dibaca dari ledger, dan parameter keuangan tetap tersedia untuk Buku kas.

## Virtualized reports

Savings and Loans use `@tanstack/vue-virtual` pinned to `3.13.39` (virtual-core
`3.17.11` in the lockfile). Only visible rows plus five rows of overscan on each
side are mounted. Totals use the complete filtered report. Loan section headings
and subtotals participate in the virtual row list. Monthly columns are virtualized
as complete month groups, preserving the merged four-row headers. Leading identity
and loan metadata columns remain mounted, with the first two columns pinned. Empty
column spacers preserve the complete scroll width without mounting offscreen months.

Rows have a fixed height of 32px and explicit column widths to prevent table layout
from changing as different rows enter the viewport. Arrow-key navigation uses
logical row and column indexes and renders an offscreen destination before focusing it.

Run `pnpm test:virtualization` with Chrome installed, or set `CHROME_BINARY` to a
Chrome/Chromium executable. This browser regression check starts a separate local
Vite server and temporary browser profile, replaces Tauri commands with synthetic
data (1,000 members, 800 loans, 12 months), and checks bounded DOM row counts, row
heights, horizontal month windows, header/data/total alignment, totals, filtering,
keyboard navigation, section boundaries and tab scroll
restoration. It does not access the native application database.

The production build still targets Chromium 88. Native `scrollend` is disabled;
TanStack uses its timer fallback. The browser regression runs on the locally
installed browser and does **not** certify Windows 7 compatibility. Before release,
test the packaged application on Windows 7 x64 with bundled WebView2 `88.0.705.81`:
scroll both reports to the bottom and across months, traverse offscreen rows with
arrow keys, cross both loan groups, filter while scrolled down, and switch between
two tabs with different filters and scroll positions. Verify totals and absence of
blank ranges or runtime errors. Do not upgrade the fixed runtime for this feature.
