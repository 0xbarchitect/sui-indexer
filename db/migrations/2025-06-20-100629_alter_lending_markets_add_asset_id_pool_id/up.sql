-- Your SQL goes here

ALTER TABLE lending_markets
    ADD COLUMN IF NOT EXISTS asset_id INTEGER,
    ADD COLUMN IF NOT EXISTS pool_id VARCHAR(66);
