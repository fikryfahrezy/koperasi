-- Sejak 040, isian Neraca Dana Sosial dan Dana PMDK berarti saldo 1 Januari, dan pemakaiannya
-- diambil dari kategori Buku Kas (Dansos mengurangi Dana Sosial, Dekopinda mengurangi Dana PMDK).
-- Isian 2025 masih berisi saldo 31-12-2025 dari sheet (4.232.995 dan 6.494.387), jadi Dansos
-- 315.000 dan Dekopinda 1.745.000 tahun 2025 terpotong dua kali di kolom 31 Desember 2025.
-- Skrip ini menambahkan pemakaian 2025 itu ke isian 2025, supaya Neraca 31-12-2025 tetap sama
-- dengan sheet. Hanya diubah kalau isiannya masih angka sheet; database tanpa data 2025 tidak
-- berubah.
CREATE TEMP TABLE saldo041 AS
SELECT b.item_key, b.amount AS lama, b.amount + COALESCE((
  SELECT SUM(c.amount)
  FROM transaction_components c
  JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id
  WHERE t.company_id = 'default'
    AND t.channel = 'KAS'
    AND t.business_date BETWEEN '2025-01-01' AND '2025-12-31'
    AND t.reversed_transaction_id IS NULL
    AND NOT EXISTS (
      SELECT 1 FROM transactions r
      WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id
    )
    AND c.component_type = CASE b.item_key WHEN 'SOCIAL_FUND' THEN 'DANSOS' ELSE 'DEKOPINDA' END
), 0) AS baru
FROM balance_sheet_items b
WHERE b.company_id = 'default' AND b.year = 2025
  AND ((b.item_key = 'SOCIAL_FUND' AND b.amount = 4232995)
    OR (b.item_key = 'PMDK_FUND' AND b.amount = 6494387));

DELETE FROM saldo041 WHERE baru = lama;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT 'default', 'BALANCE_SHEET', '2025:' || item_key, 'UPDATED',
  json_object('amount', lama), json_object('amount', baru), 'Data script 041_saldo_awal_dana_2025'
FROM saldo041;

UPDATE balance_sheet_items
SET amount = (SELECT baru FROM saldo041 s WHERE s.item_key = balance_sheet_items.item_key)
WHERE company_id = 'default' AND year = 2025 AND item_key IN (SELECT item_key FROM saldo041);

DROP TABLE saldo041;
