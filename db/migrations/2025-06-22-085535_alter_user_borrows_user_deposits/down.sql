-- This file should undo anything in `up.sql`

ALTER TABLE user_borrows
    DROP CONSTRAINT uq_user_borrows_platform_wallet_coin;

ALTER TABLE user_deposits
    DROP CONSTRAINT uq_user_deposits_platform_wallet_coin;