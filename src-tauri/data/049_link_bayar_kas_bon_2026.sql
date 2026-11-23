-- Tiga Bayar Kas Bon Buku Kas 2026 (29-03, 01-04, 15-04) dihubungkan ke bonnya di rincian
-- Piutang Lainnya 2026, supaya status Lunas bon itu terlihat. Jumlah dan uraian tiap pembayaran
-- persis sama dengan satu bon di daftar (dicek 23-11-2026); bon dicari lewat jumlah dan tanggal
-- bonnya. Piutang Lainnya tidak berubah (tetap 23.009.000): hanya ditandai bon mana yang dibayar.
-- Hanya jalan untuk pembayaran yang masih berlaku dengan jumlah itu, yang belum terhubung, dan
-- kalau bonnya ada tepat satu. Database lain tidak punya baris ini, jadi tidak ada yang diubah.

INSERT INTO cash_bond_payments (company_id, transaction_id, bon_key, amount)
SELECT t.company_id, t.id, d.bon_key, t.amount
FROM (
  SELECT 'default-MIG-KAS-234' AS id, 3900000 AS amount, '2024-01-30' AS loan_date
  UNION ALL SELECT 'default-MIG-KAS-245', 500000, '2023-01-04'
  UNION ALL SELECT 'default-MIG-KAS-296', 700000, '2024-01-02'
) link
JOIN effective_transactions t
  ON t.company_id = 'default' AND t.id = link.id AND t.amount = link.amount
JOIN balance_sheet_details d
  ON d.company_id = t.company_id AND d.year = 2026 AND d.item_key = 'OTHER_LOANS'
 AND d.amount = link.amount AND d.loan_date = link.loan_date AND d.bon_key IS NOT NULL
WHERE EXISTS (
    SELECT 1 FROM transaction_components c
    WHERE c.company_id = t.company_id AND c.transaction_id = t.id
      AND c.component_type = 'BAYAR_KAS_BON' AND c.amount = link.amount
  )
  AND (
    SELECT COUNT(*) FROM balance_sheet_details other
    WHERE other.company_id = t.company_id AND other.year = 2026 AND other.item_key = 'OTHER_LOANS'
      AND other.amount = link.amount AND other.loan_date = link.loan_date
  ) = 1
  AND NOT EXISTS (
    SELECT 1 FROM cash_bond_payments p
    WHERE p.company_id = t.company_id AND p.transaction_id = t.id
  );
