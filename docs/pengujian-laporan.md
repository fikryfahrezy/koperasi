# Virtualisasi dan pengujian laporan

## Implementasi

Simpanan dan Pinjaman menggunakan `@tanstack/vue-virtual` versi `3.13.39`
(`virtual-core` versi `3.17.11` dalam lockfile). Hanya baris yang terlihat serta
lima baris tambahan pada setiap sisi yang dipasang ke DOM. Total dihitung dari
seluruh laporan yang telah difilter. Judul bagian dan subtotal pinjaman ikut
menjadi bagian daftar baris virtual.

Kolom bulanan divirtualisasi sebagai kelompok bulan utuh agar struktur header
gabungan empat baris tetap terjaga. Kolom identitas dan metadata pinjaman tetap
dipasang, dengan dua kolom pertama dibekukan. Kolom pengisi kosong mempertahankan
lebar gulir penuh tanpa memasang bulan di luar area tampilan.

Tinggi baris tetap 32px dan lebar kolom ditetapkan secara eksplisit agar tata letak
tabel tidak berubah saat baris memasuki area tampilan. Navigasi tombol panah
menggunakan indeks baris dan kolom logis serta merender tujuan di luar area
tampilan sebelum memindahkan fokus.

## Pengujian browser

```bash
bun run test:virtualization
```

Chrome harus tersedia, atau tetapkan `CHROME_BINARY` ke executable Chrome/Chromium.
Pengujian ini menjalankan server Vite dan profil browser sementara yang terpisah,
lalu mengganti command Tauri dengan data sintetis: 1.000 anggota, 800 pinjaman,
dan 12 bulan. Pengujian tidak mengakses database aplikasi native.

Pemeriksaan mencakup jumlah baris DOM yang dibatasi, tinggi baris, rentang bulan
horizontal, keselarasan header/data/total, total, filter, navigasi keyboard, batas
bagian, dan pemulihan posisi gulir tab.

Untuk regresi header, jalankan:

```bash
node tests/sticky-headers.browser.mjs
```

Pengujian ini memeriksa keempat halaman tabel saat menggulir, mengubah tinggi baris
header, dan mengubah ukuran jendela, termasuk kolom beku pada Simpanan dan
Pinjaman. Setiap sel header menggunakan posisi sticky dengan offset baris yang
diukur untuk mendukung Chromium 88.

## Pemeriksaan sebelum rilis

Build produksi tetap menargetkan Chromium 88. Event native `scrollend`
dinonaktifkan; TanStack menggunakan fallback timer. Pengujian browser berjalan
pada browser lokal dan tidak menjamin kompatibilitas Windows 7.

Sebelum rilis, uji aplikasi yang telah dikemas pada Windows 7 x64 dengan WebView2
`88.0.705.81` yang dibundel:

1. Gulir kedua laporan hingga bagian bawah dan melintasi bulan.
2. Gunakan tombol panah untuk berpindah ke baris di luar area tampilan dan
   melintasi kedua kelompok pinjaman.
3. Terapkan filter saat posisi gulir berada di bawah.
4. Beralih antara dua tab dengan filter dan posisi gulir yang berbeda.
5. Pastikan total benar, tidak ada area kosong, dan tidak ada error runtime.

Jangan menaikkan versi fixed runtime untuk fitur ini.

Kembali ke [README](../README.md).
