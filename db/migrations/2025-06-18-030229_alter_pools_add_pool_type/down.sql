-- This file should undo anything in `up.sql`

ALTER TABLE pools
    DROP COLUMN IF EXISTS pool_type;