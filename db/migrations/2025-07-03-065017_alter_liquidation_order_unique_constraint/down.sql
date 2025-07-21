-- This file should undo anything in `up.sql`

ALTER TABLE liquidation_orders DROP CONSTRAINT IF EXISTS unq_liquidation_orders_platform_borrower;
