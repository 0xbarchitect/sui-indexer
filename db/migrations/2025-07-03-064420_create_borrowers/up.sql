-- Your SQL goes here

CREATE TABLE IF NOT EXISTS borrowers (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(64) NOT NULL,
    borrower VARCHAR(66) NOT NULL,
    obligation_id VARCHAR(66),
    status INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE borrowers ADD CONSTRAINT unq_platform_borrower UNIQUE (platform, borrower);

-- Create the trigger
CREATE TRIGGER update_borrowers_modtime
    BEFORE UPDATE ON borrowers
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();