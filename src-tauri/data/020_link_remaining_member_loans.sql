-- Links the Anggota loans left unlinked after 013 (notes/member-vs-borrower.md
-- groups 3 and 4) to their members, so they appear in Setoran anggota. Each
-- match is backed by Buku Kas rows; see review/uji-tautan-pinjaman-anggota.md.
-- Loan L040 is not in SIMPANAN and stays unlinked.
-- Only fills an empty link, only when the member exists. Ids only.
CREATE TEMP TABLE link_member_loans (loan_id TEXT PRIMARY KEY, member_id TEXT NOT NULL);
INSERT INTO link_member_loans (loan_id, member_id) VALUES
  ('default-L074', 'default-M008'),
  ('default-L095', 'default-M008'),
  ('default-L043', 'default-M132'),
  ('default-L140', 'default-M101'),
  ('default-L071', 'default-M066'),
  ('default-L069', 'default-M010'),
  ('default-L155', 'default-M010'),
  ('default-L067', 'default-M051'),
  ('default-L176', 'default-M051'),
  ('default-L059', 'default-M029'),
  ('default-L156', 'default-M029'),
  ('default-L034', 'default-M090'),
  ('default-L127', 'default-M090'),
  ('default-L128', 'default-M090'),
  ('default-L003', 'default-M036'),
  ('default-L100', 'default-M036'),
  ('default-L101', 'default-M036'),
  ('default-L077', 'default-M085'),
  ('default-L079', 'default-M085'),
  ('default-L162', 'default-M085'),
  ('default-L022', 'default-M083'),
  ('default-L237', 'default-M019');
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT l.company_id, 'LOAN', l.id, 'UPDATED', json_object('memberId', k.member_id), 'Data script 020_link_remaining_member_loans'
FROM loans l
JOIN link_member_loans k ON k.loan_id = l.id
WHERE l.company_id = 'default' AND COALESCE(l.member_id, '') = ''
  AND EXISTS (SELECT 1 FROM members m WHERE m.id = k.member_id AND m.company_id = l.company_id);
UPDATE loans
SET member_id = (SELECT k.member_id FROM link_member_loans k WHERE k.loan_id = loans.id)
WHERE company_id = 'default' AND COALESCE(member_id, '') = ''
  AND id IN (SELECT loan_id FROM link_member_loans)
  AND EXISTS (SELECT 1 FROM members m JOIN link_member_loans k ON k.member_id = m.id WHERE k.loan_id = loans.id AND m.company_id = loans.company_id);
DROP TABLE link_member_loans;
