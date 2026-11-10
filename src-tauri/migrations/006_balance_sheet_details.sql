-- Neraca: rincian of a typed line (Piutang Lainnya: the Bon Sementara per
-- person with the date borrowed), per book year. Saving a rincian also sets
-- the line in balance_sheet_items to its total.
CREATE TABLE balance_sheet_details (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  year INTEGER NOT NULL,
  item_key TEXT NOT NULL,
  position INTEGER NOT NULL,
  name TEXT NOT NULL,
  -- Negative for a deduction, e.g. "Bayar dari SHU".
  amount INTEGER NOT NULL,
  loan_date TEXT,
  note TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX balance_sheet_details_line
  ON balance_sheet_details (company_id, year, item_key, position);
