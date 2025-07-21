-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS update_liquidation_events_modtime ON liquidation_events;

DROP TABLE IF EXISTS liquidation_events;