-- Memperbaiki Tunggakan s/d Desember 2025 pinjaman L225 dan pinjaman L202
-- yang salah karena data script 027. Diputuskan 14-11-2026.
-- Untuk pinjaman yang punya data 2025, aplikasi sudah menghitung tunggakan akhir 2025 dari
-- jadwal 2025, sehingga komponen LOAN_OPENING_ARREARS/PREPAID_* awal 2026 adalah KOREKSI
-- (tunggakan sheet − hasil hitungan), bukan angka tunggakannya. 027 mengisinya sebagai angka
-- absolut:
-- * L225: hitungan 2025 = pokok 700.000 (2.000.000 − angsuran 1.300.000) + bunga 600.000
--   (Jan–Mar 2025, jatuh tempo 08-03-2025; ringkasan 2025 tanpa bunga). Ditambah 700.000 dari
--   027 → tampil 1.400.000 / 600.000. Target sheet PINJAMAN SEMENTARA baris 155 (O–Q):
--   700.000 / 0. Koreksi: balik tunggakan pokok 027, catat lebih bayar bunga 600.000.
-- * L202: jatuh tempo 26-05-2024, jadi 2025 tanpa angsuran baru; angsuran pokok 2025 450.000
--   membuat hitungan −450.000. Ditambah 1.850.000 dari 027 → tampil 1.400.000. Target
--   1.850.000: tambah tunggakan pokok 450.000.
-- Saldo pinjaman tidak berubah. Semua NON_KAS, Kas dan Neraca tidak berubah.
-- Revert: balik transaksi default-SYNC032-* (dan balik pembalikan 027) dengan script baru.
CREATE TEMP TABLE sync032_ok AS SELECT 1 AS ok WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE id LIKE 'default-SYNC032-%')
  AND (SELECT COUNT(*) FROM effective_transactions WHERE company_id = 'default' AND id IN ('default-SYNC027-PIN-L225-ARREARS-2026', 'default-SYNC027-PIN-L202-ARREARS-2026')) = 2;
-- 1. Balik tunggakan pokok awal 2026 L225 dari 027.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, reversed_transaction_id, actor)
SELECT 'default-SYNC032-REV-' || substr(t.id, 9), t.company_id, t.business_date, t.display_date, t.display_time, t.member_id, t.member_name, t.channel, 'Koreksi tunggakan awal · ' || t.description, t.reference || '/REV/032', CASE t.direction WHEN 'Masuk' THEN 'Keluar' ELSE 'Masuk' END, t.amount, t.id, 'Data script 032_tunggakan_awal_l202_l225'
FROM transactions t WHERE t.company_id = 'default' AND t.id = 'default-SYNC027-PIN-L225-ARREARS-2026' AND EXISTS (SELECT 1 FROM sync032_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT c.company_id, 'default-SYNC032-REV-' || substr(c.transaction_id, 9), 'REVERSAL', 'Koreksi tunggakan awal · ' || c.label, c.amount, c.loan_id, c.savings_account_id
FROM transaction_components c WHERE c.transaction_id = 'default-SYNC027-PIN-L225-ARREARS-2026' AND EXISTS (SELECT 1 FROM sync032_ok) ORDER BY c.id;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-SYNC027-PIN-L225-ARREARS-2026', 'REVERSED', json_object('reversalId', 'default-SYNC032-REV-SYNC027-PIN-L225-ARREARS-2026', 'reason', 'Tunggakan pokok 700.000 sudah terhitung dari jadwal 2025 (terhitung dua kali)'), 'Data script 032_tunggakan_awal_l202_l225'
WHERE EXISTS (SELECT 1 FROM sync032_ok);
-- 2. Catat koreksi.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, actor) SELECT * FROM (VALUES
  ('default-SYNC032-PIN-L225-ARREARS-2026', 'default', '2026-01-01', '01-Jan-2026', '00:00', NULL, (SELECT member_name FROM loans WHERE company_id = 'default' AND id = 'default-L225'), 'NON_KAS', 'Koreksi tunggakan awal 2026', 'default-SYNC032-PIN-L225-ARREARS-2026', 'Masuk', 600000, 'Data script 032_tunggakan_awal_l202_l225'),
  ('default-SYNC032-PIN-L202-ARREARS-2026', 'default', '2026-01-01', '01-Jan-2026', '00:00', NULL, (SELECT member_name FROM loans WHERE company_id = 'default' AND id = 'default-L202'), 'NON_KAS', 'Koreksi tunggakan awal 2026', 'default-SYNC032-PIN-L202-ARREARS-2026', 'Masuk', 450000, 'Data script 032_tunggakan_awal_l202_l225'))
WHERE EXISTS (SELECT 1 FROM sync032_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id) SELECT * FROM (VALUES
  ('default', 'default-SYNC032-PIN-L225-ARREARS-2026', 'LOAN_OPENING_PREPAID_INTEREST', 'Lebih bayar bunga awal 2026', 600000, 'default-L225', NULL),
  ('default', 'default-SYNC032-PIN-L202-ARREARS-2026', 'LOAN_OPENING_ARREARS_PRINCIPAL', 'Tunggakan pokok awal 2026', 450000, 'default-L202', NULL))
WHERE EXISTS (SELECT 1 FROM sync032_ok);
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor) SELECT * FROM (VALUES
  ('default', 'TRANSACTION', 'default-SYNC032-PIN-L225-ARREARS-2026', 'CREATED', json_object('reason', 'Sheet PINJAMAN SEMENTARA baris 155: tunggakan s/d Des 2025 700.000 / 0; bunga Jan–Mar 2025 600.000 dinolkan'), 'Data script 032_tunggakan_awal_l202_l225'),
  ('default', 'TRANSACTION', 'default-SYNC032-PIN-L202-ARREARS-2026', 'CREATED', json_object('reason', 'Tunggakan pokok s/d Des 2025 1.850.000; angsuran 2025 450.000 membuat hitungan −450.000'), 'Data script 032_tunggakan_awal_l202_l225'))
WHERE EXISTS (SELECT 1 FROM sync032_ok);
DROP TABLE sync032_ok;
