-- Pinjaman suplesi 15 jt realisasi 06-10-2026 (LOAN-1792467992586-4) tercatat dengan bunga
-- Menurun, sedangkan sheet PINJAMAN BULANAN 2026 baris 99 menulis Plat (bunga 300.000 tiap
-- bulan, 2% dari plafond). Form waktu itu default Menurun; pinjaman yang sama yang dicatat
-- pertama kali (sudah di-Hapus) juga Plat. Hanya mengubah jika masih Menurun.
INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT company_id, 'LOAN', id, 'UPDATED', json_object('interestType', interest_type), json_object('interestType', 'Plat'), 'Data script 031_bunga_plat_dewi_15jt_okt_2026'
FROM loans WHERE company_id = 'default' AND id = 'LOAN-1792467992586-4' AND interest_type = 'Menurun';
UPDATE loans SET interest_type = 'Plat'
WHERE company_id = 'default' AND id = 'LOAN-1792467992586-4' AND interest_type = 'Menurun';
