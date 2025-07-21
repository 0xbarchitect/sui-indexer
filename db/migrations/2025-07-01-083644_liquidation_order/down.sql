-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS update_liquidation_orders_modtime ON liquidation_orders;

DROP TABLE IF EXISTS liquidation_orders;