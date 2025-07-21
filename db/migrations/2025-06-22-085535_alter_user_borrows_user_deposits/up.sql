-- Your SQL goes here

ALTER TABLE user_borrows
    ADD CONSTRAINT uq_user_borrows_platform_wallet_coin UNIQUE (platform, wallet_address, coin_type);

ALTER TABLE user_deposits
    ADD CONSTRAINT uq_user_deposits_platform_wallet_coin UNIQUE (platform, wallet_address, coin_type);