CREATE TABLE inventory_cost_layers (
    id                 TEXT PRIMARY KEY,
    business_id        TEXT NOT NULL,
    product_id         TEXT NOT NULL,
    source_movement_id TEXT,
    source_type        TEXT NOT NULL,
    quantity_received  INTEGER NOT NULL CHECK (quantity_received > 0),
    quantity_remaining INTEGER NOT NULL
                           CHECK (quantity_remaining >= 0 AND quantity_remaining <= quantity_received),
    unit_cost          INTEGER NOT NULL CHECK (unit_cost >= 0),
    created_at         TEXT NOT NULL,
    FOREIGN KEY (business_id, product_id) REFERENCES products (business_id, id),
    UNIQUE (business_id, id)
);

CREATE UNIQUE INDEX idx_inventory_cost_layers_source
    ON inventory_cost_layers (business_id, source_movement_id)
    WHERE source_movement_id IS NOT NULL;
CREATE INDEX idx_inventory_cost_layers_fifo
    ON inventory_cost_layers (business_id, product_id, created_at)
    WHERE quantity_remaining > 0;
