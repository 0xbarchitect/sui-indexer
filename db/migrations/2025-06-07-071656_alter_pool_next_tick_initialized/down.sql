-- This file should undo anything in `up.sql`

ALTER TABLE pools
  DROP COLUMN IF EXISTS next_tick_initialized_upper, 
  DROP COLUMN IF EXISTS next_tick_initialized_lower;
