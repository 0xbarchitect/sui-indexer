-- This file should undo anything in `up.sql`

ALTER TABLE coins
    DROP COLUMN IF EXISTS pyth_decimals,
    DROP COLUMN IF EXISTS pyth_ema_price;
