-- This file should undo anything in `up.sql`

-- First drop the trigger
DROP TRIGGER IF EXISTS update_lending_markets_modtime ON lending_markets;

-- Drop the table
DROP TABLE IF EXISTS lending_markets;