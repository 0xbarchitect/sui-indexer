-- This file should undo anything in `up.sql`

ALTER TABLE liquidation_orders
    DROP COLUMN IF EXISTS status,
    DROP COLUMN IF EXISTS amount_usd,
    ALTER COLUMN amount_repay TYPE VARCHAR(64);