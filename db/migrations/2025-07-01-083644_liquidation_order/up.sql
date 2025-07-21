-- Your SQL goes here

CREATE TABLE IF NOT EXISTS liquidation_orders (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(64) NOT NULL,
    borrower VARCHAR(66) NOT NULL,
    hf REAL NOT NULL,
    debt_coin VARCHAR(256) NOT NULL,
    collateral_coin VARCHAR(256) NOT NULL,
    amount_repay REAL NOT NULL,
    source VARCHAR(64) NOT NULL,
    tx_digest VARCHAR(128),
    checkpoint BIGINT,
    bot_address VARCHAR(66),
    finalized_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_liquidation_orders_platform ON liquidation_orders(platform);
CREATE INDEX idx_liquidation_orders_borrower ON liquidation_orders(borrower);
CREATE INDEX idx_liquidation_orders_debt_coin ON liquidation_orders(debt_coin);
CREATE INDEX idx_liquidation_orders_collateral_coin ON liquidation_orders(collateral_coin);

-- Create the trigger
CREATE TRIGGER update_liquidation_orders_modtime
    BEFORE UPDATE ON liquidation_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();