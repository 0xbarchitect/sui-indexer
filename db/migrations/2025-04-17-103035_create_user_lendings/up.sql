-- Your SQL goes here

CREATE TABLE IF NOT EXISTS user_deposits (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(64) NOT NULL,
    wallet_address VARCHAR(66) NOT NULL,
    coin_type VARCHAR(256) NOT NULL,
    amount VARCHAR(32) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_deposits_wallet_address ON user_deposits(wallet_address);
CREATE INDEX idx_user_deposits_coin_type ON user_deposits(coin_type);
CREATE INDEX idx_user_deposits_platform ON user_deposits(platform);

-- Create the trigger
CREATE TRIGGER update_user_deposits_modtime
    BEFORE UPDATE ON user_deposits
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();


CREATE TABLE IF NOT EXISTS user_borrows (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(64) NOT NULL,
    wallet_address VARCHAR(66) NOT NULL,
    coin_type VARCHAR(256) NOT NULL,
    amount VARCHAR(32) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_borrows_wallet_address ON user_deposits(wallet_address);
CREATE INDEX idx_user_borrows_coin_type ON user_deposits(coin_type);
CREATE INDEX idx_user_borrows_platform ON user_deposits(platform);

-- Create the trigger
CREATE TRIGGER update_user_borrows_modtime
    BEFORE UPDATE ON user_borrows
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();
