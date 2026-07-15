CREATE TABLE IF NOT EXISTS timeline_revisions (
  project_relative_path TEXT NOT NULL,
  revision INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (project_relative_path, revision)
);

CREATE INDEX IF NOT EXISTS idx_timeline_revisions_project
  ON timeline_revisions(project_relative_path, revision DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
