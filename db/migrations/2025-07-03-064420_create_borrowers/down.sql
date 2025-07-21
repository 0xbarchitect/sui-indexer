-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS update_borrowers_modtime ON borrowers;

DROP TABLE IF EXISTS borrowers;