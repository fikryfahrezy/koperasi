-- Otomasi Neraca dan Laba Rugi 2026: baris Buku Kas 2026 yang tercatat tanpa kategori (atau
-- dengan kategori yang tidak sesuai LABA RUGI / NERACA PENYESUAIAN sheet BUKU KAS HARIAN
-- TAHUN 2026) diberi kategori, lalu isian Neraca 2026 dijadikan saldo 1 Januari.
--
-- 1. Kategori komponen Kas diubah di tempat (jumlah, tanggal, uraian tetap; Saldo Kas tidak
--    berubah). Setiap perubahan dicatat di audit_events. Hanya baris Kas koperasi (tanpa
--    anggota) tahun 2026 yang belum dibalik, dicocokkan per uraian dan arah:
--    - Biaya: honor + pulsa tiap awal bulan -> Biaya Pengurus; Biaya Humas (Adm BRI Cabang)
--      -> Humas; ATK/spidol/kertas -> Biaya ATK; servis printer, charger laptop, program
--      koperasi -> Pemeliharaan AT; rapat membahas RAT -> Humas; remunerasi -> Transfortasi;
--      BG talangan PP BRI -> Biaya Bunga.
--    - Neraca: biaya RAT 2025 -> Pakai Cadangan RAT; parsel/iuran -> Dansos (Dana Sosial),
--      Pakai Dana Pendidikan, Pakai Dana PMDK; HUT koperasi -> Pakai Dana PMDK; parsel dan
--      piknik dari laba ditahan -> Pakai Laba Ditahan; SHU anggota, dana pengurus, dana
--      pengawas 2025 -> Bagi SHU; cash bond -> Bayar Kas Bon; pelunasan dana talangan PP BRI
--      -> Cicil Hutang.
-- 2. Isian Neraca 2026 Dana Pendidikan, Dana Sosial, Dana PMDK, Cadangan RAT menjadi saldo
--    Neraca 31-12-2025 (pemakaiannya kini dari kategori di atas). Hanya diubah kalau masih
--    berisi angka lama yang sudah dikurangi parsel (3.460.622, 1.232.995, 3.494.387, kosong).
--    Hutang KOWABRI / PP 10.000.000 dan Laba ditahan 7.591.369 sudah saldo 1 Januari.
-- Database baru (instalasi baru, e2e) tidak punya baris ini, jadi skrip ini tidak mengubah
-- apa pun di sana.

CREATE TEMP TABLE kategori040 (
  pola TEXT NOT NULL,
  arah TEXT NOT NULL,
  dari TEXT NOT NULL,
  ke TEXT NOT NULL,
  label TEXT NOT NULL
);

INSERT INTO kategori040 (pola, arah, dari, ke, label) VALUES
  ('Biaya Honor Karyawan Koperasi%', 'Keluar', 'CASH', 'BIAYA_PENGURUS', 'Biaya Pengurus'),
  ('Biaya Pulsa Karyawan Koperasi%', 'Keluar', 'CASH', 'BIAYA_PENGURUS', 'Biaya Pengurus'),
  ('Biaya Humas ( Adm BRI Cabang )', 'Keluar', 'CASH', 'HUMAS', 'Humas'),
  ('Pembelian ATK', 'Keluar', 'CASH', 'BIAYA_ATK', 'Biaya ATK'),
  ('Pembelian Spidol%', 'Keluar', 'CASH', 'BIAYA_ATK', 'Biaya ATK'),
  ('Pembelian kertas HVS%', 'Keluar', 'CASH', 'BIAYA_ATK', 'Biaya ATK'),
  ('Biaya Servis Printer%', 'Keluar', 'CASH', 'PEMELIHARAAN_AT', 'Pemeliharaan AT'),
  ('Pembelian Charge Laptop%', 'Keluar', 'CASH', 'PEMELIHARAAN_AT', 'Pemeliharaan AT'),
  ('Pembelian Program Koprasi', 'Keluar', 'LAINNYA', 'PEMELIHARAAN_AT', 'Pemeliharaan AT'),
  ('Rapat membahas RAT', 'Keluar', 'LAINNYA', 'HUMAS', 'Humas'),
  ('Remunerasi %', 'Keluar', 'CASH', 'TRANSFORTASI', 'Transfortasi'),
  ('Bayar BG Talangan PP BRI%', 'Keluar', 'CASH', 'BIAYA_BUNGA', 'Biaya Bunga'),
  ('PENGELUARAN BIAYA RAT TAHUN BUKU 2025', 'Keluar', 'CASH', 'PAKAI_CADANGAN_RAT', 'Pakai Cadangan RAT'),
  ('Pengambilan Dansos u/ Parcel', 'Keluar', 'CASH', 'DANSOS', 'Dansos'),
  ('Setoran Iuran Anggota', 'Keluar', 'CASH', 'DANSOS', 'Dansos'),
  ('Humas Dekopinda', 'Keluar', 'HUMAS', 'DANSOS', 'Dansos'),
  ('Pengambilan Dana Pendidikan u/ Parcel', 'Keluar', 'CASH', 'PAKAI_DANA_PENDIDIKAN', 'Pakai Dana Pendidikan'),
  ('Setoran Dana Pendidikan', 'Keluar', 'CASH', 'PAKAI_DANA_PENDIDIKAN', 'Pakai Dana Pendidikan'),
  ('Pengambilan Dana PMDK u/ Parcel', 'Keluar', 'CASH', 'PAKAI_DANA_PMDK', 'Pakai Dana PMDK'),
  ('Setoran Dana Pembangunan Daerah Kerja', 'Keluar', 'CASH', 'PAKAI_DANA_PMDK', 'Pakai Dana PMDK'),
  ('Pembayaran HUT KOPERASI TAHUN 2026', 'Keluar', 'CASH', 'PAKAI_DANA_PMDK', 'Pakai Dana PMDK'),
  ('Pengambilan Cadangan  Laba Thn 2025 u/Parcel', 'Keluar', 'CASH', 'PAKAI_LABA_DITAHAN', 'Pakai Laba Ditahan'),
  ('Laba yang ditahan (Sumb panitia Piknik)', 'Keluar', 'CASH', 'PAKAI_LABA_DITAHAN', 'Pakai Laba Ditahan'),
  ('PENGELUARAN SHU TAHUN BUKU 2025', 'Keluar', 'CASH', 'BAGI_SHU', 'Bagi SHU'),
  ('DANA PENGURUS 10 % x Rp. 67.639.098', 'Keluar', 'CASH', 'BAGI_SHU', 'Bagi SHU'),
  ('DANA PENGAWAS Rp. 67.639.098 x 5 %', 'Keluar', 'CASH', 'BAGI_SHU', 'Bagi SHU'),
  ('%(Cash Bond)', 'Masuk', 'CASH', 'BAYAR_KAS_BON', 'Bayar Kas Bon'),
  ('Pelunasan Cash Bond %', 'Masuk', 'CASH', 'BAYAR_KAS_BON', 'Bayar Kas Bon'),
  ('PELUNASAN DANA TALANGAN PP BRI', 'Keluar', 'CASH', 'CICIL_HUTANG', 'Cicil Hutang KOWABRI / PP');

-- Komponen yang diubah: satu baris per komponen, aturan pertama yang cocok.
CREATE TEMP TABLE ubah040 AS
SELECT c.id, c.transaction_id, c.component_type AS dari, c.label AS label_lama, k.ke, k.label
FROM transaction_components c
JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id
JOIN kategori040 k
  ON k.rowid = (
    SELECT k2.rowid FROM kategori040 k2
    WHERE t.description LIKE k2.pola AND t.direction = k2.arah AND c.component_type = k2.dari
    ORDER BY k2.rowid LIMIT 1
  )
WHERE t.company_id = 'default'
  AND t.channel = 'KAS'
  AND t.member_id IS NULL
  AND t.business_date BETWEEN '2026-01-01' AND '2026-12-31'
  AND t.reversed_transaction_id IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM transactions r
    WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id
  );

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT
  'default',
  'TRANSACTION',
  transaction_id,
  'UPDATED',
  json_object('component_id', id, 'component_type', dari, 'label', label_lama),
  json_object('component_id', id, 'component_type', ke, 'label', label),
  'Data script 040_kategori_neraca_2026'
FROM ubah040;

DROP TRIGGER components_append_only_update;

UPDATE transaction_components
SET
  component_type = (SELECT ke FROM ubah040 u WHERE u.id = transaction_components.id),
  label = (SELECT label FROM ubah040 u WHERE u.id = transaction_components.id)
WHERE id IN (SELECT id FROM ubah040);

CREATE TRIGGER components_append_only_update
BEFORE UPDATE ON transaction_components
BEGIN
  SELECT RAISE(ABORT, 'Komponen transaksi tidak dapat diubah.');
END;

-- Isian Neraca 2026 menjadi saldo 1 Januari (= Neraca 31-12-2025).
CREATE TEMP TABLE saldo040 (item_key TEXT NOT NULL, lama INTEGER, baru INTEGER NOT NULL);

INSERT INTO saldo040 (item_key, lama, baru) VALUES
  ('EDUCATION_FUND', 3460622, 6460622),
  ('SOCIAL_FUND', 1232995, 4232995),
  ('PMDK_FUND', 3494387, 6494387),
  ('RAT_RESERVE', NULL, 20000000);

-- Hanya untuk database hasil impor sheet (ada baris Kas yang diubah di atas).
DELETE FROM saldo040
WHERE NOT EXISTS (SELECT 1 FROM ubah040)
  OR COALESCE((
    SELECT amount FROM balance_sheet_items b
    WHERE b.company_id = 'default' AND b.year = 2026 AND b.item_key = saldo040.item_key
  ), -1) <> COALESCE(lama, -1);

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT
  'default',
  'BALANCE_SHEET',
  '2026:' || item_key,
  'UPDATED',
  json_object('amount', lama),
  json_object('amount', baru),
  'Data script 040_kategori_neraca_2026'
FROM saldo040;

UPDATE balance_sheet_items
SET amount = (SELECT baru FROM saldo040 s WHERE s.item_key = balance_sheet_items.item_key)
WHERE company_id = 'default' AND year = 2026
  AND item_key IN (SELECT item_key FROM saldo040 WHERE lama IS NOT NULL);

INSERT INTO balance_sheet_items (company_id, year, item_key, amount)
SELECT 'default', 2026, item_key, baru FROM saldo040 WHERE lama IS NULL;

DROP TABLE saldo040;
DROP TABLE ubah040;
DROP TABLE kategori040;
