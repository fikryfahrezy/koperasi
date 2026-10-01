-- Tabel riwayat bulanan sempat dibuat pada build pengembangan sebelum riwayat
-- disimpan sebagai transaksi. Riwayat kini diturunkan dari transaksi, jadi
-- tabel tersebut tidak dipakai lagi.
DROP TABLE IF EXISTS savings_monthly;
DROP TABLE IF EXISTS loan_monthly;
