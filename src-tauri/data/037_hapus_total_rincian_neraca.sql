-- Pos Neraca yang berupa daftar rincian (Piutang Lainnya, Aktiva Tetap, pos
-- tambahan "+ Pos") dulu juga menyimpan totalnya di balance_sheet_items setiap kali
-- rinciannya disimpan. Sekarang totalnya dihitung dari rincian, jadi baris total itu
-- dihapus. Skrip ini gagal tanpa mengubah apa pun kalau ada total yang tidak sama
-- dengan jumlah rinciannya (nilainya bisa hilang).

CREATE TEMP TABLE list_totals037 AS
SELECT
  i.company_id,
  i.year,
  i.item_key,
  i.amount,
  COALESCE((
    SELECT SUM(d.amount) FROM balance_sheet_details d
    WHERE d.company_id = i.company_id AND d.year = i.year AND d.item_key = i.item_key
  ), 0) AS details_total
FROM balance_sheet_items i
WHERE i.item_key IN (
  'OTHER_LOANS', 'FIXED_ASSETS', 'EXTRA_CURRENT_ASSETS', 'EXTRA_CURRENT_LIABILITIES',
  'EXTRA_LONG_TERM_LIABILITIES', 'EXTRA_EQUITY'
);

CREATE TEMP TABLE list_totals037_guard (mismatched INTEGER CHECK (mismatched = 0));
INSERT INTO list_totals037_guard
SELECT COUNT(*) FROM list_totals037 WHERE amount <> details_total;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT company_id, 'BALANCE_SHEET', CAST(year AS TEXT), 'UPDATED',
  json_object('items', json_group_object(item_key, amount)),
  json_object('items', json_object()),
  'Data script 037_hapus_total_rincian_neraca'
FROM list_totals037
GROUP BY company_id, year;

DELETE FROM balance_sheet_items
WHERE EXISTS (
  SELECT 1 FROM list_totals037 t
  WHERE t.company_id = balance_sheet_items.company_id
    AND t.year = balance_sheet_items.year
    AND t.item_key = balance_sheet_items.item_key
);

DROP TABLE list_totals037_guard;
DROP TABLE list_totals037;
