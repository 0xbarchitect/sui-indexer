-- This file should undo anything in `up.sql`

ALTER TABLE liquidation_orders
    DROP COLUMN IF EXISTS error;