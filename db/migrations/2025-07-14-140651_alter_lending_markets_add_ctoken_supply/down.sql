-- This file should undo anything in `up.sql`

ALTER TABLE lending_markets
    DROP COLUMN IF EXISTS ctoken_supply,
    DROP COLUMN IF EXISTS available_amount,
    DROP COLUMN IF EXISTS borrowed_amount,
    DROP COLUMN IF EXISTS unclaimed_spread_fees,
    DROP COLUMN IF EXISTS pyth_feed_id;