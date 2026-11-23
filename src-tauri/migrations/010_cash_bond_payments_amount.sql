-- cash_bond_payments with the amount paid of each bon, so one Bayar Kas Bon can
-- pay several bons. A database that ran an early 009 (before it was released)
-- has the table without `amount`: there each link is the whole payment, the
-- only kind it could hold. Rebuilt the same way on every database.

CREATE TABLE cash_bond_payments_new (
  company_id TEXT NOT NULL REFERENCES companies(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  bon_key TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (company_id, transaction_id, bon_key)
);

INSERT INTO cash_bond_payments_new (company_id, transaction_id, bon_key, amount, created_at)
SELECT p.company_id, p.transaction_id, p.bon_key, t.amount, p.created_at
FROM cash_bond_payments p
JOIN transactions t ON t.company_id = p.company_id AND t.id = p.transaction_id;

DROP TABLE cash_bond_payments;
ALTER TABLE cash_bond_payments_new RENAME TO cash_bond_payments;
CREATE INDEX cash_bond_payments_bon ON cash_bond_payments (company_id, bon_key);
