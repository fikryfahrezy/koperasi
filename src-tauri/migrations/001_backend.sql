PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS members (
  id TEXT PRIMARY KEY,
  member_number TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  joined_at TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS savings_accounts (
  id TEXT PRIMARY KEY,
  member_id TEXT NOT NULL REFERENCES members(id),
  account_type TEXT NOT NULL,
  balance INTEGER NOT NULL DEFAULT 0 CHECK (balance >= 0),
  status TEXT NOT NULL,
  UNIQUE(member_id, account_type)
);

CREATE TABLE IF NOT EXISTS loans (
  id TEXT PRIMARY KEY,
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

CREATE TABLE IF NOT EXISTS transactions (
  id TEXT PRIMARY KEY,
  business_date TEXT NOT NULL,
  display_date TEXT NOT NULL,
  display_time TEXT NOT NULL,
  member_id TEXT REFERENCES members(id),
  member_name TEXT NOT NULL,
  transaction_type TEXT NOT NULL,
  description TEXT NOT NULL,
  reference TEXT NOT NULL UNIQUE,
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  status TEXT NOT NULL,
  reversed_transaction_id TEXT REFERENCES transactions(id),
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transaction_components (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  component_type TEXT NOT NULL,
  label TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  loan_id TEXT REFERENCES loans(id),
  savings_account_id TEXT REFERENCES savings_accounts(id)
);

CREATE TABLE IF NOT EXISTS cash_postings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  direction TEXT NOT NULL,
  amount INTEGER NOT NULL CHECK (amount > 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS audit_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS periods (
  period TEXT PRIMARY KEY,
  status TEXT NOT NULL,
  locked_by TEXT,
  locked_at TEXT
);

CREATE TABLE IF NOT EXISTS parameters (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  parameter_key TEXT NOT NULL,
  value REAL NOT NULL,
  effective_date TEXT NOT NULL,
  created_by TEXT NOT NULL,
  UNIQUE(parameter_key, effective_date)
);

CREATE INDEX IF NOT EXISTS idx_transactions_business_date ON transactions(business_date);
CREATE INDEX IF NOT EXISTS idx_transactions_member_id ON transactions(member_id);
CREATE INDEX IF NOT EXISTS idx_loans_member_id ON loans(member_id);
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_events(entity_type, entity_id);
