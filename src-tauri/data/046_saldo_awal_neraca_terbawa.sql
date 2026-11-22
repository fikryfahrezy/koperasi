-- Sejak versi ini, dana, cadangan, laba ditahan, dan hutang KOWABRI / PP di Neraca dimulai dari
-- Neraca 31 Desember tahun lalu: saldo 1 Januari = jumlah isian semua tahun buku sebelumnya +
-- mutasi kategori Buku Kas sebelum 1 Januari. Isian tahun itu sekarang berarti tambahan saat RAT
-- (bagian SHU), bukan lagi saldo 1 Januari.
-- Skrip ini mengubah isian lama (saldo 1 Januari) menjadi tambahan, sehingga semua angka Neraca
-- tetap sama:
--   tambahan tahun Y = saldo 1 Januari Y - saldo 1 Januari tahun isian sebelumnya
--                      - mutasi kategori di antara keduanya.
-- Tahun isian pertama tetap (dikurangi mutasi sebelum tahun itu, biasanya 0).
-- Di database bendahara: 2025 tetap; 2026 Dana Pendidikan, Dana Sosial, Dana PMDK, Cadangan
-- Modal, Cadangan RAT, Cadangan Penghapusan, Hutang KOWABRI / PP menjadi 0 (sama dengan 2025),
-- Laba ditahan 2026 tetap 7.591.369 (2025 kosong).
-- Setiap isian yang berubah dicatat di audit_events.

-- Kategori Buku Kas yang menggerakkan tiap pos (sama dengan `fundUses` di src/balance-sheet.ts).
CREATE TEMP TABLE gerak046 (item_key TEXT NOT NULL, component_type TEXT NOT NULL, sign INTEGER NOT NULL);
INSERT INTO gerak046 (item_key, component_type, sign) VALUES
  ('DEBT_PPBRI', 'TERIMA_HUTANG', 1),
  ('DEBT_PPBRI', 'CICIL_HUTANG', -1),
  ('EDUCATION_FUND', 'PAKAI_DANA_PENDIDIKAN', -1),
  ('SOCIAL_FUND', 'DANSOS', -1),
  ('PMDK_FUND', 'PAKAI_DANA_PMDK', -1),
  ('PMDK_FUND', 'DEKOPINDA', -1),
  ('RAT_RESERVE', 'PAKAI_CADANGAN_RAT', -1),
  ('RETAINED_EARNINGS', 'PAKAI_LABA_DITAHAN', -1);

CREATE TEMP TABLE isian046 AS
SELECT company_id, item_key, year, amount,
  LAG(year) OVER (PARTITION BY company_id, item_key ORDER BY year) AS prev_year,
  COALESCE(LAG(amount) OVER (PARTITION BY company_id, item_key ORDER BY year), 0) AS prev_amount
FROM balance_sheet_items
WHERE item_key IN ('DEBT_PPBRI', 'CAPITAL_RESERVE', 'EDUCATION_FUND', 'SOCIAL_FUND', 'PMDK_FUND',
                   'RAT_RESERVE', 'LOAN_WRITE_OFF_RESERVE', 'RETAINED_EARNINGS');

CREATE TEMP TABLE baru046 AS
SELECT i.company_id, i.item_key, i.year, i.amount AS old_amount,
  i.amount - i.prev_amount - COALESCE((
    SELECT SUM(g.sign * c.amount)
    FROM effective_components c
    JOIN gerak046 g ON g.component_type = c.component_type
    WHERE c.company_id = i.company_id AND g.item_key = i.item_key AND c.channel = 'KAS'
      AND c.business_date < printf('%04d-01-01', i.year)
      AND (i.prev_year IS NULL OR c.business_date >= printf('%04d-01-01', i.prev_year))
  ), 0) AS new_amount
FROM isian046 i;

DELETE FROM baru046 WHERE new_amount = old_amount;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT company_id, 'BALANCE_SHEET', year || ':' || item_key, 'UPDATED',
  json_object('amount', old_amount),
  json_object('amount', new_amount, 'note', 'Saldo 1 Januari menjadi tambahan saat RAT'),
  'Data script 046_saldo_awal_neraca_terbawa'
FROM baru046;

UPDATE balance_sheet_items
SET amount = (
  SELECT b.new_amount FROM baru046 b
  WHERE b.company_id = balance_sheet_items.company_id AND b.item_key = balance_sheet_items.item_key
    AND b.year = balance_sheet_items.year
)
WHERE EXISTS (
  SELECT 1 FROM baru046 b
  WHERE b.company_id = balance_sheet_items.company_id AND b.item_key = balance_sheet_items.item_key
    AND b.year = balance_sheet_items.year
);

DROP TABLE gerak046;
DROP TABLE isian046;
DROP TABLE baru046;
