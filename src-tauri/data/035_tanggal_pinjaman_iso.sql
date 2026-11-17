-- Tanggal realisasi dan jatuh tempo pinjaman diseragamkan menjadi YYYY-MM-DD.
-- Impor sheet menyimpan "05-Aug-2026" (dan satu "07 Agust 2024"), pinjaman dari
-- aplikasi menyimpan "2026-08-05". Campuran ini membuat tampilan tidak seragam
-- dan urutan "pinjaman tertua dulu" saat setoran salah, karena tanggal diurutkan
-- sebagai teks. Nama bulan Inggris dan Indonesia dikenali; kalau masih ada tanggal
-- yang tidak bisa dibaca, skrip ini gagal dan tidak mengubah apa pun.

CREATE TEMP TABLE loan_dates035 (
  company_id TEXT NOT NULL,
  loan_id TEXT PRIMARY KEY,
  column_name TEXT NOT NULL,
  old_value TEXT NOT NULL,
  new_value TEXT
);

-- "07 Agust 2024" -> "07-Agust-2024", lalu hari, 3 huruf pertama bulan, tahun.
WITH dates AS (
  SELECT company_id, id, 'realization_date' AS column_name, realization_date AS value
  FROM loans
  UNION ALL
  SELECT company_id, id, 'due_date', due_date
  FROM loans
),
pending AS (
  SELECT *, replace(trim(value), ' ', '-') AS norm
  FROM dates
  WHERE value NOT GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
),
parts AS (
  SELECT *,
    substr(norm, 1, instr(norm, '-') - 1) AS day,
    substr(norm, instr(norm, '-') + 1) AS rest
  FROM pending
),
split AS (
  SELECT *,
    lower(substr(rest, 1, 3)) AS month_name,
    substr(rest, instr(rest, '-') + 1) AS year
  FROM parts
),
parsed AS (
  SELECT *,
    CASE month_name
      WHEN 'jan' THEN 1 WHEN 'feb' THEN 2 WHEN 'mar' THEN 3 WHEN 'apr' THEN 4
      WHEN 'may' THEN 5 WHEN 'mei' THEN 5 WHEN 'jun' THEN 6 WHEN 'jul' THEN 7
      WHEN 'aug' THEN 8 WHEN 'agu' THEN 8 WHEN 'sep' THEN 9 WHEN 'oct' THEN 10
      WHEN 'okt' THEN 10 WHEN 'nov' THEN 11 WHEN 'dec' THEN 12 WHEN 'des' THEN 12
    END AS month
  FROM split
)
INSERT INTO loan_dates035 (company_id, loan_id, column_name, old_value, new_value)
SELECT company_id, id || ':' || column_name, column_name, value,
  CASE
    WHEN month IS NOT NULL
      AND day GLOB '[0-9]*' AND CAST(day AS INTEGER) BETWEEN 1 AND 31
      AND year GLOB '[0-9][0-9][0-9][0-9]'
    THEN printf('%04d-%02d-%02d', CAST(year AS INTEGER), month, CAST(day AS INTEGER))
  END
FROM parsed;

-- Gagal kalau ada tanggal yang tidak terbaca atau bukan tanggal yang sah
-- (mis. 31-Feb): CHECK di tabel ini menolak baris pertama yang bermasalah.
CREATE TEMP TABLE loan_dates035_guard (unreadable INTEGER CHECK (unreadable = 0));
INSERT INTO loan_dates035_guard
SELECT COUNT(*) FROM loan_dates035
WHERE new_value IS NULL OR date(new_value) IS NOT new_value;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, before_json, after_json, actor)
SELECT
  company_id,
  'LOAN',
  substr(loan_id, 1, instr(loan_id, ':') - 1),
  'UPDATED',
  json_group_object(column_name, old_value),
  json_group_object(column_name, new_value),
  'Data script 035_tanggal_pinjaman_iso'
FROM loan_dates035
GROUP BY company_id, substr(loan_id, 1, instr(loan_id, ':') - 1);

UPDATE loans
SET realization_date = COALESCE((
  SELECT new_value FROM loan_dates035
  WHERE loan_id = loans.id || ':realization_date'
), realization_date),
due_date = COALESCE((
  SELECT new_value FROM loan_dates035
  WHERE loan_id = loans.id || ':due_date'
), due_date);

DROP TABLE loan_dates035_guard;
DROP TABLE loan_dates035;
