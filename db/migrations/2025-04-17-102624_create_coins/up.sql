-- Your SQL goes here

CREATE TABLE IF NOT EXISTS coins (
    id SERIAL PRIMARY KEY,
    coin_type VARCHAR(256) NOT NULL,    
    decimals INTEGER NOT NULL,
    name VARCHAR(256),
    symbol VARCHAR(64),
    price_pyth VARCHAR(32),
    price_supra VARCHAR(32),
    price_switchboard VARCHAR(32),    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE coins ADD CONSTRAINT unique_coin_type UNIQUE (coin_type);

-- Create the trigger
CREATE TRIGGER update_coins_modtime
    BEFORE UPDATE ON coins
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();