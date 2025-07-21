-- Your SQL goes here

ALTER TABLE lending_markets
    ADD COLUMN ctoken_supply VARCHAR(64),
    ADD COLUMN available_amount VARCHAR(64),
    ADD COLUMN borrowed_amount VARCHAR(64),
    ADD COLUMN unclaimed_spread_fees VARCHAR(64),
    ADD COLUMN pyth_feed_id VARCHAR(256);
