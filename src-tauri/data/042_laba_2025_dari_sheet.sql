-- Laba 2025 tidak bisa dihitung dari aplikasi: pinjaman 2025 hasil migrasi mencatat setoran
-- pokok dan bunga setahun sekaligus di Desember 2025 (bunga 75.001.532, di DATA PINJAMAN 2025
-- sekitar 148 jt). Laba Tahun 2025 di Neraca 31-12-2025 diisi dari LABA RUGI sheet BUKU KAS
-- HARIAN TAHUN 2026 kolom 2025: 75.913.690 (isian NET_INCOME).
-- Baris Buku Kas 30-01-2025 "SEWA KANTOR" 140.000.000 berkategori Sewa Kantor, padahal sewa
-- kantor 2025 di BIAYA PENGELUARAN hanya 1.600.000; kategorinya dilepas (jadi mutasi kas biasa).
-- Saldo Kas tidak berubah. Hanya untuk database hasil impor sheet (ada Neraca 2025 dari sheet).
CREATE TEMP TABLE laba042 AS
SELECT 75913690 AS amount
WHERE EXISTS (
  SELECT 1 FROM balance_sheet_items
  WHERE company_id = 'default' AND year = 2025 AND item_key = 'CAPITAL_RESERVE' AND amount = 42991150
)
AND NOT EXISTS (
  SELECT 1 FROM balance_sheet_items
  WHERE company_id = 'default' AND year = 2025 AND item_key = 'NET_INCOME'
);

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT 'default', 'BALANCE_SHEET', '2025:NET_INCOME', 'CREATED', NULL,
  json_object('amount', amount), 'Data script 042_laba_2025_dari_sheet'
FROM laba042;

INSERT INTO balance_sheet_items (company_id, year, item_key, amount)
SELECT 'default', 2025, 'NET_INCOME', amount FROM laba042;

CREATE TEMP TABLE sewa042 AS
SELECT c.id, c.transaction_id
FROM transaction_components c
JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id
WHERE t.company_id = 'default'
  AND t.channel = 'KAS'
  AND t.business_date = '2025-01-30'
  AND t.description = 'SEWA KANTOR'
  AND t.direction = 'Keluar'
  AND c.component_type = 'SEWA_KANTOR'
  AND c.amount = 140000000;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT 'default', 'TRANSACTION', transaction_id, 'UPDATED',
  json_object('component_id', id, 'component_type', 'SEWA_KANTOR'),
  json_object('component_id', id, 'component_type', 'CASH'),
  'Data script 042_laba_2025_dari_sheet'
FROM sewa042;

DROP TRIGGER components_append_only_update;

UPDATE transaction_components
SET component_type = 'CASH', label = 'Mutasi kas'
WHERE id IN (SELECT id FROM sewa042);

CREATE TRIGGER components_append_only_update
BEFORE UPDATE ON transaction_components
BEGIN
  SELECT RAISE(ABORT, 'Komponen transaksi tidak dapat diubah.');
END;

DROP TABLE sewa042;
DROP TABLE laba042;
