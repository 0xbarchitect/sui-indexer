-- Your SQL goes here

ALTER TABLE user_borrows
    RENAME COLUMN wallet_address TO borrower;
    
ALTER TABLE user_borrows
    RENAME COLUMN borrow_index TO debt_borrow_index;

ALTER TABLE user_deposits
    RENAME COLUMN wallet_address TO borrower;