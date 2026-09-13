# Koperasi Bina Sejahtera

Aplikasi operasional desktop berbasis Vue 3 + Tauri untuk mengelola anggota,
simpanan, pinjaman, pembayaran anggota, buku kas, rekonsiliasi, migrasi Excel, dan
parameter administrasi.

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

Untuk validasi frontend produksi dan test backend:

```bash
pnpm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
```

Database lokal dibuat otomatis pada startup pertama. Di Windows lokasinya berada
di `%APPDATA%\com.fikryfahrezy.koperasi\koperasi-v2.db`.

## Arsitektur transaksi

- Vue hanya menangani form dan read model tampilan.
- Command Tauri di Rust memvalidasi transaksi dan periode.
- SQLite menyimpan anggota, rekening simpanan, kontrak pinjaman, komponen
  transaksi, posting kas, parameter berversi, dan audit event.
- Setoran/penarikan simpanan, pencairan + provisi, pembayaran split, serta
  reversal memakai database transaction agar seluruh posting commit atau
  rollback bersama-sama.

Aplikasi langsung membuka dashboard operasional dan tidak memerlukan
proses login. Seluruh mutasi operasional disimpan oleh backend Rust ke database
SQLite lokal di direktori data aplikasi; frontend Vue hanya memanggil command
Tauri dan menampilkan snapshot ledger. Posting pembayaran dan reversal dijalankan
secara atomik serta direkam pada audit trail. Data awal dan angka rekonsiliasi
mengacu pada workbook 2026 di folder `docs/`.
