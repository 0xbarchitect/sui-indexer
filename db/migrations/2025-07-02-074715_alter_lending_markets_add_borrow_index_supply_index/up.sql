-- Your SQL goes here

ALTER TABLE lending_markets
    ADD COLUMN IF NOT EXISTS borrow_index VARCHAR(64),
    ADD COLUMN IF NOT EXISTS supply_index VARCHAR(64);
