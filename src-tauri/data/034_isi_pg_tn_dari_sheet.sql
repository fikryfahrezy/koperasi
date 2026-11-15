-- PG/TN yang masih kosong di database, dicek ulang ke kolom F sheet DATA PINJAMAN
-- TAHUN 2026 (PINJAMAN BULANAN, PINJAMAN SEMENTARA, SALDO PERBULAN, dll.) pada 15-11-2026.
-- Dari 68 pinjaman yang kosong, hanya satu yang di sheet berisi PG/TN; sisanya di sheet
-- juga kosong (pinjaman lama dan Non Anggota), dan 5 pinjaman November belum ada di sheet.
-- Sekalian menyeragamkan 'pg' (huruf kecil, dari impor) menjadi 'PG'.
-- Hanya mengubah yang masih kosong / huruf kecil. Sudah dijalankan langsung ke database
-- 15-11-2026 (tercatat di applied_data_scripts), jadi aplikasi melewatinya saat start.
CREATE TEMP TABLE guarantee034 (loan_id TEXT PRIMARY KEY, guarantee TEXT NOT NULL);
INSERT INTO guarantee034 (loan_id, guarantee) VALUES
  ('LOAN-1793157950385-0', 'PG');   -- PINJAMAN BULANAN baris 105, 500.000, 28-10-2026
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT l.company_id, 'LOAN', l.id, 'UPDATED', json_object('guarantee', g.guarantee), 'Data script 034_isi_pg_tn_dari_sheet'
FROM loans l JOIN guarantee034 g ON g.loan_id = l.id
WHERE l.company_id = 'default' AND COALESCE(TRIM(l.guarantee), '') = '';
UPDATE loans
SET guarantee = (SELECT g.guarantee FROM guarantee034 g WHERE g.loan_id = loans.id)
WHERE company_id = 'default' AND COALESCE(TRIM(guarantee), '') = ''
  AND id IN (SELECT loan_id FROM guarantee034);
DROP TABLE guarantee034;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT company_id, 'LOAN', id, 'UPDATED', json_object('guarantee', UPPER(TRIM(guarantee))), 'Data script 034_isi_pg_tn_dari_sheet'
FROM loans WHERE TRIM(guarantee) IN ('pg', 'tn', 'Pg', 'Tn');
UPDATE loans SET guarantee = UPPER(TRIM(guarantee)) WHERE TRIM(guarantee) IN ('pg', 'tn', 'Pg', 'Tn');
