-- This file should undo anything in `up.sql`

ALTER TABLE pools
    DROP COLUMN IF EXISTS current_tick_index;

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS current_tick_index INTEGER;
