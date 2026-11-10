-- DJUANTARA NAZAR / NINING ROHANI, pinjaman 20 jt (realisasi 14-09-2026, LOAN-1792499786484-1):
-- dilunasi 19-10-2026 sebesar 19.333.000 saat suplesi ke pinjaman 22 jt (default-SYNC016-L105).
-- Sheet PINJAMAN BULANAN 2026 (10-11-2026) baris 93: saldo Oktober 0. Kas sudah tercatat di
-- Buku Kas (default-SYNC016-KAS-717 "Pelunasan Pinjaman Lama" 19.333.000), jadi di sini NON_KAS
-- dan saldo Kas tidak berubah. Hanya jalan bila sisa pokok pinjaman itu masih tepat 19.333.000.
CREATE TEMP TABLE sync025_ok AS SELECT 1 AS ok
WHERE NOT EXISTS (SELECT 1 FROM transactions WHERE id LIKE 'default-SYNC025-%')
  AND EXISTS (SELECT 1 FROM loans WHERE id = 'LOAN-1792499786484-1' AND company_id = 'default' AND plafond = 20000000)
  AND (SELECT COALESCE(SUM(c.amount), 0) FROM transaction_components c JOIN effective_transactions t ON t.id = c.transaction_id
       WHERE c.loan_id = 'LOAN-1792499786484-1' AND c.component_type = 'LOAN_PRINCIPAL') = 667000;
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, actor)
SELECT 'default-SYNC025-PIN-1792499786484-1-2026-10', 'default', '2026-10-19', '19-Oct-2026', '00:00', l.member_id, l.member_name, 'NON_KAS',
       'Pelunasan pinjaman lama (suplesi 22 jt)', 'default-SYNC025-PIN-1792499786484-1-2026-10', 'Masuk', 19333000, 'Data script 025_lunas_djuantara_20jt_okt_2026'
FROM loans l WHERE l.id = 'LOAN-1792499786484-1' AND EXISTS (SELECT 1 FROM sync025_ok);
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT 'default', 'default-SYNC025-PIN-1792499786484-1-2026-10', 'LOAN_PRINCIPAL', 'Pokok pinjaman', 19333000, 'LOAN-1792499786484-1', NULL
WHERE EXISTS (SELECT 1 FROM sync025_ok);
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-SYNC025-PIN-1792499786484-1-2026-10', 'CREATED',
       json_object('reason', 'Lunas 19-10-2026; sheet PINJAMAN BULANAN baris 93; Buku Kas default-SYNC016-KAS-717'), 'Data script 025_lunas_djuantara_20jt_okt_2026'
WHERE EXISTS (SELECT 1 FROM sync025_ok);
DROP TABLE sync025_ok;
