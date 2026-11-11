-- 1) Pinjaman AISYAH / BU ADE (L011, L012, L083, L111, L112, L163, L164) ke anggota
--    default-M068. Dulu ditautkan oleh script 011 yang hanya jalan di satu database dan
--    tidak ikut 013/020, jadi di database lain bisa masih kosong
--    (notes/member-vs-borrower.md bagian 4). Hanya mengisi tautan yang kosong, dan hanya
--    bila anggotanya ada.
-- 2) Rekomendasi pinjaman Non Anggota L202 = SALMON, seperti kolom FU sheet PINJAMAN
--    BULANAN 2026 (10-11-2026) baris 240. Hanya bila rekomendasinya masih kosong.
CREATE TEMP TABLE link029 (loan_id TEXT PRIMARY KEY, member_id TEXT NOT NULL);
INSERT INTO link029 (loan_id, member_id) VALUES
  ('default-L011', 'default-M068'), ('default-L012', 'default-M068'),
  ('default-L083', 'default-M068'), ('default-L111', 'default-M068'),
  ('default-L112', 'default-M068'), ('default-L163', 'default-M068'),
  ('default-L164', 'default-M068');
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT l.company_id, 'LOAN', l.id, 'UPDATED', json_object('memberId', k.member_id), 'Data script 029_link_aisyah_rekomendasi_udin'
FROM loans l JOIN link029 k ON k.loan_id = l.id
WHERE l.company_id = 'default' AND COALESCE(l.member_id, '') = ''
  AND EXISTS (SELECT 1 FROM members m WHERE m.id = k.member_id AND m.company_id = l.company_id);
UPDATE loans
SET member_id = (SELECT k.member_id FROM link029 k WHERE k.loan_id = loans.id)
WHERE company_id = 'default' AND COALESCE(member_id, '') = ''
  AND id IN (SELECT loan_id FROM link029)
  AND EXISTS (SELECT 1 FROM members m JOIN link029 k ON k.member_id = m.id WHERE k.loan_id = loans.id AND m.company_id = loans.company_id);
DROP TABLE link029;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT company_id, 'LOAN', id, 'UPDATED', json_object('recommender', 'SALMON'), 'Data script 029_link_aisyah_rekomendasi_udin'
FROM loans WHERE company_id = 'default' AND id = 'default-L202' AND COALESCE(TRIM(recommender), '') = '';
UPDATE loans SET recommender = 'SALMON'
WHERE company_id = 'default' AND id = 'default-L202' AND COALESCE(TRIM(recommender), '') = '';
