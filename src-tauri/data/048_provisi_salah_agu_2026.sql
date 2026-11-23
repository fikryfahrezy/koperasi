-- Provisi 23.410.000 pada mutasi Agustus 2026 pinjaman L042 adalah salah input di sheet DATA
-- PINJAMAN TAHUN 2026 (PINJAMAN BULANAN, kolom Provisi blok Agustus); dikonfirmasi bendahara
-- 23-11-2026. Angkanya sama dengan pelunasan September (pokok 22.910.000 + bunga 500.000),
-- bukan provisi.
-- Transaksi tidak bisa diubah, jadi mutasi Agustus itu dibalik (seperti Hapus) lalu dicatat
-- ulang tanpa provisi: pokok 695.000 + bunga 500.000 = 1.195.000 (sama dengan kolom Jumlah
-- sheet), di grup impor yang sama. Saldo pinjaman tidak berubah; Pendapatan Provisi Agustus
-- turun 23.410.000.
-- Hanya jalan kalau mutasinya masih berlaku dengan isi persis seperti di atas. Database lain
-- (instalasi baru, e2e) tidak punya baris ini, jadi tidak ada yang diubah.

CREATE TEMP TABLE fix048_ok AS
SELECT 1 AS ok
FROM effective_transactions t
WHERE t.company_id = 'default'
  AND t.id = 'default-MIG-PIN-L042-2026-08'
  AND t.business_date = '2026-08-27'
  AND t.amount = 24605000
  AND (SELECT COUNT(*) FROM transaction_components c
       WHERE c.company_id = t.company_id AND c.transaction_id = t.id) = 3
  AND EXISTS (SELECT 1 FROM transaction_components c
              WHERE c.company_id = t.company_id AND c.transaction_id = t.id
                AND c.component_type = 'LOAN_PROVISION' AND c.amount = 23410000
                AND c.loan_id = 'default-L042')
  AND NOT EXISTS (SELECT 1 FROM transactions WHERE company_id = 'default' AND id LIKE 'default-FIX048-%');

-- 1. Balik mutasi Agustus.
INSERT INTO transactions (id, company_id, business_date, member_id, member_name, channel, description, direction, amount, reversed_transaction_id)
SELECT 'default-FIX048-REV-L042-2026-08', company_id, business_date, member_id, member_name, channel,
  'Hapus · ' || description, CASE direction WHEN 'Masuk' THEN 'Keluar' ELSE 'Masuk' END, amount, id
FROM transactions
WHERE company_id = 'default' AND id = 'default-MIG-PIN-L042-2026-08'
  AND EXISTS (SELECT 1 FROM fix048_ok);

INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT company_id, 'default-FIX048-REV-L042-2026-08', 'REVERSAL', 'Hapus · ' || label, amount, loan_id, savings_account_id
FROM transaction_components
WHERE company_id = 'default' AND transaction_id = 'default-MIG-PIN-L042-2026-08'
  AND EXISTS (SELECT 1 FROM fix048_ok)
ORDER BY id;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-MIG-PIN-L042-2026-08', 'REVERSED',
  json_object('reversalId', 'default-FIX048-REV-L042-2026-08',
    'reason', 'Provisi 23.410.000 salah input di sheet; dicatat ulang tanpa provisi'),
  'Data script 048_provisi_salah_agu_2026'
WHERE EXISTS (SELECT 1 FROM fix048_ok);

-- 2. Catat ulang tanpa provisi, di grup impor yang sama.
INSERT INTO transactions (id, company_id, business_date, member_id, member_name, channel, description, direction, amount)
SELECT 'default-FIX048-PIN-L042-2026-08', company_id, business_date, member_id, member_name, channel,
  description, direction, 1195000
FROM transactions
WHERE company_id = 'default' AND id = 'default-MIG-PIN-L042-2026-08'
  AND EXISTS (SELECT 1 FROM fix048_ok);

INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT company_id, 'default-FIX048-PIN-L042-2026-08', component_type, label, amount, loan_id, savings_account_id
FROM transaction_components
WHERE company_id = 'default' AND transaction_id = 'default-MIG-PIN-L042-2026-08'
  AND component_type IN ('LOAN_PRINCIPAL', 'LOAN_INTEREST')
  AND EXISTS (SELECT 1 FROM fix048_ok)
ORDER BY id;

INSERT INTO transaction_groups (company_id, group_id, member_id, loan_id, transaction_id)
SELECT company_id, group_id, member_id, loan_id, 'default-FIX048-PIN-L042-2026-08'
FROM transaction_groups
WHERE company_id = 'default' AND transaction_id = 'default-MIG-PIN-L042-2026-08'
  AND EXISTS (SELECT 1 FROM fix048_ok);

INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', 'default-FIX048-PIN-L042-2026-08', 'CREATED',
  json_object('reason', 'Mutasi Agustus 2026 pinjaman L042 tanpa provisi salah input',
    'replaces', 'default-MIG-PIN-L042-2026-08'),
  'Data script 048_provisi_salah_agu_2026'
WHERE EXISTS (SELECT 1 FROM fix048_ok);

DROP TABLE fix048_ok;
