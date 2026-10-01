PRAGMA foreign_keys = OFF;
BEGIN;

CREATE TABLE companies (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO companies (id, name) VALUES
  ('default', 'Koperasi Bina Sejahtera'),
  ('testing', 'Perusahaan Pengujian');

ALTER TABLE app_meta RENAME TO app_meta_legacy;
ALTER TABLE members RENAME TO members_legacy;
ALTER TABLE savings_accounts RENAME TO savings_accounts_legacy;
ALTER TABLE loans RENAME TO loans_legacy;
ALTER TABLE transactions RENAME TO transactions_legacy;
ALTER TABLE transaction_components RENAME TO transaction_components_legacy;
ALTER TABLE cash_postings RENAME TO cash_postings_legacy;
ALTER TABLE audit_events RENAME TO audit_events_legacy;
ALTER TABLE periods RENAME TO periods_legacy;
ALTER TABLE parameters RENAME TO parameters_legacy;

CREATE TABLE app_meta (
  company_id TEXT NOT NULL REFERENCES companies(id),
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (company_id, key)
);

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

CREATE TABLE audit_events (
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

CREATE TABLE periods (
  company_id TEXT NOT NULL REFERENCES companies(id),
  period TEXT NOT NULL,
  status TEXT NOT NULL,
  locked_by TEXT,
  locked_at TEXT,
  PRIMARY KEY(company_id, period)
);

CREATE TABLE parameters (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id TEXT NOT NULL REFERENCES companies(id),
  parameter_key TEXT NOT NULL,
  value REAL NOT NULL,
  effective_date TEXT NOT NULL,
  created_by TEXT NOT NULL,
  UNIQUE(company_id, parameter_key, effective_date)
);

INSERT INTO app_meta SELECT 'default', key, value FROM app_meta_legacy;
INSERT INTO members SELECT id, 'default', member_number, name, joined_at, status, created_at FROM members_legacy;
INSERT INTO savings_accounts SELECT id, 'default', member_id, account_type, balance, status FROM savings_accounts_legacy;
INSERT INTO loans SELECT id, 'default', member_id, member_name, plafond, balance, rate_annual, tenor, interest_type, realization_date, due_date, status, created_at FROM loans_legacy;
INSERT INTO transactions SELECT id, 'default', business_date, display_date, display_time, member_id, member_name, transaction_type, description, reference, direction, amount, status, reversed_transaction_id, actor, created_at FROM transactions_legacy;
INSERT INTO transaction_components SELECT id, 'default', transaction_id, component_type, label, amount, loan_id, savings_account_id FROM transaction_components_legacy;
INSERT INTO cash_postings SELECT id, 'default', transaction_id, direction, amount, created_at FROM cash_postings_legacy;
INSERT INTO audit_events SELECT id, 'default', entity_type, entity_id, action, before_json, after_json, actor, created_at FROM audit_events_legacy;
INSERT INTO periods SELECT 'default', period, status, locked_by, locked_at FROM periods_legacy;
INSERT INTO parameters SELECT id, 'default', parameter_key, value, effective_date, created_by FROM parameters_legacy;
INSERT INTO parameters (company_id, parameter_key, value, effective_date, created_by)
  SELECT 'testing', parameter_key, value, effective_date, 'System Multi-company' FROM parameters_legacy;

DROP TABLE transaction_components_legacy;
DROP TABLE cash_postings_legacy;
DROP TABLE transactions_legacy;
DROP TABLE savings_accounts_legacy;
DROP TABLE loans_legacy;
DROP TABLE members_legacy;
DROP TABLE audit_events_legacy;
DROP TABLE periods_legacy;
DROP TABLE parameters_legacy;
DROP TABLE app_meta_legacy;

CREATE INDEX idx_members_company ON members(company_id);
CREATE INDEX idx_transactions_company_date ON transactions(company_id, business_date);
CREATE INDEX idx_transactions_member_id ON transactions(member_id);
CREATE INDEX idx_loans_company_member ON loans(company_id, member_id);
CREATE INDEX idx_savings_company_member ON savings_accounts(company_id, member_id);
CREATE INDEX idx_audit_company_entity ON audit_events(company_id, entity_type, entity_id);

COMMIT;
PRAGMA foreign_keys = ON;
