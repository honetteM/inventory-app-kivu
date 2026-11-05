ALTER TABLE categories ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE brands ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE units ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE products ADD COLUMN branch_id TEXT REFERENCES branches (id);

ALTER TABLE suppliers ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE customers ADD COLUMN branch_id TEXT REFERENCES branches (id);

ALTER TABLE inventory_balances ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE stock_movements ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE inventory_cost_layers ADD COLUMN branch_id TEXT REFERENCES branches (id);

ALTER TABLE sales ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE purchases ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE expenses ADD COLUMN branch_id TEXT REFERENCES branches (id);

ALTER TABLE customer_debts ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE customer_debt_payments ADD COLUMN branch_id TEXT REFERENCES branches (id);
ALTER TABLE purchase_payments ADD COLUMN branch_id TEXT REFERENCES branches (id);

INSERT INTO branches (id, business_id, name, address, phone, is_active, created_at, updated_at)
SELECT lower(hex(randomblob(16))), b.id, 'Main branch', NULL, NULL, 1,
       COALESCE(b.created_at, '1970-01-01T00:00:00Z'),
       COALESCE(b.updated_at, '1970-01-01T00:00:00Z')
FROM businesses b
WHERE NOT EXISTS (
    SELECT 1 FROM branches br WHERE br.business_id = b.id
);

UPDATE users
SET branch_id = (
    SELECT br.id FROM branches br
    WHERE br.business_id = users.business_id AND br.is_active = 1
    ORDER BY br.created_at, br.id LIMIT 1
)
WHERE branch_id IS NULL;

UPDATE categories SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = categories.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE brands SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = brands.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE units SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = units.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE products SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = products.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE suppliers SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = suppliers.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE customers SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = customers.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE inventory_balances SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = inventory_balances.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE stock_movements SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = stock_movements.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE inventory_cost_layers SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = inventory_cost_layers.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE sales SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = sales.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE purchases SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = purchases.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE expenses SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = expenses.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE customer_debts SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = customer_debts.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE customer_debt_payments SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = customer_debt_payments.business_id LIMIT 1
) WHERE branch_id IS NULL;
UPDATE purchase_payments SET branch_id = (
    SELECT br.id FROM branches br WHERE br.business_id = purchase_payments.business_id LIMIT 1
) WHERE branch_id IS NULL;

CREATE TABLE branch_manager_assignments (
    business_id TEXT NOT NULL REFERENCES businesses (id),
    branch_id   TEXT NOT NULL,
    manager_id  TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    assigned_by TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    PRIMARY KEY (business_id, branch_id, manager_id),
    FOREIGN KEY (business_id, branch_id) REFERENCES branches (business_id, id),
    FOREIGN KEY (business_id, manager_id) REFERENCES users (business_id, id),
    FOREIGN KEY (business_id, assigned_by) REFERENCES users (business_id, id)
);

CREATE UNIQUE INDEX idx_one_active_manager_per_branch
    ON branch_manager_assignments (business_id, branch_id)
    WHERE is_active = 1;
