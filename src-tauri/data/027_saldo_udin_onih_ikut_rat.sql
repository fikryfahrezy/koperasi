-- Membatalkan koreksi 019 untuk UDIN / LILIK (L202) dan ONIH / JAJA / US (L225): saldo akhir 2025
-- ikut sheet DATA PINJAMAN TAHUN 2026 (versi 10-11-2026, PINJAMAN SEMENTARA baris 24 dan 47) dan
-- Neraca RAT 31-12-2025 (Piutang Non Anggota 221.390.000). Keputusan Fahrezy 10-11-2026.
-- * L202: angsuran pokok 2025 450.000, saldo dan tunggakan pokok akhir 2025 1.850.000
--   (setoran 1.650.000 tgl 29-03-2026 tetap, sisa 200.000).
-- * L225: angsuran pokok 2025 1.300.000, saldo dan tunggakan pokok akhir 2025 700.000.
-- Semua NON_KAS, saldo Kas tidak berubah. Transaksi 019 dibalik, lalu dicatat ulang.
CREATE TEMP TABLE sync027_ok AS SELECT 1 AS ok WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE id LIKE 'default-SYNC027-%')
  AND (SELECT COUNT(*) FROM effective_transactions WHERE company_id = 'default' AND id IN ('default-SYNC019-PIN-L202-2025', 'default-SYNC019-PIN-L202-ARREARS-2026', 'default-SYNC019-PIN-L225-2025')) = 3;
CREATE TEMP TABLE sync027_rev (old_id TEXT, reason TEXT);
INSERT INTO sync027_rev VALUES
  ('default-SYNC019-PIN-L202-2025', 'Ikut sheet & Neraca RAT: angsuran pokok 2025 UDIN / LILIK 450.000'),
  ('default-SYNC019-PIN-L202-ARREARS-2026', 'Ikut sheet & Neraca RAT: tunggakan pokok awal 2026 UDIN / LILIK 1.850.000'),
  ('default-SYNC019-PIN-L225-2025', 'Ikut sheet & Neraca RAT: ONIH / JAJA / US saldo akhir 2025 700.000');
-- 1. Balik transaksi 019.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, reversed_transaction_id, actor)
SELECT 'default-SYNC027-REV-' || substr(t.id, 9), t.company_id, t.business_date, t.display_date, t.display_time, t.member_id, t.member_name, t.channel, 'Ikut Neraca RAT · ' || t.description, t.reference || '/REV/027', CASE t.direction WHEN 'Masuk' THEN 'Keluar' ELSE 'Masuk' END, t.amount, t.id, 'Data script 027_saldo_udin_onih_ikut_rat'
FROM transactions t JOIN sync027_rev r ON r.old_id = t.id WHERE t.company_id = 'default' AND EXISTS (SELECT 1 FROM sync027_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT c.company_id, 'default-SYNC027-REV-' || substr(c.transaction_id, 9), 'REVERSAL', 'Ikut Neraca RAT · ' || c.label, c.amount, c.loan_id, c.savings_account_id
FROM transaction_components c JOIN sync027_rev r ON r.old_id = c.transaction_id WHERE EXISTS (SELECT 1 FROM sync027_ok) ORDER BY c.id;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', r.old_id, 'REVERSED', json_object('reversalId', 'default-SYNC027-REV-' || substr(r.old_id, 9), 'reason', r.reason), 'Data script 027_saldo_udin_onih_ikut_rat'
FROM sync027_rev r WHERE EXISTS (SELECT 1 FROM sync027_ok);
-- 2. Catat angka sheet.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, actor) SELECT * FROM (VALUES
  ('default-SYNC027-PIN-L202-2025', 'default', '2025-12-31', '31-Dec-2025', '00:00', NULL, 'UDIN / LILIK', 'NON_KAS', 'Angsuran pinjaman 2025 (ringkasan tahunan, ikut Neraca RAT)', 'default-SYNC027-PIN-L202-2025', 'Masuk', 450000, 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default-SYNC027-PIN-L202-ARREARS-2026', 'default', '2026-01-01', '01-Jan-2026', '00:00', NULL, 'UDIN / LILIK', 'NON_KAS', 'Tunggakan awal 2026', 'default-SYNC027-PIN-L202-ARREARS-2026', 'Masuk', 1850000, 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default-SYNC027-PIN-L225-2025', 'default', '2025-12-31', '31-Dec-2025', '00:00', NULL, 'ONIH / JAJA / US', 'NON_KAS', 'Angsuran pinjaman 2025 (ringkasan tahunan, ikut Neraca RAT)', 'default-SYNC027-PIN-L225-2025', 'Masuk', 1300000, 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default-SYNC027-PIN-L225-ARREARS-2026', 'default', '2026-01-01', '01-Jan-2026', '00:00', NULL, 'ONIH / JAJA / US', 'NON_KAS', 'Tunggakan awal 2026', 'default-SYNC027-PIN-L225-ARREARS-2026', 'Masuk', 700000, 'Data script 027_saldo_udin_onih_ikut_rat'))
WHERE EXISTS (SELECT 1 FROM sync027_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id) SELECT * FROM (VALUES
  ('default', 'default-SYNC027-PIN-L202-2025', 'LOAN_PRINCIPAL', 'Pokok pinjaman', 450000, 'default-L202', NULL),
  ('default', 'default-SYNC027-PIN-L202-ARREARS-2026', 'LOAN_OPENING_ARREARS_PRINCIPAL', 'Tunggakan pokok awal 2026', 1850000, 'default-L202', NULL),
  ('default', 'default-SYNC027-PIN-L225-2025', 'LOAN_PRINCIPAL', 'Pokok pinjaman', 1300000, 'default-L225', NULL),
  ('default', 'default-SYNC027-PIN-L225-ARREARS-2026', 'LOAN_OPENING_ARREARS_PRINCIPAL', 'Tunggakan pokok awal 2026', 700000, 'default-L225', NULL))
WHERE EXISTS (SELECT 1 FROM sync027_ok);
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) SELECT * FROM (VALUES
  ('default', 'TRANSACTION', 'default-SYNC027-PIN-L202-2025', 'CREATED', json_object('reason', 'Sheet PINJAMAN SEMENTARA baris 24: saldo Des 2025 1.850.000'), 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default', 'TRANSACTION', 'default-SYNC027-PIN-L202-ARREARS-2026', 'CREATED', json_object('reason', 'Tunggakan pokok s/d Des 2025 1.850.000'), 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default', 'TRANSACTION', 'default-SYNC027-PIN-L225-2025', 'CREATED', json_object('reason', 'Sheet PINJAMAN SEMENTARA baris 47: saldo Des 2025 700.000'), 'Data script 027_saldo_udin_onih_ikut_rat'),
  ('default', 'TRANSACTION', 'default-SYNC027-PIN-L225-ARREARS-2026', 'CREATED', json_object('reason', 'Tunggakan pokok s/d Des 2025 700.000'), 'Data script 027_saldo_udin_onih_ikut_rat'))
WHERE EXISTS (SELECT 1 FROM sync027_ok);
DROP TABLE sync027_rev;
DROP TABLE sync027_ok;
