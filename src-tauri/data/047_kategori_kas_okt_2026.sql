-- Baris Buku Kas 2026 yang masih tanpa kategori diberi kategori seperti sheet BUKU KAS HARIAN
-- TAHUN 2026 (tab TAHUN 2026):
--   * Kaos PP 13 dan 14 Okt dan Sumbangan HUT PP BRI 19 Okt ada di kolom Y "Cadangan Laba"
--     -> Pakai Laba Ditahan (jumlah kolom Y jadi 9.250.000, sama dengan sheet);
--   * Transfer ke Tabungan 10 Sep dan 16 Okt -> Transfer ke Tabungan, seperti baris Transfer
--     lainnya;
--   * TAMBAHAN KAS 6 Okt -> Tambahan Kas, seperti baris TAMBAHAN KAS lainnya.
-- Kategori komponen diubah di tempat (jumlah, tanggal, uraian tetap; Saldo Kas tidak berubah),
-- setiap perubahan dicatat di audit_events. Dicocokkan per tanggal, uraian, arah, dan jumlah,
-- hanya kalau komponennya masih CASH dan barisnya belum dibalik. Baris keluar 2.000.000 tanpa
-- uraian tanggal 26 Agu dibiarkan: di sheet juga tanpa uraian dan kolom.
-- Database lain (instalasi baru, e2e) tidak punya baris ini, jadi tidak ada yang diubah.

CREATE TEMP TABLE kategori047 (
  tanggal TEXT NOT NULL,
  uraian TEXT NOT NULL,
  arah TEXT NOT NULL,
  jumlah INTEGER NOT NULL,
  ke TEXT NOT NULL,
  label TEXT NOT NULL
);

INSERT INTO kategori047 (tanggal, uraian, arah, jumlah, ke, label) VALUES
  ('2026-10-13', 'Pembelian Kaos PP sebanyak 5 kaos', 'Keluar', 750000, 'PAKAI_LABA_DITAHAN', 'Pakai Laba Ditahan'),
  ('2026-10-14', 'Pembelian 10 buah Koas PP', 'Keluar', 1500000, 'PAKAI_LABA_DITAHAN', 'Pakai Laba Ditahan'),
  ('2026-10-19', 'Sumbangan HUT PP BRI ke 57', 'Keluar', 1500000, 'PAKAI_LABA_DITAHAN', 'Pakai Laba Ditahan'),
  ('2026-09-10', 'Transfer ke  Tabungan', 'Keluar', 22915000, 'TRANSFER_KE_TABUNGAN', 'Transfer ke Tabungan'),
  ('2026-10-16', 'Transfer ke  Tabungan', 'Keluar', 500000, 'TRANSFER_KE_TABUNGAN', 'Transfer ke Tabungan'),
  ('2026-10-06', 'TAMBAHAN KAS', 'Masuk', 25000000, 'TAMBAHAN_KAS', 'Tambahan Kas');

CREATE TEMP TABLE ubah047 AS
SELECT c.id, c.transaction_id, c.component_type AS dari, c.label AS label_lama, k.ke, k.label
FROM transaction_components c
JOIN transactions t ON t.company_id = c.company_id AND t.id = c.transaction_id
JOIN kategori047 k
  ON k.tanggal = t.business_date AND k.uraian = t.description
 AND k.arah = t.direction AND k.jumlah = t.amount AND k.jumlah = c.amount
WHERE t.company_id = 'default'
  AND t.channel = 'KAS'
  AND t.member_id IS NULL
  AND c.component_type = 'CASH'
  AND t.reversed_transaction_id IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM transactions r
    WHERE r.company_id = t.company_id AND r.reversed_transaction_id = t.id
  );

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT
  'default',
  'TRANSACTION',
  transaction_id,
  'UPDATED',
  json_object('component_id', id, 'component_type', dari, 'label', label_lama),
  json_object('component_id', id, 'component_type', ke, 'label', label),
  'Data script 047_kategori_kas_okt_2026'
FROM ubah047;

DROP TRIGGER components_append_only_update;

UPDATE transaction_components
SET
  component_type = (SELECT ke FROM ubah047 u WHERE u.id = transaction_components.id),
  label = (SELECT label FROM ubah047 u WHERE u.id = transaction_components.id)
WHERE id IN (SELECT id FROM ubah047);

CREATE TRIGGER components_append_only_update
BEFORE UPDATE ON transaction_components
BEGIN
  SELECT RAISE(ABORT, 'Komponen transaksi tidak dapat diubah.');
END;

DROP TABLE ubah047;
DROP TABLE kategori047;
