-- Rincian LABA RUGI 2025: transaksi 2025 sudah dihapus (043), jadi Pendapatan dan Biaya 2025
-- tidak bisa dihitung lagi dan halaman Laba Rugi hanya menampilkan Laba 2025 (NET_INCOME, 042).
-- Skrip ini menyimpan rincian kolom 2025 sheet LABA RUGI (BUKU KAS HARIAN TAHUN 2026) sebagai
-- isian Neraca 2025: INCOME_* untuk pendapatan, EXPENSE_<kategori Buku Kas> untuk biaya.
-- Baris yang 0 (Pendapatan lainnya, Penghapusan Pinjaman, Pemeliharaan AT) tidak disimpan.
--   Bunga 147.325.870 + Provisi 7.390.960 - Biaya 78.803.140 = Laba 75.913.690 (= NET_INCOME).
-- Hanya untuk database dengan Laba 2025 dari sheet (NET_INCOME 2025 = 75.913.690) yang belum
-- punya rincian ini; selain itu (instalasi baru, e2e) tidak ada yang diubah.
CREATE TEMP TABLE rincian045 (item_key TEXT PRIMARY KEY, amount INTEGER NOT NULL);

INSERT INTO rincian045 (item_key, amount) VALUES
  ('INCOME_INTEREST', 147325870),
  ('INCOME_PROVISION', 7390960),
  ('EXPENSE_BIAYA_PENGURUS', 44960000),
  ('EXPENSE_BIAYA_ATK', 867000),
  ('EXPENSE_TRANSFORTASI', 3050000),
  ('EXPENSE_LAINNYA', 2165000),
  ('EXPENSE_BIAYA_BUNGA', 3287040),
  ('EXPENSE_SEWA_KANTOR', 1600000),
  ('EXPENSE_BIAYA_RAT', 20000000),
  ('EXPENSE_HUMAS', 2874100);

DELETE FROM rincian045
WHERE NOT EXISTS (
    SELECT 1 FROM balance_sheet_items
    WHERE company_id = 'default' AND year = 2025 AND item_key = 'NET_INCOME' AND amount = 75913690
  )
  OR EXISTS (
    SELECT 1 FROM balance_sheet_items
    WHERE company_id = 'default' AND year = 2025
      AND (item_key LIKE 'INCOME\_%' ESCAPE '\' OR item_key LIKE 'EXPENSE\_%' ESCAPE '\')
  );

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT 'default', 'BALANCE_SHEET', '2025:' || item_key, 'CREATED', NULL,
  json_object('amount', amount), 'Data script 045_rincian_laba_rugi_2025'
FROM rincian045;

INSERT INTO balance_sheet_items (company_id, year, item_key, amount)
SELECT 'default', 2025, item_key, amount FROM rincian045;

DROP TABLE rincian045;
