-- This file should undo anything in `up.sql`

-- Drop the triggers

DROP TRIGGER IF EXISTS update_user_deposits_modtime ON user_deposits;
DROP TRIGGER IF EXISTS update_user_borrows_modtime ON user_borrows;

-- Drop the tables

DROP TABLE IF EXISTS user_deposits;
DROP TABLE IF EXISTS user_borrows;