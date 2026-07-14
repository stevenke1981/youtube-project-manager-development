PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS libraries (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  display_name TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  title TEXT NOT NULL,
  channel TEXT,
  series TEXT,
  status TEXT NOT NULL,
  aspect_ratio TEXT NOT NULL,
  language TEXT NOT NULL,
  progress INTEGER NOT NULL DEFAULT 0 CHECK(progress BETWEEN 0 AND 100),
  planned_publish_at TEXT,
  published_at TEXT,
  schema_version INTEGER NOT NULL,
  file_mtime_ms INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(library_id, relative_path)
);

CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);
CREATE INDEX IF NOT EXISTS idx_projects_publish ON projects(planned_publish_at);
CREATE INDEX IF NOT EXISTS idx_projects_updated ON projects(updated_at DESC);

CREATE TABLE IF NOT EXISTS assets (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  display_name TEXT,
  state TEXT NOT NULL,
  source_type TEXT,
  size_bytes INTEGER,
  sha256 TEXT,
  duration_ms INTEGER,
  width INTEGER,
  height INTEGER,
  generator TEXT,
  model TEXT,
  prompt TEXT,
  version_group_id TEXT,
  version_number INTEGER NOT NULL DEFAULT 1,
  is_adopted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(project_id, relative_path)
);

CREATE INDEX IF NOT EXISTS idx_assets_project_kind ON assets(project_id, kind);
CREATE INDEX IF NOT EXISTS idx_assets_hash ON assets(sha256);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL,
  priority TEXT NOT NULL,
  order_key REAL NOT NULL DEFAULT 0,
  due_at TEXT,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_project_status ON tasks(project_id, status, order_key);

CREATE TABLE IF NOT EXISTS project_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT,
  operation_id TEXT,
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  entity_type TEXT,
  entity_id TEXT,
  payload_json TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_project_created ON project_events(project_id, created_at DESC);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
