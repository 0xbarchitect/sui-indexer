-- This file should undo anything in `up.sql`

CREATE TABLE IF NOT EXISTS navi_coins (
    id SERIAL PRIMARY KEY,
    asset_id INTEGER NOT NULL,
    coin_type VARCHAR(256) NOT NULL,
    name VARCHAR(256),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE navi_coins ADD CONSTRAINT unique_navi_coin UNIQUE (asset_id);
