CREATE TABLE purchases (
    id                       TEXT PRIMARY KEY,
    business_id              TEXT NOT NULL REFERENCES businesses (id),
    supplier_id              TEXT NOT NULL,
    supplier_invoice_number  TEXT NOT NULL,
    status                   TEXT NOT NULL DEFAULT 'pending'
                                 CHECK (status IN ('pending', 'partially_received', 'received')),
    subtotal                 INTEGER NOT NULL CHECK (subtotal >= 0),
    duplicate_authorized     INTEGER NOT NULL DEFAULT 0
                                 CHECK (duplicate_authorized IN (0, 1)),
    duplicate_of_purchase_id TEXT,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL,
    UNIQUE (business_id, id),
    FOREIGN KEY (business_id, supplier_id) REFERENCES suppliers (business_id, id),
    FOREIGN KEY (business_id, duplicate_of_purchase_id) REFERENCES purchases (business_id, id)
);

CREATE INDEX idx_purchases_business_created
    ON purchases (business_id, created_at DESC);
CREATE INDEX idx_purchases_supplier_invoice
    ON purchases (business_id, supplier_id, supplier_invoice_number COLLATE NOCASE);

CREATE TABLE purchase_items (
    id                TEXT PRIMARY KEY,
    business_id       TEXT NOT NULL,
    purchase_id       TEXT NOT NULL,
    product_id        TEXT NOT NULL,
    product_name      TEXT NOT NULL,
    sku               TEXT NOT NULL,
    unit_name         TEXT NOT NULL,
    ordered_quantity  INTEGER NOT NULL CHECK (ordered_quantity > 0),
    received_quantity INTEGER NOT NULL DEFAULT 0
                          CHECK (received_quantity >= 0 AND received_quantity <= ordered_quantity),
    unit_cost         INTEGER NOT NULL CHECK (unit_cost >= 0),
    line_total        INTEGER NOT NULL CHECK (line_total >= 0),
    FOREIGN KEY (business_id, purchase_id) REFERENCES purchases (business_id, id),
    FOREIGN KEY (business_id, product_id) REFERENCES products (business_id, id),
    UNIQUE (purchase_id, product_id),
    UNIQUE (business_id, id)
);

CREATE INDEX idx_purchase_items_purchase ON purchase_items (purchase_id);
CREATE INDEX idx_purchase_items_product ON purchase_items (product_id);

CREATE TABLE purchase_receipts (
    id              TEXT PRIMARY KEY,
    business_id     TEXT NOT NULL,
    purchase_id     TEXT NOT NULL,
    delivery_reference TEXT,
    notes           TEXT,
    created_at      TEXT NOT NULL,
    FOREIGN KEY (business_id, purchase_id) REFERENCES purchases (business_id, id),
    UNIQUE (business_id, id)
);

CREATE INDEX idx_purchase_receipts_purchase
    ON purchase_receipts (purchase_id, created_at DESC);

CREATE TABLE purchase_receipt_items (
    id                TEXT PRIMARY KEY,
    business_id       TEXT NOT NULL,
    receipt_id        TEXT NOT NULL,
    purchase_item_id  TEXT NOT NULL,
    quantity          INTEGER NOT NULL CHECK (quantity > 0),
    unit_cost         INTEGER NOT NULL CHECK (unit_cost >= 0),
    total_cost        INTEGER NOT NULL CHECK (total_cost >= 0),
    FOREIGN KEY (business_id, receipt_id) REFERENCES purchase_receipts (business_id, id),
    FOREIGN KEY (business_id, purchase_item_id) REFERENCES purchase_items (business_id, id),
    UNIQUE (receipt_id, purchase_item_id)
);

CREATE INDEX idx_purchase_receipt_items_line
    ON purchase_receipt_items (purchase_item_id);
