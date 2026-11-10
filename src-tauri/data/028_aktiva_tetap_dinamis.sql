-- Aktiva Tetap jadi daftar pos per tahun buku (balance_sheet_details, item FIXED_ASSETS).
-- Nilai lama dari isian Meja, Kursi, dan Lap-Top dipindahkan ke daftar itu. Nilai 1 di Lap-Top
-- berasal dari Neraca RAT (Perlengkapan Kantor = 1), jadi diberi nama itu. Tahun yang sudah punya
-- daftar Aktiva Tetap tidak disentuh. Pos Aktiva Tetap di neraca = jumlah daftar.
CREATE TEMP TABLE fa028 AS
SELECT i.company_id, i.year,
       COALESCE(MAX(CASE WHEN i.item_key = 'FIXED_ASSETS_DESK' THEN i.amount END), 0) AS desk,
       COALESCE(MAX(CASE WHEN i.item_key = 'FIXED_ASSETS_CHAIR' THEN i.amount END), 0) AS chair,
       COALESCE(MAX(CASE WHEN i.item_key = 'FIXED_ASSETS' THEN i.amount END), 0) AS laptop
FROM balance_sheet_items i
WHERE i.item_key IN ('FIXED_ASSETS_DESK', 'FIXED_ASSETS_CHAIR', 'FIXED_ASSETS')
  AND NOT EXISTS (SELECT 1 FROM balance_sheet_details d WHERE d.company_id = i.company_id AND d.year = i.year AND d.item_key = 'FIXED_ASSETS')
GROUP BY i.company_id, i.year;
INSERT INTO balance_sheet_details (company_id, year, item_key, position, name, amount, loan_date, note)
SELECT company_id, year, 'FIXED_ASSETS', 0, CASE WHEN laptop = 1 THEN 'Perlengkapan Kantor' ELSE 'Lap-Top' END, laptop, NULL, '' FROM fa028 WHERE laptop <> 0
UNION ALL SELECT company_id, year, 'FIXED_ASSETS', 1, 'Meja', desk, NULL, '' FROM fa028 WHERE desk <> 0
UNION ALL SELECT company_id, year, 'FIXED_ASSETS', 2, 'Kursi', chair, NULL, '' FROM fa028 WHERE chair <> 0;
UPDATE balance_sheet_items
SET amount = (SELECT f.laptop + f.desk + f.chair FROM fa028 f WHERE f.company_id = balance_sheet_items.company_id AND f.year = balance_sheet_items.year),
    updated_at = CURRENT_TIMESTAMP
WHERE item_key = 'FIXED_ASSETS'
  AND EXISTS (SELECT 1 FROM fa028 f WHERE f.company_id = balance_sheet_items.company_id AND f.year = balance_sheet_items.year);
UPDATE balance_sheet_items SET amount = 0, updated_at = CURRENT_TIMESTAMP
WHERE item_key IN ('FIXED_ASSETS_DESK', 'FIXED_ASSETS_CHAIR')
  AND EXISTS (SELECT 1 FROM fa028 f WHERE f.company_id = balance_sheet_items.company_id AND f.year = balance_sheet_items.year);
DROP TABLE fa028;
