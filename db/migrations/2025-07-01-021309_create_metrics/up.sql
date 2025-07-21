-- Your SQL goes here

CREATE TABLE IF NOT EXISTS metrics (
    id SERIAL PRIMARY KEY,
    latest_seq_number INTEGER NOT NULL,
    total_checkpoints INTEGER NOT NULL,
    total_processed_checkpoints INTEGER NOT NULL,
    max_processing_time REAL NOT NULL,
    min_processing_time REAL NOT NULL,
    avg_processing_time REAL NOT NULL,
    max_lagging REAL NOT NULL,
    min_lagging REAL NOT NULL,
    avg_lagging REAL NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_metrics_latest_seq_number ON metrics (latest_seq_number);

-- Create the trigger
CREATE TRIGGER update_metrics_modtime
    BEFORE UPDATE ON metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();
