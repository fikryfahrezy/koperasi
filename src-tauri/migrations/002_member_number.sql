-- ID Anggota: entered by the user when registering a member and editable later.
-- Unique per company (case-insensitive); members registered before this column
-- existed are filled by data/001_prefill_member_number.sql.
ALTER TABLE members ADD COLUMN member_number TEXT;

CREATE UNIQUE INDEX idx_members_company_number
  ON members(company_id, member_number COLLATE NOCASE)
  WHERE member_number IS NOT NULL;
