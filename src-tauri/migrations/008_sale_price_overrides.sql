ALTER TABLE sale_items
    ADD COLUMN listed_unit_price INTEGER NOT NULL DEFAULT 0
    CHECK (listed_unit_price >= 0);

UPDATE sale_items
SET listed_unit_price = unit_price;

ALTER TABLE sale_items
    ADD COLUMN price_change_reason TEXT
    CHECK (
        price_change_reason IS NULL
        OR length(trim(price_change_reason)) BETWEEN 1 AND 500
    );
