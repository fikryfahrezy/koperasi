-- Penabung Non Anggota keluar dari tabel anggota (migrasi 008), seperti peminjam Non Anggota
-- pada pinjaman: rekening Manasuka mereka tetap, tanpa anggota, dengan nama penabung.
-- Mereka di sheet SIMPANAN nomor 137 dst., tanpa simpanan pokok dan wajib, dan jumlah manasuka
-- mereka sama dengan Simpanan Non Anggota di NERACA (876.000):
--   A0137 600.000, A0138 50.000, A0139 60.000, A0140 50.000, A0143 0, A0144 116.000.
-- Yang berubah:
--   * rekening Manasuka mereka: member_id dikosongkan, holder_name diisi nama anggota;
--   * rekening Pokok dan Wajib mereka (saldo 0, tanpa transaksi) dihapus;
--   * transaksi mereka (saldo awal simpanan 2026): member_id dikosongkan, nama tetap;
--   * baris anggota mereka dihapus. Setiap orang dicatat di audit_events.
-- Saldo simpanan dan Kas tidak berubah. Simpanan Non Anggota di Neraca kini dihitung dari
-- rekening ini; isian NON_MEMBER_SAVINGS tidak dipakai lagi.
-- Dicocokkan per id dan nomor anggota (tanpa nama, agar nama tidak masuk Git). Hanya jalan kalau
-- keenamnya masih anggota dengan nomor itu dan tanpa pokok, wajib, maupun pinjaman. Database
-- tanpa satu pun dari mereka (instalasi baru, e2e) tidak diubah; database yang hanya sebagian
-- cocok membuat skrip gagal tanpa mengubah apa pun (CHECK di guard044).

CREATE TEMP TABLE penabung044 (member_id TEXT PRIMARY KEY, member_number TEXT NOT NULL);

INSERT INTO penabung044 (member_id, member_number) VALUES
  ('default-M137', 'A0137'),
  ('default-M138', 'A0138'),
  ('default-M139', 'A0139'),
  ('default-M140', 'A0140'),
  ('default-M143', 'A0143'),
  ('default-M144', 'A0144');

-- Semua cocok, atau tidak ada satu pun (database lain): selain itu skrip gagal (CHECK).
CREATE TEMP TABLE guard044 (state TEXT NOT NULL CHECK (state IN ('kosong', 'cocok')));

INSERT INTO guard044 (state)
SELECT CASE
  WHEN NOT EXISTS (SELECT 1 FROM members m JOIN penabung044 p ON p.member_id = m.id) THEN 'kosong'
  WHEN (SELECT COUNT(*) FROM members m JOIN penabung044 p ON p.member_id = m.id AND p.member_number = m.member_number
          WHERE m.company_id = 'default') = 6
   AND NOT EXISTS (
     SELECT 1 FROM savings_balances b JOIN penabung044 p ON p.member_id = b.member_id
     WHERE b.account_type IN ('POKOK', 'WAJIB') AND b.balance <> 0)
   AND NOT EXISTS (
     SELECT 1 FROM transaction_components c
     JOIN savings_accounts sa ON sa.id = c.savings_account_id
     JOIN penabung044 p ON p.member_id = sa.member_id
     WHERE sa.account_type IN ('POKOK', 'WAJIB'))
   AND NOT EXISTS (SELECT 1 FROM loans l JOIN penabung044 p ON p.member_id = l.member_id)
   AND NOT EXISTS (SELECT 1 FROM transaction_groups g JOIN penabung044 p ON p.member_id = g.member_id)
   AND (SELECT COUNT(*) FROM savings_accounts sa JOIN penabung044 p ON p.member_id = sa.member_id
          WHERE sa.account_type = 'MANASUKA') = 6
  THEN 'cocok'
  ELSE 'beda'
END;

DELETE FROM penabung044 WHERE NOT EXISTS (SELECT 1 FROM guard044 WHERE state = 'cocok');

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT 'default', 'MEMBER', m.id, 'UPDATED',
  json_object('name', m.name, 'memberNumber', m.member_number, 'joinedAt', m.joined_at),
  json_object('nonMember', json('true'), 'savingsAccountId', 'SA-' || m.id || '-MANASUKA',
    'note', 'Penabung Non Anggota: keluar dari data anggota, rekening Manasuka tetap'),
  'Data script 044_penabung_non_anggota'
FROM members m JOIN penabung044 p ON p.member_id = m.id;

UPDATE savings_accounts
SET member_id = NULL,
    holder_name = (SELECT m.name FROM members m WHERE m.id = savings_accounts.member_id)
WHERE account_type = 'MANASUKA' AND member_id IN (SELECT member_id FROM penabung044);

DELETE FROM savings_accounts
WHERE account_type IN ('POKOK', 'WAJIB') AND member_id IN (SELECT member_id FROM penabung044);

-- Transaksi append-only: trigger dilepas selama perubahan, lalu dibuat ulang persis seperti semula.
DROP TRIGGER transactions_append_only_update;

UPDATE transactions SET member_id = NULL
WHERE member_id IN (SELECT member_id FROM penabung044);

CREATE TRIGGER transactions_append_only_update
BEFORE UPDATE ON transactions
BEGIN
  SELECT RAISE(ABORT, 'Transaksi tidak dapat diubah; buat reversal.');
END;

DELETE FROM members WHERE id IN (SELECT member_id FROM penabung044);

DROP TABLE penabung044;
DROP TABLE guard044;
