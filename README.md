# Koperasi Bina Sejahtera

Aplikasi desktop berbasis Vue 3 + Tauri untuk mengelola anggota, simpanan,
pinjaman, pembayaran anggota, dan buku kas. Data disimpan di SQLite lokal.

## Menjalankan aplikasi

```bash
bun install
bun tauri dev
```

`bun tauri dev` menjalankan frontend dan backend Rust. `bun run dev` hanya
menjalankan frontend sehingga command database tidak tersedia.

Database dibuat otomatis saat startup pertama. Di Windows, lokasinya berada di
`%APPDATA%\com.fikryfahrezy.koperasi\koperasi.db`.
Menu **Database** di kanan bawah mengekspor seluruh database ke satu file `.db`
dan mengimpornya kembali (misalnya untuk pindah komputer). Sebelum impor, data
saat ini disalin ke `koperasi-sebelum-impor-*.db` di folder yang sama.

## Konfigurasi perusahaan

Untuk menetapkan perusahaan dan menyembunyikan pilihan perusahaan di toolbar,
buat `.env.local` di root project:

```dotenv
VITE_COMPANY_ID=default
```

Gunakan ID perusahaan yang tersedia di database. Jika tidak diisi, perusahaan
dapat dipilih melalui toolbar; ID yang tidak ditemukan menampilkan error
konfigurasi. Setelah mengubah nilai ini, restart proses dev atau build ulang.

## Build dan pengujian

```bash
bun run build
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
```

- Pengujian desktop: `bun run test:e2e` pada Linux/Windows. Lihat [panduan E2E](e2e/README.md).
- Pengujian laporan: `bun run test:virtualization`. Lihat [panduan pengujian laporan](docs/pengujian-laporan.md).

## Kompatibilitas desktop

Target aplikasi adalah Windows 7 64-bit dengan WebView2 Fixed Runtime
`88.0.705.81` yang dibundel, Rust `1.77.2`, dan output frontend untuk Chromium 88.
Jangan menaikkan versi WebView2, Rust, atau Tauri secara independen tanpa pengujian
ulang pada Windows 7. CI memverifikasi ketersediaan fixed runtime sebelum packaging.

Detail pembagian frontend/backend dan transaksi tersedia di
[panduan arsitektur](docs/arsitektur.md).
