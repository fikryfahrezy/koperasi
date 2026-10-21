-- Batas sisa pinjaman per peminjam: a borrower may hold several loans while
-- their remaining principal, with the new loan, stays within this amount.
-- The cash entry form only warns past it, since the board can make exceptions.
-- See notes/rules-and-formulas.md 1.12.
INSERT OR IGNORE INTO parameters (company_id, parameter_key, value, effective_date, created_by)
SELECT id, 'LOAN_MAX_OUTSTANDING', 25000000.0, '2026-01-01', 'Data script 015_loan_outstanding_limit'
FROM companies;
