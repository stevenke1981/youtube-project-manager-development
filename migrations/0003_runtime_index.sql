CREATE INDEX IF NOT EXISTS idx_projects_library_relative_path
  ON projects(library_id, relative_path);

CREATE INDEX IF NOT EXISTS idx_projects_library_status_updated
  ON projects(library_id, status, updated_at DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
