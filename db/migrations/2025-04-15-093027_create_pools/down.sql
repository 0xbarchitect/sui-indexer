-- This file should undo anything in `up.sql`

-- First drop the trigger
DROP TRIGGER IF EXISTS update_pools_modtime ON pools;

-- Then drop the function
DROP FUNCTION IF EXISTS update_modified_column();

-- Finally drop the table
DROP TABLE IF EXISTS pools;
