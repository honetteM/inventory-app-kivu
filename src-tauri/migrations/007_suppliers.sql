CREATE TABLE suppliers (
    id             TEXT PRIMARY KEY,
    business_id    TEXT NOT NULL REFERENCES businesses (id),
    name           TEXT NOT NULL,
    contact_person TEXT,
    phone          TEXT,
    email          TEXT,
    address        TEXT,
    tax_id         TEXT,
    is_active      INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    UNIQUE (business_id, id)
);

CREATE INDEX idx_suppliers_business_name
    ON suppliers (business_id, name COLLATE NOCASE);
