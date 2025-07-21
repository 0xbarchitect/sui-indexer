-- Your SQL goes here

CREATE TABLE IF NOT EXISTS pools (
    id SERIAL PRIMARY KEY,
    exchange VARCHAR(64) NOT NULL,
    address VARCHAR(66) NOT NULL,
    coin_a VARCHAR(256) NOT NULL,
    coin_b VARCHAR(256) NOT NULL,
    coin_a_amount VARCHAR(32),
    coin_b_amount VARCHAR(32),
    liquidity VARCHAR(32),
    current_sqrt_price VARCHAR(32),
    current_tick_index INTEGER,
    tick_spacing INTEGER,
    fee_rate INTEGER,
    is_pause BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE pools ADD CONSTRAINT unique_pool_address UNIQUE (address);

-- Create the function for updating timestamp
CREATE OR REPLACE FUNCTION update_modified_column() 
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create the trigger
CREATE TRIGGER update_pools_modtime
    BEFORE UPDATE ON pools
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();