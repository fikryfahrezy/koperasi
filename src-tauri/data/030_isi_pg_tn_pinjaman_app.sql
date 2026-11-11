-- Kolom PG/TN pinjaman yang dibuat lewat form Pinjaman di aplikasi (id LOAN-...).
-- Form itu dulu belum punya isian PG/TN, jadi kolomnya kosong. Nilai diambil dari
-- kolom F sheet PINJAMAN BULANAN 2026 (versi 10-11-2026), baris di komentar.
-- Hanya mengisi yang masih kosong.
CREATE TEMP TABLE guarantee030 (loan_id TEXT PRIMARY KEY, guarantee TEXT NOT NULL);
INSERT INTO guarantee030 (loan_id, guarantee) VALUES
  ('LOAN-1792331033333-7', 'PG'),   -- baris 88, realisasi 02-09-2026
  ('LOAN-1792499786484-1', 'PG'),   -- baris 93, 20 jt 14-09-2026
  ('LOAN-1792500314370-9', 'PG'),   -- baris 94, 15-09-2026
  ('LOAN-1792501659029-16', 'TN'),  -- baris 95, 22-09-2026
  ('LOAN-1792502079384-20', 'PG'),  -- baris 96, 22-09-2026
  ('LOAN-1791684006108-0', 'PG'),   -- baris 97, 05-10-2026
  ('LOAN-1791616070695-4', 'PG'),   -- baris 99, 06-10-2026
  ('LOAN-1792467992586-4', 'PG');   -- baris 99, 06-10-2026 (tercatat dua kali)
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT l.company_id, 'LOAN', l.id, 'UPDATED', json_object('guarantee', g.guarantee), 'Data script 030_isi_pg_tn_pinjaman_app'
FROM loans l JOIN guarantee030 g ON g.loan_id = l.id
WHERE l.company_id = 'default' AND COALESCE(TRIM(l.guarantee), '') = '';
UPDATE loans
SET guarantee = (SELECT g.guarantee FROM guarantee030 g WHERE g.loan_id = loans.id)
WHERE company_id = 'default' AND COALESCE(TRIM(guarantee), '') = ''
  AND id IN (SELECT loan_id FROM guarantee030);
DROP TABLE guarantee030;
