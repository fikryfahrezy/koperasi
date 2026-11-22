-- Penabung Non Anggota (SIMPANAN nomor 137 dst.) are not
-- members, as for Non Anggota borrowers on loans: their savings account has no
-- member, only the saver's name (holder_name). They only save Manasuka: no
-- Simpanan Pokok/Wajib, no SHU or Insentif. Neraca shows their balance as
-- Simpanan Non Anggota.
-- member_id is NOT NULL, so SQLite cannot relax it in place; the table is
-- rebuilt. The view reading it is dropped first and recreated below; the
-- migration runner turns foreign keys off and checks them before commit.

DROP VIEW IF EXISTS savings_balances;

CREATE TABLE savings_accounts_new (
  id TEXT PRIMARY KEY,
  company_id TEXT NOT NULL REFERENCES companies(id),
  -- Empty for a penabung Non Anggota.
  member_id TEXT REFERENCES members(id),
  -- Name of a penabung Non Anggota; empty for a member's account.
  holder_name TEXT,
  account_type TEXT NOT NULL,
  UNIQUE(company_id, member_id, account_type),
  CHECK (
    (member_id IS NOT NULL AND holder_name IS NULL)
    OR (member_id IS NULL AND TRIM(COALESCE(holder_name, '')) <> '' AND account_type = 'MANASUKA')
  )
);

INSERT INTO savings_accounts_new (rowid, id, company_id, member_id, holder_name, account_type)
SELECT rowid, id, company_id, member_id, NULL, account_type
FROM savings_accounts;

DROP TABLE savings_accounts;
ALTER TABLE savings_accounts_new RENAME TO savings_accounts;

CREATE INDEX idx_savings_company_member ON savings_accounts(company_id, member_id);

-- One account per penabung Non Anggota name (case-insensitive).
CREATE UNIQUE INDEX idx_savings_company_holder
  ON savings_accounts(company_id, holder_name COLLATE NOCASE)
  WHERE member_id IS NULL;

CREATE VIEW savings_balances AS
SELECT
  sa.company_id,
  sa.id AS savings_account_id,
  sa.member_id,
  sa.holder_name,
  sa.account_type,
  COALESCE(SUM(CASE c.component_type
    WHEN 'SAVINGS_WITHDRAWAL' THEN -c.amount
    ELSE c.amount
  END), 0) AS balance
FROM savings_accounts sa
LEFT JOIN effective_components c
  ON c.company_id = sa.company_id AND c.savings_account_id = sa.id
GROUP BY sa.company_id, sa.id;
