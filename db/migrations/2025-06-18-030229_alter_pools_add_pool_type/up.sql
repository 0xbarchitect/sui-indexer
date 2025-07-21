-- Your SQL goes here

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS pool_type VARCHAR(256);
