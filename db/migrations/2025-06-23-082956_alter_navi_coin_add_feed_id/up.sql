-- Your SQL goes here

ALTER TABLE coins
    ADD COLUMN navi_asset_id INTEGER,
    ADD COLUMN navi_oracle_id INTEGER,
    ADD COLUMN navi_feed_id VARCHAR(66);

ALTER TABLE coins
    ADD CONSTRAINT uq_coins_navi_asset_id UNIQUE (navi_asset_id),
    ADD CONSTRAINT uq_coins_navi_oracle_id UNIQUE (navi_oracle_id);
    