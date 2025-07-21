-- Your SQL goes here

ALTER TABLE user_borrows
    ALTER COLUMN amount TYPE VARCHAR(64);

ALTER TABLE user_deposits
    ALTER COLUMN amount TYPE VARCHAR(64);