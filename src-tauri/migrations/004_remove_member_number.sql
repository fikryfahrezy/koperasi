PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;
BEGIN;

ALTER TABLE members RENAME TO members_number_legacy;

CREATE TABLE members (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  name TEXT NOT NULL,
  joined_at TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(company_id, id)
);

INSERT INTO members (id, company_id, name, joined_at, status, created_at)
SELECT id, company_id, name, joined_at, status, created_at
FROM members_number_legacy;

DROP TABLE members_number_legacy;

CREATE INDEX idx_members_company ON members(company_id);

COMMIT;
PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;
