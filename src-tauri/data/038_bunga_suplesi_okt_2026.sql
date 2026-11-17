-- Pinjaman suplesi 25.000.000 (realisasi 06-10-2026, 25 bulan, PG) dicatat dengan bunga
-- 24%/tahun menurun, mengikuti sheet DATA PINJAMAN TAHUN 2026 sebelum 10-11-2026. Sheet
-- terbaru (PINJAMAN BULANAN) sudah diperbaiki menjadi 18%/tahun (1,5%/bulan) Plat:
-- angsuran 1.000.000 pokok + 375.000 bunga. Belum ada setoran untuk pinjaman ini, jadi
-- hanya data pinjamannya yang diubah; provisi 30.000 tetap.
-- Gagal tanpa mengubah apa pun kalau pinjamannya tidak ada.
CREATE TEMP TABLE loan_rate038_guard (found INTEGER CHECK (found = 1));
INSERT INTO loan_rate038_guard
SELECT COUNT(*) FROM loans
WHERE company_id = 'default' AND id = 'default-SYNC017-L102';

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT
  company_id,
  'LOAN',
  id,
  'UPDATED',
  json_object('rate_annual', rate_annual, 'interest_type', interest_type),
  json_object('rate_annual', 18.0, 'interest_type', 'Plat'),
  'Data script 038_bunga_suplesi_okt_2026'
FROM loans
WHERE company_id = 'default' AND id = 'default-SYNC017-L102'
  AND (rate_annual <> 18.0 OR interest_type <> 'Plat');

UPDATE loans
SET rate_annual = 18.0, interest_type = 'Plat'
WHERE company_id = 'default' AND id = 'default-SYNC017-L102'
  AND (rate_annual <> 18.0 OR interest_type <> 'Plat');

DROP TABLE loan_rate038_guard;
