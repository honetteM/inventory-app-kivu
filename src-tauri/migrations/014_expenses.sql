CREATE TABLE expenses (
    id              TEXT PRIMARY KEY,
    business_id     TEXT NOT NULL REFERENCES businesses (id),
    category        TEXT NOT NULL CHECK (category IN (
        'rent', 'electricity', 'transport', 'salaries', 'internet',
        'repairs', 'security', 'other'
    )),
    amount          INTEGER NOT NULL CHECK (amount > 0),
    expense_date    TEXT NOT NULL,
    payment_method  TEXT NOT NULL CHECK (payment_method IN (
        'cash', 'mtn_mobile_money', 'airtel_money', 'bank_transfer'
    )),
    description     TEXT NOT NULL,
    responsible_name TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    UNIQUE (business_id, id)
);

CREATE INDEX idx_expenses_business_date
    ON expenses (business_id, expense_date DESC, created_at DESC);
