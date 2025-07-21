-- Your SQL goes here

CREATE TABLE IF NOT EXISTS liquidation_events (
    id SERIAL PRIMARY KEY,
    tx_digest VARCHAR(128) NOT NULL,
    platform VARCHAR(64) NOT NULL,
    borrower VARCHAR(66),
    liquidator VARCHAR(66),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE liquidation_events ADD CONSTRAINT unique_tx_digest UNIQUE (tx_digest);

-- Create the trigger for updated_at column
CREATE TRIGGER update_liquidation_events_modtime
    BEFORE UPDATE ON liquidation_events
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();