INSERT INTO permissions (id, description) VALUES
    ('expenses.read', 'View operating expense records'),
    ('expenses.manage', 'Create and manage operating expenses'),
    ('cash_accounts.balance.read', 'View account balances for payment validation');

INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'owner';
