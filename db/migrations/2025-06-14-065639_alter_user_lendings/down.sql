-- This file should undo anything in `up.sql`

DROP INDEX IF EXISTS idx_user_borrows_wallet_address;
DROP INDEX IF EXISTS idx_user_borrows_coin_type;
DROP INDEX IF EXISTS idx_user_borrows_platform;