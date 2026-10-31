-- 002_products.sql
-- Product catalogue tables.
--
-- Conventions:
--   money              INTEGER, scale x100 (hundredths of RWF)
--   quantity           INTEGER, scale x1000 (thousandths of base unit)
--   conversion_factor  INTEGER, scale x1000 (1000 = 1.000 base units)

CREATE TABLE categories (
    id          TEXT PRIMARY KEY,
    business_id TEXT NOT NULL REFERENCES businesses (id),
    parent_id   TEXT REFERENCES categories (id),
    name        TEXT NOT NULL,
    description TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_categories_unique_name
    ON categories (business_id, COALESCE(parent_id, ''), name);
CREATE INDEX idx_categories_business
    ON categories (business_id, sort_order);

CREATE TABLE brands (
    id          TEXT PRIMARY KEY,
    business_id TEXT NOT NULL REFERENCES businesses (id),
    name        TEXT NOT NULL,
    is_active   INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_brands_unique_name
    ON brands (business_id, name);

-- Units of measurement. business_id IS NULL means system-wide unit.
CREATE TABLE units (
    id           TEXT PRIMARY KEY,
    business_id  TEXT REFERENCES businesses (id),
    code         TEXT NOT NULL,
    name         TEXT NOT NULL,
    decimals_ok  INTEGER NOT NULL DEFAULT 1 CHECK (decimals_ok IN (0, 1)),
    is_system    INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    is_active    INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_units_unique_code
    ON units (COALESCE(business_id, ''), code);

-- Seed system units used by hardware shops.
INSERT INTO units (id, business_id, code, name, decimals_ok, is_system, is_active, created_at, updated_at) VALUES
    ('sys-unit-bag',    NULL, 'bag',  'Bag',         0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-pc',     NULL, 'pc',   'Piece',       0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-m',      NULL, 'm',    'Metre',       1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-cm',     NULL, 'cm',   'Centimetre',  1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-kg',     NULL, 'kg',   'Kilogram',    1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-g',      NULL, 'g',    'Gram',        1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-l',      NULL, 'l',    'Litre',       1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-ml',     NULL, 'ml',   'Millilitre',  1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-m2',     NULL, 'm2',   'Square metre',1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-roll',   NULL, 'roll', 'Roll',        0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-bundle', NULL, 'bdl',  'Bundle',      0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-bkt',    NULL, 'bkt',  'Bucket',      0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-pack',   NULL, 'pck',  'Pack',        0, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('sys-unit-t',      NULL, 't',    'Ton',         1, 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');

CREATE TABLE products (
    id                 TEXT PRIMARY KEY,
    business_id        TEXT NOT NULL REFERENCES businesses (id),
    category_id        TEXT REFERENCES categories (id),
    brand_id           TEXT REFERENCES brands (id),
    name               TEXT NOT NULL,
    sku                TEXT NOT NULL,
    barcode            TEXT,
    description        TEXT,
    size_dimensions    TEXT,
    colour             TEXT,
    image_path         TEXT,
    base_unit_id       TEXT NOT NULL REFERENCES units (id),
    purchase_price     INTEGER NOT NULL DEFAULT 0 CHECK (purchase_price >= 0),
    selling_price      INTEGER NOT NULL DEFAULT 0 CHECK (selling_price >= 0),
    min_stock_quantity INTEGER NOT NULL DEFAULT 0 CHECK (min_stock_quantity >= 0),
    vat_class          TEXT NOT NULL DEFAULT 'standard'
                           CHECK (vat_class IN ('standard', 'zero', 'exempt')),
    is_active          INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_products_sku ON products (business_id, sku);
CREATE INDEX idx_products_name      ON products (business_id, name);
CREATE INDEX idx_products_barcode   ON products (business_id, barcode);
CREATE INDEX idx_products_category  ON products (category_id);
CREATE INDEX idx_products_brand     ON products (brand_id);
CREATE INDEX idx_products_active    ON products (business_id, is_active);

-- Selling units per product with conversion to the base unit.
-- The base unit row has conversion_factor = 1000 (1.000).
CREATE TABLE product_units (
    id                 TEXT PRIMARY KEY,
    product_id         TEXT NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    unit_id            TEXT NOT NULL REFERENCES units (id),
    conversion_factor  INTEGER NOT NULL CHECK (conversion_factor > 0),
    is_base            INTEGER NOT NULL DEFAULT 0 CHECK (is_base IN (0, 1)),
    selling_price      INTEGER CHECK (selling_price IS NULL OR selling_price >= 0),
    sort_order         INTEGER NOT NULL DEFAULT 0,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    UNIQUE (product_id, unit_id)
);

CREATE INDEX idx_product_units_product ON product_units (product_id);

-- Price change history (audit trail for pricing).
CREATE TABLE product_prices (
    id             TEXT PRIMARY KEY,
    product_id     TEXT NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    price_type     TEXT NOT NULL CHECK (price_type IN ('purchase', 'selling')),
    unit_id        TEXT REFERENCES units (id),
    price          INTEGER NOT NULL CHECK (price >= 0),
    effective_from TEXT NOT NULL,
    effective_to   TEXT,
    created_by     TEXT REFERENCES users (id),
    created_at     TEXT NOT NULL
);

CREATE INDEX idx_product_prices_product
    ON product_prices (product_id, created_at DESC);
