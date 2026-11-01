-- Bank of the Neraca at 31 Desember 2025 (Neraca RAT, kolom TAHUN 2025; also
-- the opening Kas Buku Tabungan of 2026). Bank is now typed per book year.
INSERT OR IGNORE INTO balance_sheet_items (company_id, year, item_key, amount)
SELECT id, 2025, 'BANK_BALANCE', 11698551 FROM companies WHERE id = 'default';
