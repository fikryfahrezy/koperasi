-- Piutang Lainnya: every bon (Bon Sementara) keeps a stable key, so a payment
-- can say which bon it pays. A Bayar Kas Bon of Buku Kas links to its bon in
-- cash_bond_payments. A Pengurang row of the list (e.g. "Bayar dari SHU")
-- pays no bon in particular; it only lowers its person's total. A bon is
-- Lunas once its payments add up to its amount. See notes/rules-and-formulas.md (Piutang Lainnya).

-- The bon's own key; set on the bon rows (amount > 0) of Piutang Lainnya.
ALTER TABLE balance_sheet_details ADD COLUMN bon_key TEXT;

UPDATE balance_sheet_details
SET bon_key = 'BON-' || id
WHERE item_key = 'OTHER_LOANS' AND amount > 0;

-- How much of a Bayar Kas Bon of Buku Kas pays each bon: one payment may pay
-- several bons, all or part of each. The rows stay when the payment is
-- deleted (Hapus); only payments still in effect count.
CREATE TABLE cash_bond_payments (
  company_id TEXT NOT NULL REFERENCES companies(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  bon_key TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (company_id, transaction_id, bon_key)
);
CREATE INDEX cash_bond_payments_bon ON cash_bond_payments (company_id, bon_key);
