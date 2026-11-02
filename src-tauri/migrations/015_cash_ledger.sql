CREATE UNIQUE INDEX idx_branches_business_id
    ON branches (business_id, id);

CREATE UNIQUE INDEX idx_users_business_id
    ON users (business_id, id);

CREATE TABLE cash_accounts (
    id                  TEXT PRIMARY KEY,
    business_id         TEXT NOT NULL REFERENCES businesses (id),
    branch_id           TEXT,
    account_type        TEXT NOT NULL CHECK (account_type IN (
        'cash_drawer', 'bank', 'mtn_wallet', 'airtel_wallet', 'other'
    )),
    name                TEXT NOT NULL,
    responsible_user_id TEXT,
    is_active           INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE (business_id, id),
    FOREIGN KEY (business_id, branch_id)
        REFERENCES branches (business_id, id),
    FOREIGN KEY (business_id, responsible_user_id)
        REFERENCES users (business_id, id),
    CHECK (account_type != 'cash_drawer' OR responsible_user_id IS NOT NULL),
    CHECK (length(trim(name)) > 0)
);

CREATE INDEX idx_cash_accounts_business_active
    ON cash_accounts (business_id, is_active, account_type);
CREATE INDEX idx_cash_accounts_branch
    ON cash_accounts (business_id, branch_id, is_active);

CREATE TABLE cash_transfers (
    id                       TEXT PRIMARY KEY,
    business_id              TEXT NOT NULL REFERENCES businesses (id),
    from_account_id          TEXT NOT NULL,
    to_account_id            TEXT NOT NULL,
    amount                   INTEGER NOT NULL CHECK (amount > 0),
    status                   TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'confirmed', 'cancelled'
    )),
    occurred_at              TEXT NOT NULL,
    initiated_by_user_id     TEXT NOT NULL,
    confirmed_by_user_id     TEXT,
    confirmed_at             TEXT,
    cancelled_by_user_id     TEXT,
    cancelled_at             TEXT,
    confirmation_reference  TEXT,
    created_at               TEXT NOT NULL,
    UNIQUE (business_id, id),
    FOREIGN KEY (business_id, from_account_id)
        REFERENCES cash_accounts (business_id, id),
    FOREIGN KEY (business_id, to_account_id)
        REFERENCES cash_accounts (business_id, id),
    FOREIGN KEY (business_id, initiated_by_user_id)
        REFERENCES users (business_id, id),
    FOREIGN KEY (business_id, confirmed_by_user_id)
        REFERENCES users (business_id, id),
    FOREIGN KEY (business_id, cancelled_by_user_id)
        REFERENCES users (business_id, id),
    CHECK (from_account_id != to_account_id),
    CHECK (
        (status = 'pending'
            AND confirmed_by_user_id IS NULL AND confirmed_at IS NULL
            AND cancelled_by_user_id IS NULL AND cancelled_at IS NULL)
        OR (status = 'confirmed'
            AND confirmed_by_user_id IS NOT NULL AND confirmed_at IS NOT NULL
            AND cancelled_by_user_id IS NULL AND cancelled_at IS NULL)
        OR (status = 'cancelled'
            AND confirmed_by_user_id IS NULL AND confirmed_at IS NULL
            AND cancelled_by_user_id IS NOT NULL AND cancelled_at IS NOT NULL)
    )
);

CREATE INDEX idx_cash_transfers_business_status
    ON cash_transfers (business_id, status, occurred_at DESC);
CREATE INDEX idx_cash_transfers_accounts
    ON cash_transfers (business_id, from_account_id, to_account_id, occurred_at DESC);

CREATE TABLE cash_movements (
    id                  TEXT PRIMARY KEY,
    business_id         TEXT NOT NULL REFERENCES businesses (id),
    account_id          TEXT NOT NULL,
    movement_type       TEXT NOT NULL CHECK (movement_type IN (
        'sale_collection',
        'customer_repayment',
        'loan_issued',
        'loan_repayment',
        'expense_payment',
        'supplier_payment',
        'other_receipt',
        'other_payment',
        'transfer_in',
        'transfer_out'
    )),
    direction           TEXT NOT NULL CHECK (direction IN ('in', 'out')),
    amount              INTEGER NOT NULL CHECK (amount > 0),
    source_type         TEXT NOT NULL CHECK (source_type IN (
        'sale_payment',
        'customer_debt_payment',
        'loan',
        'expense_payment',
        'purchase_payment',
        'cash_transfer',
        'other'
    )),
    source_id           TEXT NOT NULL,
    source_leg          TEXT NOT NULL DEFAULT 'single' CHECK (source_leg IN (
        'single', 'in', 'out'
    )),
    transfer_id         TEXT,
    responsible_user_id TEXT NOT NULL,
    occurred_at         TEXT NOT NULL,
    recorded_at         TEXT NOT NULL,
    description         TEXT,
    UNIQUE (business_id, source_type, source_id, source_leg),
    UNIQUE (business_id, id),
    FOREIGN KEY (business_id, account_id)
        REFERENCES cash_accounts (business_id, id),
    FOREIGN KEY (business_id, responsible_user_id)
        REFERENCES users (business_id, id),
    FOREIGN KEY (business_id, transfer_id)
        REFERENCES cash_transfers (business_id, id),
    CHECK (
        (movement_type IN (
            'sale_collection', 'customer_repayment', 'loan_repayment',
            'other_receipt', 'transfer_in'
        ) AND direction = 'in')
        OR (movement_type IN (
            'loan_issued', 'expense_payment', 'supplier_payment',
            'other_payment', 'transfer_out'
        ) AND direction = 'out')
    ),
    CHECK (
        (movement_type = 'transfer_in' AND transfer_id IS NOT NULL
            AND source_type = 'cash_transfer' AND source_leg = 'in')
        OR (movement_type = 'transfer_out' AND transfer_id IS NOT NULL
            AND source_type = 'cash_transfer' AND source_leg = 'out')
        OR (movement_type NOT IN ('transfer_in', 'transfer_out')
            AND transfer_id IS NULL AND source_leg = 'single')
    ),
    CHECK (
        (movement_type = 'sale_collection' AND source_type = 'sale_payment')
        OR (movement_type = 'customer_repayment' AND source_type = 'customer_debt_payment')
        OR (movement_type IN ('loan_issued', 'loan_repayment') AND source_type = 'loan')
        OR (movement_type = 'expense_payment' AND source_type = 'expense_payment')
        OR (movement_type = 'supplier_payment' AND source_type = 'purchase_payment')
        OR (movement_type IN ('other_receipt', 'other_payment') AND source_type = 'other')
        OR (movement_type IN ('transfer_in', 'transfer_out') AND source_type = 'cash_transfer')
    )
);

CREATE INDEX idx_cash_movements_account_time
    ON cash_movements (business_id, account_id, occurred_at, id);
CREATE INDEX idx_cash_movements_source
    ON cash_movements (business_id, source_type, source_id);
CREATE INDEX idx_cash_movements_transfer
    ON cash_movements (business_id, transfer_id);

CREATE TRIGGER validate_cash_transfer_movement
BEFORE INSERT ON cash_movements
WHEN NEW.transfer_id IS NOT NULL AND NOT EXISTS (
    SELECT 1
    FROM cash_transfers t
    WHERE t.id = NEW.transfer_id
      AND t.business_id = NEW.business_id
      AND t.status = 'confirmed'
      AND t.id = NEW.source_id
      AND t.amount = NEW.amount
      AND (
          (NEW.source_leg = 'out'
              AND NEW.movement_type = 'transfer_out'
              AND NEW.account_id = t.from_account_id
              AND NEW.responsible_user_id = t.initiated_by_user_id)
          OR (NEW.source_leg = 'in'
              AND NEW.movement_type = 'transfer_in'
              AND NEW.account_id = t.to_account_id
              AND NEW.responsible_user_id = t.confirmed_by_user_id)
      )
)
BEGIN
    SELECT RAISE(ABORT, 'cash transfer movement must match a confirmed transfer');
END;

CREATE TRIGGER post_confirmed_cash_transfer
AFTER UPDATE OF status ON cash_transfers
WHEN OLD.status = 'pending' AND NEW.status = 'confirmed'
BEGIN
    INSERT INTO cash_movements (
        id, business_id, account_id, movement_type, direction, amount,
        source_type, source_id, source_leg, transfer_id, responsible_user_id,
        occurred_at, recorded_at, description
    ) VALUES (
        lower(hex(randomblob(16))), NEW.business_id, NEW.from_account_id,
        'transfer_out', 'out', NEW.amount, 'cash_transfer', NEW.id, 'out',
        NEW.id, NEW.initiated_by_user_id, NEW.occurred_at, NEW.confirmed_at,
        'Cash transfer out'
    );
    INSERT INTO cash_movements (
        id, business_id, account_id, movement_type, direction, amount,
        source_type, source_id, source_leg, transfer_id, responsible_user_id,
        occurred_at, recorded_at, description
    ) VALUES (
        lower(hex(randomblob(16))), NEW.business_id, NEW.to_account_id,
        'transfer_in', 'in', NEW.amount, 'cash_transfer', NEW.id, 'in',
        NEW.id, NEW.confirmed_by_user_id, NEW.occurred_at, NEW.confirmed_at,
        'Cash transfer in'
    );
END;

CREATE TRIGGER prevent_cash_movement_update
BEFORE UPDATE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'cash movements are immutable; record a correction instead');
END;

CREATE TRIGGER prevent_cash_movement_delete
BEFORE DELETE ON cash_movements
BEGIN
    SELECT RAISE(ABORT, 'cash movements are immutable; record a correction instead');
END;

CREATE TRIGGER prevent_final_cash_transfer_update
BEFORE UPDATE ON cash_transfers
WHEN OLD.status IN ('confirmed', 'cancelled')
BEGIN
    SELECT RAISE(ABORT, 'confirmed or cancelled cash transfers are immutable');
END;

CREATE TRIGGER prevent_cash_transfer_delete
BEFORE DELETE ON cash_transfers
BEGIN
    SELECT RAISE(ABORT, 'cash transfers are permanent; cancel a pending transfer instead');
END;
