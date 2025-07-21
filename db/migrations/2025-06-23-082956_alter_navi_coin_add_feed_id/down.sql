-- This file should undo anything in `up.sql`

ALTER TABLE coins
    DROP CONSTRAINT uq_coins_navi_asset_id,
    DROP CONSTRAINT uq_coins_navi_oracle_id;

ALTER TABLE coins
    DROP COLUMN navi_asset_id,
    DROP COLUMN navi_oracle_id,
    DROP COLUMN navi_feed_id;
    