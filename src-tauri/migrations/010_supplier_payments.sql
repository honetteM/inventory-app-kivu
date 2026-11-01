CREATE TABLE purchase_payments (
    id             TEXT PRIMARY KEY,
    business_id    TEXT NOT NULL,
    purchase_id    TEXT NOT NULL,
    payment_method TEXT NOT NULL CHECK (payment_method IN (
        'cash', 'mtn_mobile_money', 'airtel_money', 'bank_transfer'
    )),
    amount         INTEGER NOT NULL CHECK (amount > 0),
    created_at     TEXT NOT NULL,
    FOREIGN KEY (business_id, purchase_id) REFERENCES purchases (business_id, id),
    UNIQUE (business_id, id)
);

CREATE INDEX idx_purchase_payments_purchase
    ON purchase_payments (business_id, purchase_id, created_at DESC);
