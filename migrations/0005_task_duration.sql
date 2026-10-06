ALTER TABLE tasks
    ADD COLUMN duration_minutes INTEGER NOT NULL DEFAULT 15
    CHECK (duration_minutes BETWEEN 1 AND 1440);