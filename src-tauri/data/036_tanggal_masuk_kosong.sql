-- Tanggal masuk anggota hasil impor berisi teks "Migrasi 2026", bukan tanggal:
-- sheet SIMPANAN tidak punya tanggal masuk. Teks itu diganti kosong, yang artinya
-- "belum diketahui"; aplikasi sudah menampilkannya sebagai "—" dan Data anggota
-- bisa mengisinya. Tanggal yang sudah benar (YYYY-MM-DD) tidak diubah.

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT company_id, 'MEMBER', id, 'UPDATED',
  json_object('joinedAt', joined_at), json_object('joinedAt', ''),
  'Data script 036_tanggal_masuk_kosong'
FROM members
WHERE joined_at <> '' AND date(joined_at) IS NOT joined_at;

UPDATE members
SET joined_at = ''
WHERE joined_at <> '' AND date(joined_at) IS NOT joined_at;
