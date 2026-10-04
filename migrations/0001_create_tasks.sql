CREATE TABLE tasks (
    id         BIGSERIAL PRIMARY KEY,
    title      TEXT        NOT NULL CHECK (btrim(title) <> ''),
    notes      TEXT        NOT NULL DEFAULT '',
    priority   SMALLINT    NOT NULL DEFAULT 2 CHECK (priority BETWEEN 1 AND 3),
    completed  BOOLEAN     NOT NULL DEFAULT FALSE,
    rollover   BOOLEAN     NOT NULL DEFAULT FALSE,
    due_date   DATE        NOT NULL,
    slice_id   TEXT,
    start_time INTEGER     CHECK (start_time BETWEEN 0 AND 2879),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX tasks_due_date_idx ON tasks (due_date);