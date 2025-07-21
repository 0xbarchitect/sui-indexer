-- Your SQL goes here

ALTER TABLE pools
    DROP COLUMN IF EXISTS next_tick_initialized_upper,
    DROP COLUMN IF EXISTS next_tick_initialized_lower;

ALTER TABLE pools
    ALTER COLUMN current_tick_index TYPE INTEGER USING current_tick_index::INTEGER;

ALTER TABLE pools
    ALTER COLUMN liquidity TYPE VARCHAR(64) USING liquidity::VARCHAR(64);

CREATE TABLE IF NOT EXISTS pool_ticks (
    id SERIAL PRIMARY KEY,
    address VARCHAR(66) NOT NULL,
    tick_index INTEGER NOT NULL,
    liquidity_net VARCHAR(64),
    liquidity_gross VARCHAR(64),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE pool_ticks ADD CONSTRAINT unique_pool_tick UNIQUE (address, tick_index);

-- Create the trigger for updated_at column
CREATE TRIGGER update_pool_ticks_modtime
    BEFORE UPDATE ON pool_ticks
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();

