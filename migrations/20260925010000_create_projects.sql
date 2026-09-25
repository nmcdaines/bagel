CREATE TABLE projects (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT     NOT NULL,
    description TEXT     NOT NULL DEFAULT '',
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX projects_updated_at_idx ON projects (updated_at DESC, id DESC);

-- A note belongs to at most one project. Deleting a project keeps its notes, unassigned.
ALTER TABLE notes ADD COLUMN project_id INTEGER REFERENCES projects (id) ON DELETE SET NULL;

CREATE INDEX notes_project_id_idx ON notes (project_id);
