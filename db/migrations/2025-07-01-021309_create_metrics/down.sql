-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS update_metrics_modtime ON metrics;

-- Drop the tables

DROP INDEX IF EXISTS idx_metrics_latest_seq_number;
DROP TABLE IF EXISTS metrics;
