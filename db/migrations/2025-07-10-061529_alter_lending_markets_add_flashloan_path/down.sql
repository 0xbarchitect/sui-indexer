-- This file should undo anything in `up.sql`

ALTER TABLE lending_markets
    DROP COLUMN flashloan_path;
