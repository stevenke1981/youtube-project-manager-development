CREATE VIRTUAL TABLE IF NOT EXISTS project_search USING fts5(
  project_id UNINDEXED,
  title,
  channel,
  series,
  tags,
  tokenize = 'unicode61'
);

CREATE VIRTUAL TABLE IF NOT EXISTS asset_search USING fts5(
  asset_id UNINDEXED,
  project_id UNINDEXED,
  display_name,
  relative_path,
  prompt,
  tokenize = 'unicode61'
);

INSERT OR IGNORE INTO schema_migrations(version, applied_at)
VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
