-- Your SQL goes here

ALTER TABLE coins
    ADD COLUMN IF NOT EXISTS hermes_price VARCHAR(32),
    ADD COLUMN IF NOT EXISTS hermes_latest_updated_at TIMESTAMP,
    ADD COLUMN IF NOT EXISTS vaa TEXT;