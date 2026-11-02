ALTER TABLE expenses
    ADD COLUMN cash_account_id TEXT REFERENCES cash_accounts (id);

CREATE TRIGGER validate_expense_account
BEFORE INSERT ON expenses
WHEN NEW.cash_account_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM cash_accounts a
    JOIN cash_account_openings o
      ON o.business_id = a.business_id AND o.account_id = a.id
    WHERE a.business_id = NEW.business_id AND a.id = NEW.cash_account_id
      AND a.is_active = 1
      AND a.account_type = CASE NEW.payment_method
          WHEN 'cash' THEN 'cash_drawer'
          WHEN 'bank_transfer' THEN 'bank'
          WHEN 'mtn_mobile_money' THEN 'mtn_wallet'
          WHEN 'airtel_money' THEN 'airtel_wallet'
      END
)
BEGIN
    SELECT RAISE(ABORT, 'expense requires an active, opened account matching its payment method');
END;

CREATE TRIGGER validate_expense_cash_movement_source
BEFORE INSERT ON cash_movements
WHEN NEW.source_type = 'expense_payment' AND NOT EXISTS (
    SELECT 1
    FROM expenses e
    WHERE e.id = NEW.source_id AND e.business_id = NEW.business_id
      AND e.cash_account_id = NEW.account_id
      AND e.amount = NEW.amount
      AND e.created_at = NEW.occurred_at
      AND NEW.movement_type = 'expense_payment'
      AND NEW.direction = 'out'
)
BEGIN
    SELECT RAISE(ABORT, 'expense movement must match its recorded payment');
END;

CREATE TRIGGER prevent_linked_expense_update
BEFORE UPDATE ON expenses
WHEN OLD.cash_account_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'account-backed expenses are immutable; record a correction instead');
END;

CREATE TRIGGER prevent_linked_expense_delete
BEFORE DELETE ON expenses
WHEN OLD.cash_account_id IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'account-backed expenses are permanent; record a correction instead');
END;

INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
JOIN permissions p ON p.id IN ('expenses.read', 'expenses.manage')
WHERE r.name = 'manager';
