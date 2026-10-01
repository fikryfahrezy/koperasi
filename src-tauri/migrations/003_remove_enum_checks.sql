PRAGMA foreign_keys = OFF;
BEGIN;

ALTER TABLE members RENAME TO members_enum_legacy;
ALTER TABLE savings_accounts RENAME TO savings_accounts_enum_legacy;
ALTER TABLE loans RENAME TO loans_enum_legacy;
ALTER TABLE transactions RENAME TO transactions_enum_legacy;
ALTER TABLE transaction_components RENAME TO transaction_components_enum_legacy;
ALTER TABLE cash_postings RENAME TO cash_postings_enum_legacy;
ALTER TABLE periods RENAME TO periods_enum_legacy;

CREATE TABLE members (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  member_number TEXT NOT NULL,
  name TEXT NOT NULL,
  joined_at TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(company_id, member_number),
  UNIQUE(company_id, id)
);

CREATE TABLE savings_accounts (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  member_id TEXT NOT NULL REFERENCES members(id),
  account_type TEXT NOT NULL,
  balance INTEGER NOT NULL DEFAULT 0 CHECK (balance >= 0),
  status TEXT NOT NULL,
  UNIQUE(company_id, member_id, account_type)
);

CREATE TABLE loans (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  plafond INTEGER NOT NULL DEFAULT 0 CHECK (plafond >= 0),
  balance INTEGER NOT NULL DEFAULT 0 CHECK (balance >= 0),
  rate_annual REAL NOT NULL,
  tenor INTEGER NOT NULL CHECK (tenor > 0),
  interest_type TEXT NOT NULL,
  realization_date TEXT NOT NULL,
  due_date TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE transactions (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  business_date TEXT NOT NULL,
  display_date TEXT NOT NULL,
  display_time TEXT NOT NULL,
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  transaction_type TEXT NOT NULL,
  description TEXT NOT NULL,
  reference TEXT NOT NULL,
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  status TEXT NOT NULL,
  reversed_transaction_id TEXT REFERENCES transactions(id),
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(company_id, reference)
);

CREATE TABLE transaction_components (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  component_type TEXT NOT NULL,
  label TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  loan_id TEXT REFERENCES loans(id),
  savings_account_id TEXT REFERENCES savings_accounts(id)
);

CREATE TABLE cash_postings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE periods (
  company_id TEXT NOT NULL REFERENCES companies(id),
  period TEXT NOT NULL,
  status TEXT NOT NULL,
  locked_by TEXT,
  locked_at TEXT,
  PRIMARY KEY(company_id, period)
);

INSERT INTO members SELECT * FROM members_enum_legacy;
INSERT INTO savings_accounts SELECT * FROM savings_accounts_enum_legacy;
INSERT INTO loans SELECT * FROM loans_enum_legacy;
INSERT INTO transactions SELECT * FROM transactions_enum_legacy;
INSERT INTO transaction_components SELECT * FROM transaction_components_enum_legacy;
INSERT INTO cash_postings SELECT * FROM cash_postings_enum_legacy;
INSERT INTO periods SELECT * FROM periods_enum_legacy;

DROP TABLE transaction_components_enum_legacy;
DROP TABLE cash_postings_enum_legacy;
DROP TABLE transactions_enum_legacy;
DROP TABLE savings_accounts_enum_legacy;
DROP TABLE loans_enum_legacy;
DROP TABLE members_enum_legacy;
DROP TABLE periods_enum_legacy;

CREATE INDEX idx_members_company ON members(company_id);
CREATE INDEX idx_transactions_company_date ON transactions(company_id, business_date);
CREATE INDEX idx_transactions_member_id ON transactions(member_id);
CREATE INDEX idx_loans_company_member ON loans(company_id, member_id);
CREATE INDEX idx_savings_company_member ON savings_accounts(company_id, member_id);

COMMIT;
PRAGMA foreign_keys = ON;
