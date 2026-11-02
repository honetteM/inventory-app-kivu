CREATE TABLE cash_account_openings (
    id                   TEXT PRIMARY KEY,
    business_id          TEXT NOT NULL,
    account_id           TEXT NOT NULL,
    opening_amount       INTEGER NOT NULL CHECK (opening_amount >= 0),
    opened_at             TEXT NOT NULL,
    entered_by_user_id   TEXT NOT NULL,
    authorized_by_user_id TEXT NOT NULL,
    recorded_at           TEXT NOT NULL,
    UNIQUE (business_id, account_id),
    FOREIGN KEY (business_id, account_id)
        REFERENCES cash_accounts (business_id, id),
    FOREIGN KEY (business_id, entered_by_user_id)
        REFERENCES users (business_id, id),
    FOREIGN KEY (business_id, authorized_by_user_id)
        REFERENCES users (business_id, id)
);

CREATE TRIGGER prevent_cash_account_opening_update
BEFORE UPDATE ON cash_account_openings
BEGIN
    SELECT RAISE(ABORT, 'cash account openings are immutable; record an authorized correction');
END;

CREATE TRIGGER prevent_cash_account_opening_delete
BEFORE DELETE ON cash_account_openings
BEGIN
    SELECT RAISE(ABORT, 'cash account openings are permanent; record an authorized correction');
END;

ALTER TABLE sale_payments ADD COLUMN cash_account_id TEXT REFERENCES cash_accounts (id);
ALTER TABLE customer_debt_payments ADD COLUMN cash_account_id TEXT REFERENCES cash_accounts (id);
ALTER TABLE purchase_payments ADD COLUMN cash_account_id TEXT REFERENCES cash_accounts (id);

CREATE TRIGGER validate_sale_payment_account
BEFORE INSERT ON sale_payments
WHEN NEW.cash_account_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM sales s
    JOIN cash_accounts a
      ON a.id = NEW.cash_account_id AND a.business_id = s.business_id
    JOIN cash_account_openings o
      ON o.account_id = a.id AND o.business_id = a.business_id
    WHERE s.id = NEW.sale_id AND a.is_active = 1
      AND a.account_type = CASE NEW.payment_method
          WHEN 'cash' THEN 'cash_drawer'
          WHEN 'bank_transfer' THEN 'bank'
          WHEN 'mtn_mobile_money' THEN 'mtn_wallet'
          WHEN 'airtel_money' THEN 'airtel_wallet'
      END
)
BEGIN
    SELECT RAISE(ABORT, 'sale collection requires an active, opened account matching its payment method');
END;

CREATE TRIGGER validate_customer_repayment_account
BEFORE INSERT ON customer_debt_payments
WHEN NEW.cash_account_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM cash_accounts a
    JOIN cash_account_openings o
      ON o.account_id = a.id AND o.business_id = a.business_id
    WHERE a.id = NEW.cash_account_id AND a.business_id = NEW.business_id
      AND a.is_active = 1
      AND a.account_type = CASE NEW.payment_method
          WHEN 'cash' THEN 'cash_drawer'
          WHEN 'bank_transfer' THEN 'bank'
          WHEN 'mtn_mobile_money' THEN 'mtn_wallet'
          WHEN 'airtel_money' THEN 'airtel_wallet'
      END
)
BEGIN
    SELECT RAISE(ABORT, 'customer repayment requires an active, opened account matching its payment method');
END;

CREATE TRIGGER validate_supplier_payment_account
BEFORE INSERT ON purchase_payments
WHEN NEW.cash_account_id IS NULL OR NOT EXISTS (
    SELECT 1
    FROM cash_accounts a
    JOIN cash_account_openings o
      ON o.account_id = a.id AND o.business_id = a.business_id
    WHERE a.id = NEW.cash_account_id AND a.business_id = NEW.business_id
      AND a.is_active = 1
      AND a.account_type = CASE NEW.payment_method
          WHEN 'cash' THEN 'cash_drawer'
          WHEN 'bank_transfer' THEN 'bank'
          WHEN 'mtn_mobile_money' THEN 'mtn_wallet'
          WHEN 'airtel_money' THEN 'airtel_wallet'
      END
)
BEGIN
    SELECT RAISE(ABORT, 'supplier payment requires an active, opened account matching its payment method');
END;

CREATE TRIGGER validate_cash_movement_opening
BEFORE INSERT ON cash_movements
WHEN NOT EXISTS (
    SELECT 1 FROM cash_account_openings o
    WHERE o.business_id = NEW.business_id AND o.account_id = NEW.account_id
      AND NEW.recorded_at >= o.recorded_at
)
BEGIN
    SELECT RAISE(ABORT, 'cash account has no authorized opening balance');
END;

CREATE TRIGGER validate_cash_movement_funds
BEFORE INSERT ON cash_movements
WHEN NEW.direction = 'out' AND COALESCE((
    SELECT o.opening_amount + COALESCE(SUM(
        CASE WHEN m.direction = 'in' THEN m.amount ELSE -m.amount END
    ), 0)
    FROM cash_account_openings o
    LEFT JOIN cash_movements m
      ON m.business_id = o.business_id AND m.account_id = o.account_id
      AND m.recorded_at >= o.recorded_at
    WHERE o.business_id = NEW.business_id AND o.account_id = NEW.account_id
    GROUP BY o.opening_amount
), -1) < NEW.amount
BEGIN
    SELECT RAISE(ABORT, 'insufficient verified funds in the selected cash account');
END;

CREATE TRIGGER validate_sale_cash_movement_source
BEFORE INSERT ON cash_movements
WHEN NEW.source_type = 'sale_payment' AND NOT EXISTS (
    SELECT 1
    FROM sale_payments sp
    JOIN sales s ON s.id = sp.sale_id
    WHERE sp.id = NEW.source_id AND s.business_id = NEW.business_id
      AND sp.cash_account_id = NEW.account_id
      AND sp.amount = NEW.amount AND sp.created_at = NEW.occurred_at
      AND NEW.movement_type = 'sale_collection' AND NEW.direction = 'in'
)
BEGIN
    SELECT RAISE(ABORT, 'sale collection movement must match its recorded payment');
END;

CREATE TRIGGER validate_customer_repayment_cash_movement_source
BEFORE INSERT ON cash_movements
WHEN NEW.source_type = 'customer_debt_payment' AND NOT EXISTS (
    SELECT 1 FROM customer_debt_payments p
    WHERE p.id = NEW.source_id AND p.business_id = NEW.business_id
      AND p.cash_account_id = NEW.account_id
      AND p.amount = NEW.amount AND p.created_at = NEW.occurred_at
      AND NEW.movement_type = 'customer_repayment' AND NEW.direction = 'in'
)
BEGIN
    SELECT RAISE(ABORT, 'customer repayment movement must match its recorded payment');
END;

CREATE TRIGGER validate_supplier_payment_cash_movement_source
BEFORE INSERT ON cash_movements
WHEN NEW.source_type = 'purchase_payment' AND NOT EXISTS (
    SELECT 1 FROM purchase_payments p
    WHERE p.id = NEW.source_id AND p.business_id = NEW.business_id
      AND p.cash_account_id = NEW.account_id
      AND p.amount = NEW.amount AND p.created_at = NEW.occurred_at
      AND NEW.movement_type = 'supplier_payment' AND NEW.direction = 'out'
)
BEGIN
    SELECT RAISE(ABORT, 'supplier payment movement must match its recorded payment');
END;

CREATE TRIGGER validate_cash_transfer_openings
BEFORE UPDATE OF status ON cash_transfers
WHEN OLD.status = 'pending' AND NEW.status = 'confirmed'
AND (
    NOT EXISTS (
        SELECT 1 FROM cash_account_openings o
        WHERE o.business_id = NEW.business_id AND o.account_id = NEW.from_account_id
    )
    OR NOT EXISTS (
        SELECT 1 FROM cash_account_openings o
        WHERE o.business_id = NEW.business_id AND o.account_id = NEW.to_account_id
    )
)
BEGIN
    SELECT RAISE(ABORT, 'both cash transfer accounts require authorized opening balances');
END;

CREATE TRIGGER validate_cash_transfer_active_accounts
BEFORE UPDATE OF status ON cash_transfers
WHEN OLD.status = 'pending' AND NEW.status = 'confirmed'
AND (
    NOT EXISTS (
        SELECT 1 FROM cash_accounts a
        WHERE a.business_id = NEW.business_id AND a.id = NEW.from_account_id
          AND a.is_active = 1
    )
    OR NOT EXISTS (
        SELECT 1 FROM cash_accounts a
        WHERE a.business_id = NEW.business_id AND a.id = NEW.to_account_id
          AND a.is_active = 1
    )
)
BEGIN
    SELECT RAISE(ABORT, 'both cash transfer accounts must be active');
END;

INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
JOIN permissions p ON p.id = 'cash_accounts.balance.read'
WHERE r.name = 'manager';
