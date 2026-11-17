-- Removes schema pieces nothing reads any more.

-- app_meta only held seed_version for the workbook import, which was removed.
DROP TABLE IF EXISTS app_meta;

-- Covered by the index behind members' UNIQUE(company_id, id).
DROP INDEX IF EXISTS idx_members_company;

-- transaction_groups.label was never filled in; the app always shows
-- "<member or borrower> · <month>" built from the group.
ALTER TABLE transaction_groups DROP COLUMN label;

-- updated_at of the Neraca tables only kept the time of the last save, and the
-- rincian are re-inserted on every save. Each save is now an audit event
-- (BALANCE_SHEET) with its time and the old and new values.
ALTER TABLE balance_sheet_items DROP COLUMN updated_at;
ALTER TABLE balance_sheet_details DROP COLUMN updated_at;

-- transactions rebuilt without four columns nothing needs:
--   reference     the forms never let anyone type one, so it was always the id
--                 or built from it; reversed_transaction_id links a Hapus.
--   display_date  business_date written for display; the app formats it.
--   display_time  the clock time when saved; created_at holds the same moment.
--   actor         always "Aplikasi lokal".
-- reference is in a UNIQUE constraint, so SQLite cannot drop it in place.
-- Views reading the table are dropped first and recreated below; the
-- migration runner turns foreign keys off and checks them before commit.

DROP VIEW IF EXISTS loan_balances;
DROP VIEW IF EXISTS savings_balances;
DROP VIEW IF EXISTS effective_components;
DROP VIEW IF EXISTS effective_transactions;

CREATE TABLE transactions_new (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  business_date TEXT NOT NULL,
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  channel TEXT NOT NULL,
  description TEXT NOT NULL,
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  reversed_transaction_id TEXT REFERENCES transactions(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- rowid is copied too: Buku Kas orders a day's rows by it.
INSERT INTO transactions_new (
  rowid, id, company_id, business_date, member_id, member_name, channel,
  description, direction, amount, reversed_transaction_id, created_at
)
SELECT
  rowid, id, company_id, business_date, member_id, member_name, channel,
  description, direction, amount, reversed_transaction_id, created_at
FROM transactions;

DROP TABLE transactions;
ALTER TABLE transactions_new RENAME TO transactions;

CREATE INDEX idx_transactions_company_date ON transactions(company_id, business_date);
CREATE INDEX idx_transactions_reversed ON transactions(company_id, reversed_transaction_id);
CREATE INDEX idx_transactions_member_id ON transactions(member_id);

CREATE TRIGGER transactions_append_only_update
BEFORE UPDATE ON transactions
BEGIN
  SELECT RAISE(ABORT, 'Transaksi tidak dapat diubah; buat reversal.');
END;

CREATE TRIGGER transactions_append_only_delete
BEFORE DELETE ON transactions
BEGIN
  SELECT RAISE(ABORT, 'Transaksi tidak dapat dihapus; buat reversal.');
END;

CREATE VIEW effective_transactions AS
SELECT t.*
FROM transactions t
WHERE t.reversed_transaction_id IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM transactions r
    WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id
  );

CREATE VIEW effective_components AS
SELECT c.*, t.business_date, t.channel
FROM transaction_components c
JOIN effective_transactions t
  ON t.company_id = c.company_id AND t.id = c.transaction_id;

CREATE VIEW savings_balances AS
SELECT
  sa.company_id,
  sa.id AS savings_account_id,
  sa.member_id,
  sa.account_type,
  COALESCE(SUM(CASE c.component_type
    WHEN 'SAVINGS_WITHDRAWAL' THEN -c.amount
    ELSE c.amount
  END), 0) AS balance
FROM savings_accounts sa
LEFT JOIN effective_components c
  ON c.company_id = sa.company_id AND c.savings_account_id = sa.id
GROUP BY sa.company_id, sa.id;

-- loan_balances without the unused movements count.
CREATE VIEW loan_balances AS
SELECT
  l.company_id,
  l.id AS loan_id,
  COALESCE(SUM(CASE
    WHEN c.component_type IN ('LOAN_OPENING', 'LOAN_DISBURSEMENT') THEN c.amount
    WHEN c.component_type = 'LOAN_PRINCIPAL' THEN -c.amount
    ELSE 0
  END), 0) AS balance
FROM loans l
LEFT JOIN effective_components c
  ON c.company_id = l.company_id AND c.loan_id = l.id
GROUP BY l.company_id, l.id;
