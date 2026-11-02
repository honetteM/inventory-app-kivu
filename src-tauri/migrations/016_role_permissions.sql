INSERT INTO permissions (id, description) VALUES
    ('system.read', 'Read basic application information'),
    ('business.read', 'View business configuration'),
    ('business.manage', 'Create and manage business configuration'),
    ('employees.manage', 'Manage employee profiles and access'),
    ('cash_accounts.manage', 'Manage cash, bank and mobile-money accounts'),
    ('catalog.read', 'View product catalog, categories, brands and units'),
    ('catalog.manage', 'Manage products, categories, brands and units'),
    ('inventory.read', 'View stock levels and movement history'),
    ('inventory.manage', 'Adjust stock and receive inventory'),
    ('suppliers.read', 'View supplier records'),
    ('suppliers.manage', 'Manage supplier records'),
    ('customers.read', 'View customer records'),
    ('customers.manage', 'Manage customer records'),
    ('pos.use', 'Search products for checkout and record point-of-sale transactions'),
    ('sales.read', 'View sales records'),
    ('sales.manage', 'Record sales and customer payments'),
    ('purchases.read', 'View purchases and purchase costs'),
    ('purchases.manage', 'Manage purchases, deliveries and supplier payments'),
    ('finance.read', 'View expenses, debt balances and financial activity'),
    ('finance.manage', 'Manage expenses and financial payments'),
    ('reports.read', 'View business financial reports');

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'owner';
