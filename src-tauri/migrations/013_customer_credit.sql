ALTER TABLE sales ADD COLUMN customer_id TEXT REFERENCES customers (id);

CREATE INDEX idx_sales_customer
    ON sales (business_id, customer_id, created_at DESC);

CREATE TABLE customer_debts (
    id              TEXT PRIMARY KEY,
    business_id     TEXT NOT NULL,
    customer_id     TEXT NOT NULL,
    sale_id         TEXT NOT NULL UNIQUE,
    original_amount INTEGER NOT NULL CHECK (original_amount > 0),
    created_at      TEXT NOT NULL,
    UNIQUE (business_id, id),
    FOREIGN KEY (business_id, customer_id)
        REFERENCES customers (business_id, id),
    FOREIGN KEY (business_id, sale_id)
        REFERENCES sales (business_id, id)
);

CREATE INDEX idx_customer_debts_business_customer
    ON customer_debts (business_id, customer_id, created_at DESC);

CREATE TABLE customer_debt_payments (
    id             TEXT PRIMARY KEY,
    business_id    TEXT NOT NULL,
    debt_id        TEXT NOT NULL,
    payment_method TEXT NOT NULL CHECK (payment_method IN (
        'cash', 'mtn_mobile_money', 'airtel_money', 'bank_transfer'
    )),
    amount         INTEGER NOT NULL CHECK (amount > 0),
    created_at     TEXT NOT NULL,
    FOREIGN KEY (business_id, debt_id)
        REFERENCES customer_debts (business_id, id)
);

CREATE INDEX idx_customer_debt_payments_debt
    ON customer_debt_payments (business_id, debt_id, created_at DESC);
