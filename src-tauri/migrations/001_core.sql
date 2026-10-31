-- 001_core.sql
-- Business, security and system tables.
--
-- Conventions:
--   id          TEXT UUID (RFC 4122 v4) for sync-readiness
--   timestamps  TEXT ISO-8601 UTC (YYYY-MM-DDTHH:MM:SSZ)
--   money       INTEGER, scale x100 (hundredths of RWF)
--   quantity    INTEGER, scale x1000 (thousandths of base unit)
--   booleans    INTEGER 0/1 with CHECK constraints

CREATE TABLE businesses (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    tax_id       TEXT,
    currency     TEXT NOT NULL DEFAULT 'RWF',
    timezone     TEXT NOT NULL DEFAULT 'Africa/Kigali',
    date_format  TEXT NOT NULL DEFAULT 'DD/MM/YYYY',
    language     TEXT NOT NULL DEFAULT 'en',
    is_active    INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE branches (
    id          TEXT PRIMARY KEY,
    business_id TEXT NOT NULL REFERENCES businesses (id),
    name        TEXT NOT NULL,
    address     TEXT,
    phone       TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_branches_business ON branches (business_id);

CREATE TABLE roles (
    id           TEXT PRIMARY KEY,
    business_id  TEXT NOT NULL REFERENCES businesses (id),
    name         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    is_system    INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    UNIQUE (business_id, name)
);

CREATE TABLE permissions (
    id          TEXT PRIMARY KEY,
    description TEXT NOT NULL
);

CREATE TABLE role_permissions (
    role_id       TEXT NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id TEXT NOT NULL REFERENCES permissions (id),
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    business_id   TEXT NOT NULL REFERENCES businesses (id),
    branch_id     TEXT REFERENCES branches (id),
    username      TEXT NOT NULL,
    full_name     TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    is_active     INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    last_login_at TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (business_id, username)
);

CREATE INDEX idx_users_business ON users (business_id);

CREATE TABLE user_roles (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id    TEXT NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE (user_id, role_id)
);

CREATE TABLE audit_logs (
    id          TEXT PRIMARY KEY,
    business_id TEXT NOT NULL REFERENCES businesses (id),
    user_id     TEXT REFERENCES users (id),
    action      TEXT NOT NULL,
    entity_type TEXT,
    entity_id   TEXT,
    details     TEXT,
    created_at  TEXT NOT NULL
);

CREATE INDEX idx_audit_logs_business_created
    ON audit_logs (business_id, created_at DESC);
CREATE INDEX idx_audit_logs_user
    ON audit_logs (user_id, created_at DESC);

CREATE TABLE settings (
    business_id TEXT NOT NULL REFERENCES businesses (id),
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (business_id, key)
);
