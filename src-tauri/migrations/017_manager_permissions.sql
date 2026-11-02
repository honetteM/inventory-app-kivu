INSERT INTO permissions (id, description) VALUES
    ('roles.manage', 'Configure Manager permissions'),
    ('customer_debts.read', 'View outstanding customer debts for collection'),
    ('customer_payments.manage', 'Record customer debt repayments');

INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'owner';

DELETE FROM role_permissions
WHERE role_id IN (SELECT id FROM roles WHERE name = 'manager');

DELETE FROM role_permissions
WHERE role_id IN (SELECT id FROM roles WHERE name = 'cashier');

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
JOIN permissions p ON p.id IN (
    'system.read',
    'business.read',
    'catalog.read',
    'inventory.read',
    'inventory.manage',
    'suppliers.read',
    'suppliers.manage',
    'customers.read',
    'customers.manage',
    'customer_debts.read',
    'customer_payments.manage',
    'pos.use',
    'sales.manage',
    'purchases.read',
    'purchases.manage'
)
WHERE r.name = 'manager';
