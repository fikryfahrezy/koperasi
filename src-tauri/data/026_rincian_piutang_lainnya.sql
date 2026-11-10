-- Rincian Piutang Lainnya (Bon Sementara) tahun buku 2025 dan 2026, dari Catatan
-- Neraca RAT 31 Desember 2025 (CONTOH LAPORAN KEUANGAN RAT, sheet Neraca, "Bukti
-- Tanggal Pinjam"). Jumlahnya 28.109.000, sama dengan Piutang Lainnya yang sudah
-- diketik. Tahun yang sudah punya rincian, atau Piutang Lainnya-nya sudah diubah,
-- tidak disentuh.
CREATE TEMP TABLE seed026 (position INTEGER, name TEXT, amount INTEGER, loan_date TEXT, note TEXT);
INSERT INTO seed026 (position, name, amount, loan_date, note) VALUES
  (0, 'Dadang Sunaryo', 2000000, '2023-06-26', ''),
  (1, 'Pujiono', 700000, '2024-01-02', ''),
  (2, 'H Masady', 1800000, '2024-03-13', ''),
  (3, 'Ujang Sukma', 200000, '2025-01-13', ''),
  (4, 'Ujang Sukma', 1000000, '2023-07-06', ''),
  (5, 'Ujang Sukma', 300000, '2024-02-13', ''),
  (6, 'Ujang Sukma', 1000000, '2023-07-14', ''),
  (7, 'Ujang Sukma', 500000, '2024-07-23', ''),
  (8, 'Ujang Sukma', 500000, '2024-06-08', ''),
  (9, 'Ujang Sukma', 200000, '2025-02-11', ''),
  (10, 'Ujang Sukma', -1391000, NULL, 'Bayar dari SHU 2023/2024'),
  (11, 'H Salmon', 1000000, '2024-07-02', ''),
  (12, 'H Salmon', 2500000, '2024-11-13', ''),
  (13, 'H Salmon', 500000, '2025-02-02', ''),
  (14, 'H Salmon', 8400000, '2023-11-20', ''),
  (15, 'H Salmon', 1500000, '2024-02-02', 'Hj Erna / H Salmon'),
  (16, 'H Salmon', 1000000, '2024-11-05', 'Hj Erna / H Salmon'),
  (17, 'H Salmon', 2000000, '2024-07-01', 'Iwan Setiawan / H Salmon'),
  (18, 'H Salmon', 3900000, '2024-01-30', 'Ade Dadi / H Salmon'),
  (19, 'Adang Rahmat', 500000, '2023-01-04', '');
INSERT INTO balance_sheet_details (company_id, year, item_key, position, name, amount, loan_date, note)
SELECT c.id, y.year, 'OTHER_LOANS', s.position, s.name, s.amount, s.loan_date, s.note
FROM companies c, (SELECT 2025 AS year UNION ALL SELECT 2026) y, seed026 s
WHERE c.id = 'default'
  AND NOT EXISTS (SELECT 1 FROM balance_sheet_details d WHERE d.company_id = c.id AND d.year = y.year AND d.item_key = 'OTHER_LOANS')
  AND COALESCE((SELECT amount FROM balance_sheet_items i WHERE i.company_id = c.id AND i.year = y.year AND i.item_key = 'OTHER_LOANS'), 28109000) = 28109000
ORDER BY y.year, s.position;
INSERT OR IGNORE INTO balance_sheet_items (company_id, year, item_key, amount)
SELECT DISTINCT company_id, year, 'OTHER_LOANS', 28109000 FROM balance_sheet_details WHERE company_id = 'default' AND item_key = 'OTHER_LOANS' AND year IN (2025, 2026);
DROP TABLE seed026;
