-- This file should undo anything in `up.sql`

DROP TRIGGER IF EXISTS update_shared_objects_modtime ON shared_objects;

DROP TABLE IF EXISTS shared_objects;
