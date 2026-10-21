-- Four new members' Simpanan Pokok of 50.000 on 02-04-2026 were typed in the
-- Debet column of DATA SIMPANAN (its saldo formula adds them), so the import
-- recorded them as withdrawals and their Pokok balance became -50.000. Buku Kas
-- shows no such payout. The ledger is append-only, so each member's imported
-- April savings transaction is reversed and recorded again with that
-- component as a deposit, in the same import group. Only acts where the
-- transaction is still in effect and has exactly that withdrawal; ids only.
CREATE TEMP TABLE fix_principal (old_id TEXT PRIMARY KEY, reversal_id TEXT NOT NULL, new_id TEXT NOT NULL);
WITH f(key) AS (VALUES ('M133-2026-04'), ('M134-2026-04'), ('M135-2026-04'), ('M136-2026-04'))
INSERT INTO fix_principal (old_id, reversal_id, new_id)
SELECT t.id, 'default-FIX014-REV-' || f.key, 'default-FIX014-' || f.key
FROM f
JOIN effective_transactions t ON t.company_id = 'default' AND t.id = 'default-MIG-SIM-' || f.key
WHERE (
  SELECT COUNT(*) FROM transaction_components c
  JOIN savings_accounts sa ON sa.id = c.savings_account_id
  WHERE c.company_id = t.company_id AND c.transaction_id = t.id
    AND c.component_type = 'SAVINGS_WITHDRAWAL'
) = 1
AND EXISTS (
  SELECT 1 FROM transaction_components c
  JOIN savings_accounts sa ON sa.id = c.savings_account_id
  WHERE c.company_id = t.company_id AND c.transaction_id = t.id
    AND c.component_type = 'SAVINGS_WITHDRAWAL' AND sa.account_type = 'POKOK' AND c.amount = 50000
);

-- Reversal, as Hapus writes it: opposite direction, REVERSAL components.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, reversed_transaction_id, actor)
SELECT f.reversal_id, t.company_id, t.business_date, t.display_date, t.display_time, t.member_id, t.member_name, t.channel,
  'Hapus · ' || t.description, t.reference || '/REV/014', CASE t.direction WHEN 'Masuk' THEN 'Keluar' ELSE 'Masuk' END,
  t.amount, t.id, 'Data script 014_fix_april_principal_deposits'
FROM fix_principal f JOIN transactions t ON t.id = f.old_id;
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT c.company_id, f.reversal_id, 'REVERSAL', 'Hapus · ' || c.label, c.amount, c.loan_id, c.savings_account_id
FROM fix_principal f JOIN transaction_components c ON c.transaction_id = f.old_id
ORDER BY c.id;

-- The same month again, with the Pokok withdrawal as a deposit.
INSERT INTO transactions (id, company_id, business_date, display_date, display_time, member_id, member_name, channel, description, reference, direction, amount, actor)
SELECT f.new_id, t.company_id, t.business_date, t.display_date, t.display_time, t.member_id, t.member_name, t.channel,
  t.description, t.reference || '/014', t.direction, t.amount, 'Data script 014_fix_april_principal_deposits'
FROM fix_principal f JOIN transactions t ON t.id = f.old_id;
INSERT INTO transaction_components (company_id, transaction_id, component_type, label, amount, loan_id, savings_account_id)
SELECT c.company_id, f.new_id,
  CASE WHEN c.component_type = 'SAVINGS_WITHDRAWAL' THEN 'SAVINGS_DEPOSIT' ELSE c.component_type END,
  c.label, c.amount, c.loan_id, c.savings_account_id
FROM fix_principal f JOIN transaction_components c ON c.transaction_id = f.old_id
ORDER BY c.id;

INSERT INTO transaction_groups (company_id, group_id, label, member_id, loan_id, transaction_id)
SELECT g.company_id, g.group_id, g.label, g.member_id, g.loan_id, f.new_id
FROM fix_principal f JOIN transaction_groups g ON g.transaction_id = f.old_id;

INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', f.old_id, 'REVERSED', json_object('reversalId', f.reversal_id, 'replacementId', f.new_id), 'Data script 014_fix_april_principal_deposits'
FROM fix_principal f;
INSERT INTO audit_events (company_id, entity_type, entity_id, action, after_json, actor)
SELECT 'default', 'TRANSACTION', f.new_id, 'CREATED', json_object('replaces', f.old_id, 'reason', 'Simpanan Pokok 50.000 tertulis di kolom Debet DATA SIMPANAN'), 'Data script 014_fix_april_principal_deposits'
FROM fix_principal f;
DROP TABLE fix_principal;
