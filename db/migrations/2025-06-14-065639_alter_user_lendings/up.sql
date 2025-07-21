-- Your SQL goes here

-- There is a mistake in the `create_user_lending` migration that create the indexes on the wrong table.
-- This migration fixes that mistake by dropping the incorrect indexes and creating them on the correct table.
DROP INDEX IF EXISTS idx_user_borrows_wallet_address;
DROP INDEX IF EXISTS idx_user_borrows_coin_type;
DROP INDEX IF EXISTS idx_user_borrows_platform;


CREATE INDEX IF NOT EXISTS idx_user_borrows_wallet_address ON user_borrows(wallet_address);
CREATE INDEX IF NOT EXISTS idx_user_borrows_coin_type ON user_borrows(coin_type);
CREATE INDEX IF NOT EXISTS idx_user_borrows_platform ON user_borrows(platform);

