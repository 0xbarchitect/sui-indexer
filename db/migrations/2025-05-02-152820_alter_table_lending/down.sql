-- This file should undo anything in `up.sql`

ALTER TABLE user_deposits
    DROP COLUMN IF EXISTS obligation_id;

ALTER TABLE user_borrows
    DROP COLUMN IF EXISTS obligation_id,
    DROP COLUMN IF EXISTS borrow_index;
