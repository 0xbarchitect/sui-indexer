-- Your SQL goes here

CREATE TABLE IF NOT EXISTS shared_objects (
    id SERIAL PRIMARY KEY,
    object_id VARCHAR(66) NOT NULL,
    initial_shared_version BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE shared_objects ADD CONSTRAINT unq_shared_objects_object_id UNIQUE (object_id);

-- Create the trigger
CREATE TRIGGER update_shared_objects_modtime
    BEFORE UPDATE ON shared_objects
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();
