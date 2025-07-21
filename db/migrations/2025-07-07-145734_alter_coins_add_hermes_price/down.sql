-- This file should undo anything in `up.sql`

ALTER TABLE coins
    DROP COLUMN IF EXISTS hermes_price,
    DROP COLUMN IF EXISTS hermes_latest_updated_at,
    DROP COLUMN IF EXISTS vaa;
