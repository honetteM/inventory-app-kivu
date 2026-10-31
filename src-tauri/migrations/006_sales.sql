CREATE TABLE sales (
    id                 TEXT PRIMARY KEY,
    business_id        TEXT NOT NULL REFERENCES businesses (id),
    transaction_number TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'completed'
                           CHECK (status IN ('completed', 'cancelled')),
    subtotal           INTEGER NOT NULL CHECK (subtotal >= 0),
    discount           INTEGER NOT NULL DEFAULT 0 CHECK (discount >= 0),
    total              INTEGER NOT NULL CHECK (total >= 0),
    sold_by            TEXT REFERENCES users (id),
    created_at         TEXT NOT NULL,
    UNIQUE (business_id, transaction_number),
    UNIQUE (business_id, id)
);

CREATE INDEX idx_sales_business_created
    ON sales (business_id, created_at DESC);

CREATE TABLE sale_items (
    id             TEXT PRIMARY KEY,
    sale_id        TEXT NOT NULL,
    business_id    TEXT NOT NULL,
    product_id     TEXT NOT NULL,
    product_name   TEXT NOT NULL,
    sku            TEXT NOT NULL,
    quantity       INTEGER NOT NULL CHECK (quantity > 0),
    unit_name      TEXT NOT NULL,
    unit_price     INTEGER NOT NULL CHECK (unit_price >= 0),
    line_total     INTEGER NOT NULL CHECK (line_total >= 0),
    unit_cost      INTEGER NOT NULL CHECK (unit_cost >= 0),
    total_cost     INTEGER NOT NULL CHECK (total_cost >= 0),
    FOREIGN KEY (business_id, sale_id) REFERENCES sales (business_id, id),
    FOREIGN KEY (business_id, product_id) REFERENCES products (business_id, id)
);

CREATE INDEX idx_sale_items_sale ON sale_items (sale_id);
CREATE INDEX idx_sale_items_product ON sale_items (product_id);

CREATE TABLE sale_payments (
    id             TEXT PRIMARY KEY,
    sale_id        TEXT NOT NULL REFERENCES sales (id),
    payment_method TEXT NOT NULL CHECK (payment_method IN (
        'cash', 'mtn_mobile_money', 'airtel_money', 'bank_transfer'
    )),
    amount         INTEGER NOT NULL CHECK (amount >= 0),
    created_at     TEXT NOT NULL
);

CREATE INDEX idx_sale_payments_sale ON sale_payments (sale_id);
