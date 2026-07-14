use super::{Project, ProjectStatus, Result, YtpmError, CURRENT_SCHEMA_VERSION};
use chrono::{SecondsFormat, Utc};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Transaction};
use serde_json::Value as JsonValue;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

const LIBRARY_ID: &str = "library";
const INDEX_DIRECTORY: &str = ".ytpm";
const INDEX_FILE: &str = "index.sqlite3";

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../../../migrations/0002_search.sql");
const MIGRATION_0003: &str = include_str!("../../../migrations/0003_runtime_index.sql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexReport {
    pub db_path: PathBuf,
    pub scanned: usize,
    pub indexed: usize,
    pub invalid: usize,
}

struct ScannedProject {
    project: Project,
    relative_path: String,
}

pub fn rebuild_index(root: &Path) -> Result<IndexReport> {
    ensure_no_parent_components(root)?;
    fs::create_dir_all(root).map_err(|error| YtpmError::io(root, error))?;

    let db_path = index_path(root);
    let (projects, scanned, invalid) = scan_projects(root)?;
    let index_directory = db_path.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 SQLite index parent：{}", db_path.display()))
    })?;
    fs::create_dir_all(index_directory).map_err(|error| YtpmError::io(index_directory, error))?;

    let mut connection = open_connection(&db_path)?;
    apply_migrations(&mut connection, &db_path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| sqlite_error(&db_path, "開始 rebuild transaction", error))?;
    rebuild_transaction(&transaction, root, &projects, &db_path)?;
    transaction
        .commit()
        .map_err(|error| sqlite_error(&db_path, "提交 rebuild transaction", error))?;

    Ok(IndexReport {
        db_path,
        scanned,
        indexed: projects.len(),
        invalid,
    })
}

pub fn search_index(
    root: &Path,
    query: Option<&str>,
    status: Option<ProjectStatus>,
) -> Result<Vec<Project>> {
    ensure_no_parent_components(root)?;
    let db_path = index_path(root);
    // The SQLite database is a rebuildable derived cache. Rebuilding before each
    // query keeps results correct after files are edited outside the app and
    // avoids ever treating stale cache rows as the source of truth.
    rebuild_index(root)?;

    let mut connection = open_connection(&db_path)?;
    apply_migrations(&mut connection, &db_path)?;

    let mut sql = String::from(
        "SELECT p.relative_path
         FROM projects AS p
         WHERE p.library_id = ?1",
    );
    let mut values = vec![Value::Text(LIBRARY_ID.to_string())];
    let normalized_query = query.map(str::trim).filter(|value| !value.is_empty());

    if let Some(query) = normalized_query {
        let like_pattern = format!("%{}%", escape_like_pattern(query));
        let fts_query = safe_fts_query(query);
        let like_title = next_placeholder(values.len() + 1);
        let like_channel = next_placeholder(values.len() + 2);
        let like_series = next_placeholder(values.len() + 3);
        let fts = next_placeholder(values.len() + 4);
        sql.push_str(&format!(
            " AND (p.title LIKE {like_title} ESCAPE '\\'
                OR COALESCE(p.channel, '') LIKE {like_channel} ESCAPE '\\'
                OR COALESCE(p.series, '') LIKE {like_series} ESCAPE '\\'
                OR p.id IN (
                    SELECT project_id
                    FROM project_search
                    WHERE project_search MATCH {fts}
                ))"
        ));
        values.extend([
            Value::Text(like_pattern.clone()),
            Value::Text(like_pattern.clone()),
            Value::Text(like_pattern),
            Value::Text(fts_query),
        ]);
    }

    if let Some(status) = status.as_ref() {
        let status_placeholder = next_placeholder(values.len() + 1);
        sql.push_str(&format!(" AND p.status = {status_placeholder}"));
        values.push(Value::Text(status_key(status).to_string()));
    }

    sql.push_str(" ORDER BY p.updated_at DESC, p.relative_path ASC");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| sqlite_error(&db_path, "準備 search query", error))?;
    let candidates = statement
        .query_map(params_from_iter(values.iter()), |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| sqlite_error(&db_path, "執行 search query", error))?;

    let mut projects = Vec::new();
    for candidate in candidates {
        let relative_path =
            candidate.map_err(|error| sqlite_error(&db_path, "讀取 search candidate", error))?;
        let project_file = project_file_for_relative_path(root, &relative_path, &db_path)?;
        let project = read_project(&project_file).map_err(|error| {
            YtpmError::InvalidProject(format!(
                "SQLite index 指向無法讀取的 project.json {}：{error}。請保留專案資料，重新執行 rebuild_index。",
                project_file.display()
            ))
        })?;

        if status
            .as_ref()
            .is_some_and(|expected| project.status != *expected)
        {
            continue;
        }
        if normalized_query.is_some_and(|text| !project_matches_query(&project, text)) {
            continue;
        }
        projects.push(project);
    }

    Ok(projects)
}

fn rebuild_transaction(
    transaction: &Transaction<'_>,
    root: &Path,
    projects: &[ScannedProject],
    db_path: &Path,
) -> Result<()> {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let display_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("Library");

    transaction
        .execute(
            "INSERT INTO libraries(id, path, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(id) DO UPDATE SET
               path = excluded.path,
               display_name = excluded.display_name,
               updated_at = excluded.updated_at",
            params![
                LIBRARY_ID,
                root.to_string_lossy().as_ref(),
                display_name,
                now
            ],
        )
        .map_err(|error| sqlite_error(db_path, "更新 library row", error))?;

    transaction
        .execute(
            "DELETE FROM project_search
             WHERE project_id IN (SELECT id FROM projects WHERE library_id = ?1)",
            params![LIBRARY_ID],
        )
        .map_err(|error| sqlite_error(db_path, "清除舊 project search rows", error))?;
    transaction
        .execute(
            "DELETE FROM projects WHERE library_id = ?1",
            params![LIBRARY_ID],
        )
        .map_err(|error| sqlite_error(db_path, "清除舊 project rows", error))?;

    for scanned in projects {
        let project = &scanned.project;
        let created_at = project
            .created_at
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let updated_at = project
            .updated_at
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        transaction
            .execute(
                "INSERT INTO projects(
                    id, library_id, relative_path, title, channel, series, status,
                    aspect_ratio, language, progress, planned_publish_at, published_at,
                    schema_version, file_mtime_ms, created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
                )",
                params![
                    project.id,
                    LIBRARY_ID,
                    scanned.relative_path,
                    project.title,
                    project.channel,
                    project.series,
                    status_key(&project.status),
                    project.aspect_ratio,
                    project.language,
                    i64::from(project.progress),
                    project.planned_publish_at.map(|date| date.to_rfc3339()),
                    project.published_at.map(|date| date.to_rfc3339()),
                    i64::from(project.schema_version),
                    project_file_mtime_ms(root, &scanned.relative_path),
                    created_at,
                    updated_at,
                ],
            )
            .map_err(|error| sqlite_error(db_path, "寫入 project row", error))?;

        transaction
            .execute(
                "INSERT INTO project_search(project_id, title, channel, series, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    project.id,
                    project.title,
                    project.channel.as_deref().unwrap_or_default(),
                    project.series.as_deref().unwrap_or_default(),
                    project.tags.join(" "),
                ],
            )
            .map_err(|error| sqlite_error(db_path, "寫入 project search row", error))?;
    }

    Ok(())
}

fn scan_projects(root: &Path) -> Result<(Vec<ScannedProject>, usize, usize)> {
    let mut projects = Vec::new();
    let mut scanned = 0;
    let mut invalid = 0;
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(root, entry.path()));

    for entry in walker {
        let entry = entry.map_err(|error| {
            YtpmError::InvalidProject(format!("掃描 Library {} 失敗：{error}", root.display()))
        })?;
        if !entry.file_type().is_file() || entry.file_name() != OsStr::new("project.json") {
            continue;
        }

        scanned += 1;
        let project = match read_project(entry.path()) {
            Ok(project) => project,
            Err(_) => {
                invalid += 1;
                continue;
            }
        };
        let relative_path = match relative_project_path(root, entry.path()) {
            Ok(relative_path) => relative_path,
            Err(_) => {
                invalid += 1;
                continue;
            }
        };
        projects.push(ScannedProject {
            project,
            relative_path,
        });
    }

    Ok((projects, scanned, invalid))
}

fn read_project(path: &Path) -> Result<Project> {
    let content = fs::read_to_string(path).map_err(|error| YtpmError::io(path, error))?;
    let mut value: JsonValue = serde_json::from_str(&content)?;
    let schema_version = value
        .get("schema_version")
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| YtpmError::InvalidProject("缺少有效的 schema_version".into()))?;

    if schema_version == 1 {
        let object = value
            .as_object_mut()
            .ok_or_else(|| YtpmError::InvalidProject("project.json 必須是 JSON object".into()))?;
        object.insert(
            "schema_version".into(),
            JsonValue::from(CURRENT_SCHEMA_VERSION),
        );
        object.insert("archived_from_status".into(), JsonValue::Null);
    } else if schema_version != u64::from(CURRENT_SCHEMA_VERSION) {
        return Err(YtpmError::InvalidProject(format!(
            "不支援 schema_version {schema_version}，目前版本為 {CURRENT_SCHEMA_VERSION}"
        )));
    }

    let project: Project = serde_json::from_value(value)?;
    if project.progress > 100 || project.title.trim().is_empty() {
        return Err(YtpmError::InvalidProject(
            "project.json 的 title 或 progress 無效".into(),
        ));
    }
    Ok(project)
}

fn open_connection(db_path: &Path) -> Result<Connection> {
    Connection::open(db_path).map_err(|error| sqlite_error(db_path, "開啟 SQLite index", error))
}

fn apply_migrations(connection: &mut Connection, db_path: &Path) -> Result<()> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|error| sqlite_error(db_path, "啟用 SQLite foreign keys", error))?;
    // Bootstrap only the migration ledger. Each migration is then applied once,
    // in its own transaction, so a future migration cannot be silently replayed
    // or leave half-written schema metadata behind.
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(|error| sqlite_error(db_path, "建立 SQLite migration ledger", error))?;

    for (version, sql) in [
        (1_i64, MIGRATION_0001),
        (2, MIGRATION_0002),
        (3, MIGRATION_0003),
    ] {
        let applied: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [version],
                |row| row.get(0),
            )
            .map_err(|error| sqlite_error(db_path, "讀取 SQLite migration ledger", error))?;
        if applied {
            continue;
        }
        let transaction = connection
            .transaction()
            .map_err(|error| sqlite_error(db_path, "開始 SQLite migration transaction", error))?;
        transaction
            .execute_batch(sql)
            .map_err(|error| sqlite_error(db_path, "套用 SQLite migration", error))?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at)
                 VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                [version],
            )
            .map_err(|error| sqlite_error(db_path, "記錄 SQLite migration", error))?;
        transaction
            .commit()
            .map_err(|error| sqlite_error(db_path, "提交 SQLite migration", error))?;
    }
    Ok(())
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_DIRECTORY).join(INDEX_FILE)
}

fn project_file_for_relative_path(
    root: &Path,
    relative_path: &str,
    db_path: &Path,
) -> Result<PathBuf> {
    let relative = Path::new(relative_path);
    validate_relative_path(relative).map_err(|error| {
        YtpmError::InvalidInput(format!(
            "SQLite index {} 的 relative_path 無效：{error}。請刪除 cache 後重新 rebuild_index。",
            db_path.display()
        ))
    })?;
    Ok(root.join(relative).join("project.json"))
}

fn relative_project_path(root: &Path, project_file: &Path) -> Result<String> {
    let project_directory = project_file.parent().ok_or_else(|| {
        YtpmError::InvalidProject(format!(
            "project.json 缺少 parent：{}",
            project_file.display()
        ))
    })?;
    let relative = project_directory.strip_prefix(root).map_err(|_| {
        YtpmError::InvalidProject(format!(
            "project.json 超出 Library root：{}",
            project_file.display()
        ))
    })?;
    validate_relative_path(relative)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(YtpmError::InvalidInput(format!(
            "relative_path 必須是非空相對路徑：{}",
            path.display()
        )));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir | Component::CurDir
        )
    }) {
        return Err(YtpmError::InvalidInput(format!(
            "relative_path 不可包含絕對路徑或 .、..：{}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_no_parent_components(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(format!(
            "Library root 路徑不可包含 ..：{}",
            path.display()
        )));
    }
    Ok(())
}

fn is_ignored_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .map(|relative| {
            relative.components().any(|component| {
                component.as_os_str() == OsStr::new(INDEX_DIRECTORY)
                    || component.as_os_str() == OsStr::new("_archive")
            })
        })
        .unwrap_or(false)
}

fn project_file_mtime_ms(root: &Path, relative_path: &str) -> Option<i64> {
    let path = root.join(relative_path).join("project.json");
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(duration.as_millis()).ok()
}

fn status_key(status: &ProjectStatus) -> &'static str {
    match status {
        ProjectStatus::Idea => "idea",
        ProjectStatus::Research => "research",
        ProjectStatus::Script => "script",
        ProjectStatus::Voice => "voice",
        ProjectStatus::Visuals => "visuals",
        ProjectStatus::Editing => "editing",
        ProjectStatus::Subtitles => "subtitles",
        ProjectStatus::Thumbnail => "thumbnail",
        ProjectStatus::Review => "review",
        ProjectStatus::Scheduled => "scheduled",
        ProjectStatus::Published => "published",
        ProjectStatus::Archived => "archived",
    }
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn safe_fts_query(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn next_placeholder(index: usize) -> String {
    format!("?{index}")
}

fn project_matches_query(project: &Project, query: &str) -> bool {
    let query = query.to_lowercase();
    [
        Some(project.title.as_str()),
        project.channel.as_deref(),
        project.series.as_deref(),
    ]
    .into_iter()
    .flatten()
    .chain(project.tags.iter().map(String::as_str))
    .any(|field| field.to_lowercase().contains(&query))
}

fn sqlite_error<E: std::fmt::Display>(db_path: &Path, operation: &str, error: E) -> YtpmError {
    YtpmError::InvalidInput(format!(
        "SQLite index {operation} 失敗（{}）：{error}。這個 cache 可能損壞；請保留 project.json 與素材，刪除 {} 後重新執行 rebuild_index。",
        db_path.display(),
        db_path.display()
    ))
}
