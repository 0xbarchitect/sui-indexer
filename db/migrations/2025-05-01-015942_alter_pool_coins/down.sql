-- This file should undo anything in `up.sql`

ALTER TABLE pools
    DROP COLUMN IF EXISTS coins,
    DROP COLUMN IF EXISTS coin_amounts,
    DROP COLUMN IF EXISTS weights,
    DROP COLUMN IF EXISTS fees_swap_in,
    DROP COLUMN IF EXISTS fees_swap_out;

ALTER TABLE pools
    ADD COLUMN IF NOT EXISTS coin_a VARCHAR(256),
    ADD COLUMN IF NOT EXISTS coin_b VARCHAR(256),
    ADD COLUMN IF NOT EXISTS coin_a_amount VARCHAR(64),
    ADD COLUMN IF NOT EXISTS coin_b_amount VARCHAR(64);
