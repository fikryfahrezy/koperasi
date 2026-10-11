-- Rekomendasi: who recommended the loan (kolom EH of PINJAMAN BULANAN).
-- Existing loans are filled by data/008_prefill_loan_recommender.sql.
ALTER TABLE loans ADD COLUMN recommender TEXT;
