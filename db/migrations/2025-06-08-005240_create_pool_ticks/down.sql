-- This file should undo anything in `up.sql`

-- First drop the trigger
DROP TRIGGER IF EXISTS update_pool_ticks_modtime ON pool_ticks;

-- Drop the table
DROP TABLE IF EXISTS pool_ticks;