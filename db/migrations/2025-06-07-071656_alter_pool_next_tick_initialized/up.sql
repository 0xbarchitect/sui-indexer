-- Your SQL goes here

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS next_tick_initialized_upper VARCHAR(32);

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS next_tick_initialized_lower VARCHAR(32);