ALTER TABLE inventory_balances
    ADD COLUMN average_unit_cost INTEGER NOT NULL DEFAULT 0
    CHECK (average_unit_cost >= 0);

UPDATE inventory_balances
SET average_unit_cost = (
    SELECT purchase_price
    FROM products
    WHERE products.id = inventory_balances.product_id
);

ALTER TABLE stock_movements
    ADD COLUMN unit_cost INTEGER
    CHECK (unit_cost IS NULL OR unit_cost >= 0);

UPDATE stock_movements
SET unit_cost = (
    SELECT purchase_price
    FROM products
    WHERE products.id = stock_movements.product_id
);
