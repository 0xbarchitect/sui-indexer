-- Your SQL goes here

ALTER TABLE liquidation_orders
    ADD COLUMN status INTEGER DEFAULT 0,
    ADD COLUMN amount_usd VARCHAR(64) NOT NULL,
    ALTER COLUMN amount_repay TYPE VARCHAR(64);