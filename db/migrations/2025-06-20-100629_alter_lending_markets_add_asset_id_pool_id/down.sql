-- This file should undo anything in `up.sql`

ALTER TABLE lending_markets
    DROP COLUMN asset_id,
    DROP COLUMN pool_id;