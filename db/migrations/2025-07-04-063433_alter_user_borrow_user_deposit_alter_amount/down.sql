-- This file should undo anything in `up.sql`

ALTER TABLE user_borrows
    ALTER COLUMN amount TYPE VARCHAR(32);

ALTER TABLE user_deposits
    ALTER COLUMN amount TYPE VARCHAR(32);