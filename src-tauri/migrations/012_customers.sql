CREATE TABLE customers (
    id             TEXT PRIMARY KEY,
    business_id    TEXT NOT NULL REFERENCES businesses (id),
    name           TEXT NOT NULL,
    phone          TEXT,
    address        TEXT,
    customer_type  TEXT NOT NULL DEFAULT 'individual'
                       CHECK (customer_type IN ('individual', 'business')),
    credit_limit   INTEGER NOT NULL DEFAULT 0 CHECK (credit_limit >= 0),
    is_active      INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    UNIQUE (business_id, id)
);

CREATE INDEX idx_customers_business_name
    ON customers (business_id, name COLLATE NOCASE);

CREATE INDEX idx_customers_business_phone
    ON customers (business_id, phone);
