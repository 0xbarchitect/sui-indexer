-- Your SQL goes here

ALTER TABLE coins 
    ADD COLUMN IF NOT EXISTS pyth_ema_price VARCHAR(32),
    ADD COLUMN IF NOT EXISTS pyth_decimals INTEGER;