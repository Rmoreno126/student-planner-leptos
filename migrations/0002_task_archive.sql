-- Archived tasks stay in the database forever; they just leave every actionable view.
ALTER TABLE tasks ADD COLUMN archived BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX tasks_archived_idx ON tasks (archived);
