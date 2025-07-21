-- This file should undo anything in `up.sql`

ALTER TABLE lending_markets
    DROP COLUMN IF EXISTS borrow_index,
    DROP COLUMN IF EXISTS supply_index;
