-- Your SQL goes here

CREATE TABLE IF NOT EXISTS lending_markets (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(64) NOT NULL,
    coin_type VARCHAR(256) NOT NULL,
    ltv VARCHAR(64),
    liquidation_threshold VARCHAR(64),
    borrow_weight VARCHAR(64),
    liquidation_ratio VARCHAR(64),
    liquidation_penalty VARCHAR(64),
    liquidation_fee VARCHAR(64),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE lending_markets
    ADD CONSTRAINT unique_lending_market UNIQUE (platform, coin_type);

-- Create the trigger
CREATE TRIGGER update_lending_markets_modtime
    BEFORE UPDATE ON lending_markets
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();