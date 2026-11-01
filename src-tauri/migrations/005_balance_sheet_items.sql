-- Neraca: the lines not derived from the ledger (Bank adjustments, Pinjaman
-- Lainnya, Hutang ke PPBRI, cadangan and dana from NERACA PENYESUAIAN, Aktiva
-- Tetap, Laba ditahan), typed once per book year.
CREATE TABLE balance_sheet_items (
  company_id TEXT NOT NULL REFERENCES companies(id),
  year INTEGER NOT NULL,
  item_key TEXT NOT NULL,
  amount INTEGER NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (company_id, year, item_key)
);
