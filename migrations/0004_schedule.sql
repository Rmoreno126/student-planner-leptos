-- Blocked windows (classes, shifts, appointments) and split times, editable in the app.
CREATE TABLE schedule_blocks (
    id         BIGSERIAL PRIMARY KEY,
    label      TEXT     NOT NULL CHECK (btrim(label) <> ''),
    weekday    SMALLINT CHECK (weekday BETWEEN 0 AND 6),
    on_date    DATE,
    start_time INTEGER  NOT NULL CHECK (start_time BETWEEN 0 AND 1439),
    end_time   INTEGER  NOT NULL CHECK (end_time BETWEEN 1 AND 1440),
    CHECK ((weekday IS NULL) <> (on_date IS NULL)),
    CHECK (end_time > start_time)
);
CREATE INDEX schedule_blocks_weekday_idx ON schedule_blocks (weekday);
CREATE INDEX schedule_blocks_date_idx ON schedule_blocks (on_date);

CREATE TABLE schedule_splits (
    id      BIGSERIAL PRIMARY KEY,
    weekday SMALLINT NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    at_time INTEGER  NOT NULL CHECK (at_time BETWEEN 0 AND 1439),
    UNIQUE (weekday, at_time)
);

-- Fall 2026 timetable (weekday 0 = Monday, times in minutes since midnight).
-- Edit or delete any of these in the app.
INSERT INTO schedule_blocks (label, weekday, start_time, end_time) VALUES
    ('MATH 107 lecture', 0, 600, 650),
    ('CS 374 activity',  0, 660, 770),
    ('CS 453 lecture',   0, 960, 1010),
    ('CS 375 lecture',   1, 660, 740),
    ('CS 374 lecture',   1, 780, 860),
    ('CS 453 activity',  1, 900, 1010),
    ('MATH 107 lecture', 2, 600, 650),
    ('CS 453 lecture',   2, 960, 1010),
    ('CS 375 lecture',   3, 660, 740),
    ('CS 374 lecture',   3, 780, 860),
    ('MATH 107 lecture', 4, 600, 650),
    ('CS 375 activity',  4, 660, 770);
