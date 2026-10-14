# Arsitektur aplikasi

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

Kembali ke [README](../README.md).
