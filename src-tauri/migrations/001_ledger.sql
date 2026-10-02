-- Cooperative ledger schema.
--
-- Principle: monetary records are never updated in place. Each money movement is recorded
-- as a new transaction with its components; cash, savings, and
-- loan balances are calculated from those transactions. Corrections use
-- reversal transactions rather than editing or deleting rows.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS companies (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO companies (id, name) VALUES
  ('default', 'Koperasi Bina Sejahtera'),
  ('testing', 'Perusahaan Pengujian');

CREATE TABLE IF NOT EXISTS app_meta (
  company_id TEXT NOT NULL REFERENCES companies(id),
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (company_id, key)
);

CREATE TABLE IF NOT EXISTS members (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  name TEXT NOT NULL,
  joined_at TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(company_id, id)
);

-- Savings accounts store only identity information; balances are in the savings_balances view.
CREATE TABLE IF NOT EXISTS savings_accounts (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  member_id TEXT NOT NULL REFERENCES members(id),
  account_type TEXT NOT NULL,
  status TEXT NOT NULL,
  UNIQUE(company_id, member_id, account_type)
);

-- Loan terms; principal balances are in the loan_balances view.
CREATE TABLE IF NOT EXISTS loans (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  plafond INTEGER NOT NULL DEFAULT 0 CHECK (plafond >= 0),
  rate_annual REAL NOT NULL,
  tenor INTEGER NOT NULL CHECK (tenor > 0),
  interest_type TEXT NOT NULL,
  guarantee TEXT NOT NULL DEFAULT '',
  realization_date TEXT NOT NULL,
  due_date TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- channel:
--   KAS      recorded in the daily cash ledger and changes cash balances
--   NON_KAS  excluded from the cash ledger; only changes savings/loans
CREATE TABLE IF NOT EXISTS transactions (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  business_date TEXT NOT NULL,
  display_date TEXT NOT NULL,
  display_time TEXT NOT NULL,
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  transaction_type TEXT NOT NULL,
  channel TEXT NOT NULL,
  description TEXT NOT NULL,
  reference TEXT NOT NULL,
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  reversed_transaction_id TEXT REFERENCES transactions(id),
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(company_id, reference)
);

CREATE TABLE IF NOT EXISTS transaction_components (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  component_type TEXT NOT NULL,
  label TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  loan_id TEXT REFERENCES loans(id),
  savings_account_id TEXT REFERENCES savings_accounts(id)
);

CREATE TRIGGER IF NOT EXISTS transactions_append_only_update
BEFORE UPDATE ON transactions
BEGIN
  SELECT RAISE(ABORT, 'Transaksi tidak dapat diubah; buat reversal.');
END;

CREATE TRIGGER IF NOT EXISTS transactions_append_only_delete
BEFORE DELETE ON transactions
BEGIN
  SELECT RAISE(ABORT, 'Transaksi tidak dapat dihapus; buat reversal.');
END;

CREATE TRIGGER IF NOT EXISTS components_append_only_update
BEFORE UPDATE ON transaction_components
BEGIN
  SELECT RAISE(ABORT, 'Komponen transaksi tidak dapat diubah.');
END;

CREATE TRIGGER IF NOT EXISTS components_append_only_delete
BEFORE DELETE ON transaction_components
BEGIN
  SELECT RAISE(ABORT, 'Komponen transaksi tidak dapat dihapus.');
END;

-- Effective transactions: neither reversals nor already reversed.
CREATE VIEW IF NOT EXISTS effective_transactions AS
SELECT t.*
FROM transactions t
WHERE t.transaction_type != 'REVERSAL'
  AND NOT EXISTS (
    SELECT 1 FROM transactions r
    WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id
  );

CREATE VIEW IF NOT EXISTS effective_components AS
SELECT c.*, t.business_date, t.channel
FROM transaction_components c
JOIN effective_transactions t
  ON t.company_id = c.company_id AND t.id = c.transaction_id;

CREATE VIEW IF NOT EXISTS savings_balances AS
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

CREATE VIEW IF NOT EXISTS loan_balances AS
SELECT
  l.company_id,
  l.id AS loan_id,
  COALESCE(SUM(CASE
    WHEN c.component_type IN ('LOAN_OPENING', 'LOAN_DISBURSEMENT') THEN c.amount
    WHEN c.component_type = 'LOAN_PRINCIPAL' THEN -c.amount
    ELSE 0
  END), 0) AS balance,
  COUNT(c.id) AS movements
FROM loans l
LEFT JOIN effective_components c
  ON c.company_id = l.company_id AND c.loan_id = l.id
GROUP BY l.company_id, l.id;

CREATE TABLE IF NOT EXISTS audit_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS periods (
  company_id TEXT NOT NULL REFERENCES companies(id),
  period TEXT NOT NULL,
  status TEXT NOT NULL,
  locked_by TEXT,
  locked_at TEXT,
  PRIMARY KEY(company_id, period)
);

CREATE TABLE IF NOT EXISTS parameters (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  parameter_key TEXT NOT NULL,
  value REAL NOT NULL,
  effective_date TEXT NOT NULL,
  created_by TEXT NOT NULL,
  UNIQUE(company_id, parameter_key, effective_date)
);

INSERT OR IGNORE INTO parameters (company_id, parameter_key, value, effective_date, created_by)
SELECT c.id, p.parameter_key, p.value, '2026-01-01', 'Skema awal'
FROM companies c
CROSS JOIN (
  SELECT 'SAVINGS_PRINCIPAL' AS parameter_key, 50000.0 AS value
  UNION ALL SELECT 'SAVINGS_MONTHLY', 50000.0
  UNION ALL SELECT 'LOAN_PROVISION_RATE', 0.01
  UNION ALL SELECT 'LOAN_ANNUAL_RATE', 0.24
) p;

CREATE INDEX IF NOT EXISTS idx_members_company ON members(company_id);
CREATE INDEX IF NOT EXISTS idx_transactions_company_date ON transactions(company_id, business_date);
CREATE INDEX IF NOT EXISTS idx_transactions_reversed ON transactions(company_id, reversed_transaction_id);
CREATE INDEX IF NOT EXISTS idx_transactions_member_id ON transactions(member_id);
CREATE INDEX IF NOT EXISTS idx_components_transaction ON transaction_components(company_id, transaction_id);
CREATE INDEX IF NOT EXISTS idx_components_loan ON transaction_components(company_id, loan_id);
CREATE INDEX IF NOT EXISTS idx_components_savings ON transaction_components(company_id, savings_account_id);
CREATE INDEX IF NOT EXISTS idx_loans_company_member ON loans(company_id, member_id);
CREATE INDEX IF NOT EXISTS idx_savings_company_member ON savings_accounts(company_id, member_id);
CREATE INDEX IF NOT EXISTS idx_audit_company_entity ON audit_events(company_id, entity_type, entity_id);
