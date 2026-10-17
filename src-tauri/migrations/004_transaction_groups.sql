-- Imported months. The 2026 import wrote a member's Buku Kas rows (cash only)
-- and their loan/savings movements (per member per month) as separate rows; a
-- group holds one member's month, and Hapus of a Buku Kas row also cancels the
-- movements paired with it inside the group. Insert-only, like the ledger itself.
CREATE TABLE IF NOT EXISTS transaction_groups (
  company_id TEXT NOT NULL REFERENCES companies(id),
  -- e.g. "IMPORT-2026-02-default-M120".
  group_id TEXT NOT NULL,
  -- Shown to the user; empty shows "<member or borrower name> · <month>".
  label TEXT NOT NULL,
  -- Whose transactions these are: a member, or a loan not linked to a member.
  member_id TEXT REFERENCES members(id),
  loan_id TEXT REFERENCES loans(id),
  transaction_id TEXT NOT NULL REFERENCES transactions(id),
  PRIMARY KEY (company_id, transaction_id)
);

CREATE INDEX IF NOT EXISTS idx_transaction_groups_group
  ON transaction_groups(company_id, group_id);

CREATE TRIGGER IF NOT EXISTS transaction_groups_append_only_update
BEFORE UPDATE ON transaction_groups
BEGIN
  SELECT RAISE(ABORT, 'transaction_groups is append-only');
END;

CREATE TRIGGER IF NOT EXISTS transaction_groups_append_only_delete
BEFORE DELETE ON transaction_groups
BEGIN
  SELECT RAISE(ABORT, 'transaction_groups is append-only');
END;
