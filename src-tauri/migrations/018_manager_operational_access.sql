INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
JOIN permissions p ON p.id IN ('catalog.manage', 'sales.read')
WHERE r.name = 'manager';
