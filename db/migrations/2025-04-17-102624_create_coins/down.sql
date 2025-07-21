-- This file should undo anything in `up.sql`

-- First drop the trigger
DROP TRIGGER IF EXISTS update_coins_modtime ON coins;

-- Drop the table
DROP TABLE IF EXISTS coins;