-- Your SQL goes here

ALTER TABLE liquidation_orders ADD CONSTRAINT unq_liquidation_orders_platform_borrower UNIQUE (platform, borrower);
