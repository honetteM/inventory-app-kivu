-- Inventory quantities are stored in thousandths of the product's base unit.
CREATE UNIQUE INDEX idx_products_business_product
    ON products (business_id, id);

CREATE TABLE inventory_balances (
    product_id       TEXT PRIMARY KEY,
    business_id      TEXT NOT NULL REFERENCES businesses (id),
    quantity_on_hand INTEGER NOT NULL DEFAULT 0 CHECK (quantity_on_hand >= 0),
    updated_at       TEXT NOT NULL,
    FOREIGN KEY (business_id, product_id) REFERENCES products (business_id, id)
);

CREATE INDEX idx_inventory_balances_business
    ON inventory_balances (business_id, quantity_on_hand);

CREATE TABLE stock_movements (
    id               TEXT PRIMARY KEY,
    business_id      TEXT NOT NULL REFERENCES businesses (id),
    product_id       TEXT NOT NULL,
    movement_type    TEXT NOT NULL CHECK (movement_type IN (
        'opening_stock',
        'purchase_received',
        'sale',
        'customer_return',
        'supplier_return',
        'damaged_stock',
        'stock_correction',
        'stock_transfer',
        'manual_adjustment'
    )),
    quantity_delta   INTEGER NOT NULL,
    previous_quantity INTEGER NOT NULL CHECK (previous_quantity >= 0),
    new_quantity     INTEGER NOT NULL CHECK (new_quantity >= 0),
    reference_type   TEXT,
    reference_id     TEXT,
    reason           TEXT,
    user_id          TEXT REFERENCES users (id),
    created_at       TEXT NOT NULL,
    CHECK (new_quantity = previous_quantity + quantity_delta),
    FOREIGN KEY (business_id, product_id) REFERENCES products (business_id, id)
);

CREATE UNIQUE INDEX idx_stock_movements_opening_once
    ON stock_movements (product_id)
    WHERE movement_type = 'opening_stock';
CREATE INDEX idx_stock_movements_business_created
    ON stock_movements (business_id, created_at DESC);
CREATE INDEX idx_stock_movements_product_created
    ON stock_movements (product_id, created_at DESC);
