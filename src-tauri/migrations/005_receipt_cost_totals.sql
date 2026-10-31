ALTER TABLE stock_movements
    ADD COLUMN total_cost INTEGER
    CHECK (total_cost IS NULL OR total_cost >= 0);

UPDATE stock_movements
SET total_cost = (quantity_delta * unit_cost + 500) / 1000
WHERE movement_type IN ('opening_stock', 'purchase_received')
  AND quantity_delta >= 0
  AND unit_cost IS NOT NULL;
