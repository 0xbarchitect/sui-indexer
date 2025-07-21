-- This file should undo anything in `up.sql`

ALTER TABLE user_borrows
    RENAME COLUMN borrower TO wallet_address;

ALTER TABLE user_deposits
    RENAME COLUMN borrower TO wallet_address;

ALTER TABLE user_borrows
    RENAME COLUMN debt_borrow_index TO borrow_index;