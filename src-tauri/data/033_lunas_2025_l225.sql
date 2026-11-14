-- pinjaman L225 sudah lunas di 2025 (konfirmasi bendahara 14-11-2026):
-- Saldo Posisi Desember 2025 = 0, bukan 700.000. Sesuai sheet DATA PINJAMAN TAHUN 2025
-- (PINJAMAN baris 259: setoran 22-01-2025 dan 02-03-2025, sisa hutang Maret 2025 = 0) dan
-- daftar pinjaman RAT posisi Desember 2025 (baris 141 = 0). Neraca RAT 31-12-2025 (Piutang Non
-- Anggota 221.390.000) masih memasukkan 700.000 ini.
-- Membalik angsuran pokok 2025 1.300.000 dari 027 dan mencatat 2.000.000 (seperti 019).
-- Tunggakan s/d Des 2025 tetap 0 / 0: pokok 0 karena saldo 0, bunga Jan–Mar 2025 600.000 sudah
-- dinolkan oleh koreksi 032 (Lebih bayar bunga awal 2026 600.000).
-- Semua NON_KAS, Kas tidak berubah. Jalan setelah 032.
CREATE TEMP TABLE sync033_ok AS SELECT 1 AS ok WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE id LIKE 'default-SYNC033-%')
  AND (SELECT COUNT(*) FROM effective_transactions WHERE company_id = 'default' AND id IN ('default-SYNC027-PIN-L225-2025', 'default-SYNC032-PIN-L225-ARREARS-2026')) = 2
  AND NOT EXISTS (SELECT 1 FROM effective_transactions WHERE company_id = 'default' AND id = 'default-SYNC027-PIN-L225-ARREARS-2026');
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, reversed_transaction_id, actor)
SELECT 'default-SYNC033-REV-' || substr(t.id, 9), t.company_id, t.business_date, t.display_date, t.display_time, t.member_id, t.member_name, t.channel, 'Lunas 2025 · ' || t.description, t.reference || '/REV/033', CASE t.direction WHEN 'Masuk' THEN 'Keluar' ELSE 'Masuk' END, t.amount, t.id, 'Data script 033_lunas_2025_l225'
FROM transactions t WHERE t.company_id = 'default' AND t.id = 'default-SYNC027-PIN-L225-2025' AND EXISTS (SELECT 1 FROM sync033_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT c.company_id, 'default-SYNC033-REV-' || substr(c.transaction_id, 9), 'REVERSAL', 'Lunas 2025 · ' || c.label, c.amount, c.loan_id, c.savings_account_id
FROM transaction_components c WHERE c.transaction_id = 'default-SYNC027-PIN-L225-2025' AND EXISTS (SELECT 1 FROM sync033_ok) ORDER BY c.id;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-SYNC027-PIN-L225-2025', 'REVERSED', json_object('reversalId', 'default-SYNC033-REV-SYNC027-PIN-L225-2025', 'reason', 'Bendahara: L225 lunas di 2025, saldo Des 2025 = 0'), 'Data script 033_lunas_2025_l225'
WHERE EXISTS (SELECT 1 FROM sync033_ok);
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, actor)
SELECT 'default-SYNC033-PIN-L225-2025', 'default', '2025-12-31', '31-Dec-2025', '00:00', NULL, (SELECT member_name FROM loans WHERE company_id = 'default' AND id = 'default-L225'), 'NON_KAS', 'Angsuran pinjaman 2025 (ringkasan tahunan, lunas 2025)', 'default-SYNC033-PIN-L225-2025', 'Masuk', 2000000, 'Data script 033_lunas_2025_l225'
WHERE EXISTS (SELECT 1 FROM sync033_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT 'default', 'default-SYNC033-PIN-L225-2025', 'LOAN_PRINCIPAL', 'Pokok pinjaman', 2000000, 'default-L225', NULL
WHERE EXISTS (SELECT 1 FROM sync033_ok);
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-SYNC033-PIN-L225-2025', 'CREATED', json_object('reason', 'Lunas 2025: angsuran pokok 2025 2.000.000, saldo Des 2025 = 0'), 'Data script 033_lunas_2025_l225'
WHERE EXISTS (SELECT 1 FROM sync033_ok);
DROP TABLE sync033_ok;
