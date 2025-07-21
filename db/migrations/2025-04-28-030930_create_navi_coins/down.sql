-- This file should undo anything in `up.sql`

-- Delete coins master data
DELETE FROM coins;

-- Then drop the columns from coins table
ALTER TABLE coins
    DROP COLUMN IF EXISTS pyth_feed_id,
    DROP COLUMN IF EXISTS pyth_info_object_id,
    DROP COLUMN IF EXISTS pyth_latest_updated_at;

-- Then drop the trigger
DROP TRIGGER IF EXISTS update_navi_coins_modtime ON navi_coins;

-- Finally drop the navi_coins table
DROP TABLE IF EXISTS navi_coins;