use crate::error::{Result, YtpmError};
use crate::folder_template::{expected_directories, template_files};
use crate::migration::{migrate_project_value, CURRENT_SCHEMA_VERSION};
use crate::model::{
    CreateProjectRequest, Project, ProjectStatus, ValidationIssue, ValidationReport,
    ValidationSeverity,
};
use chrono::{Local, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, serde::Serialize, Deserialize)]
struct OperationJournal {
    id: String,
    operation: String,
    source: PathBuf,
    destination: PathBuf,
    phase: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
pub struct RecoveryReport {
    pub journal_found: bool,
    pub journal_cleared: bool,
    pub operation: Option<String>,
    pub phase: Option<String>,
}

pub fn create_project(root: &Path, request: CreateProjectRequest) -> Result<Project> {
    ensure_no_parent_components(root)?;
    reject_reparse_points(root)?;
    let title = request.title.trim();
    if title.is_empty() {
        return Err(YtpmError::InvalidInput("影片標題不可為空".into()));
    }
    if title.chars().count() > 200 {
        return Err(YtpmError::InvalidInput(
            "影片標題不可超過 200 個字元".into(),
        ));
    }
    if !is_supported_aspect_ratio(&request.aspect_ratio) {
        return Err(YtpmError::InvalidInput(format!(
            "不支援的畫面比例：{}",
            request.aspect_ratio
        )));
    }
    if request.language.trim().chars().count() < 2 {
        return Err(YtpmError::InvalidInput("語言代碼至少需要 2 個字元".into()));
    }

    fs::create_dir_all(root).map_err(|e| YtpmError::io(root, e))?;
    reject_reparse_points(root)?;
    let safe_title = sanitize_component(title);
    let date = Local::now().format("%Y-%m-%d").to_string();
    let base_name = format!("{date}_{safe_title}");
    let project_dir = allocate_unique_directory(root, &base_name);
    fs::create_dir(&project_dir).map_err(|e| YtpmError::io(&project_dir, e))?;

    if let Err(error) = populate_project_directory(&project_dir, title) {
        // Best effort cleanup only because this directory was allocated in this operation.
        let _ = fs::remove_dir_all(&project_dir);
        return Err(error);
    }

    let now = Utc::now();
    let mut seen_tags = HashSet::new();
    let tags = request
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen_tags.insert(tag.clone()))
        .collect();
    let project = Project {
        schema_version: CURRENT_SCHEMA_VERSION,
        id: Uuid::new_v4().to_string(),
        title: title.to_string(),
        folder_name: project_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&base_name)
            .to_string(),
        channel: normalized_optional(request.channel),
        series: normalized_optional(request.series),
        status: ProjectStatus::Idea,
        archived_from_status: None,
        aspect_ratio: request.aspect_ratio,
        language: request.language.trim().to_string(),
        target_duration_seconds: request.target_duration_seconds,
        planned_publish_at: request.planned_publish_at,
        published_at: None,
        progress: 0,
        tags,
        created_at: now,
        updated_at: now,
        app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };

    atomic_write_json(&project_dir.join("project.json"), &project)?;
    Ok(project)
}

pub fn list_projects(root: &Path) -> Result<Vec<Project>> {
    recover_operation_journal(root)?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    for entry in WalkDir::new(root).min_depth(1).max_depth(3) {
        let entry = entry.map_err(|e| {
            let path = e.path().unwrap_or(root).to_path_buf();
            YtpmError::InvalidProject(format!("掃描 {} 失敗：{e}", path.display()))
        })?;
        let is_archived = entry
            .path()
            .components()
            .any(|component| component.as_os_str() == OsStr::new("_archive"));
        if !is_archived && entry.file_type().is_file() && entry.file_name() == "project.json" {
            projects.push(read_project(entry.path())?);
        }
    }
    projects.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(projects)
}

/// Validates and clears a completed or not-yet-started archive/restore journal.
///
/// Ambiguous filesystem states are deliberately left untouched for manual recovery.
pub fn recover_operation_journal(library_root: &Path) -> Result<RecoveryReport> {
    let journal_path = library_root.join(".ytpm-operation.json");
    match fs::symlink_metadata(&journal_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RecoveryReport::default());
        }
        Err(error) => return Err(YtpmError::io(&journal_path, error)),
    }

    reject_reparse_points(library_root).map_err(|error| {
        YtpmError::InvalidProject(format!(
            "operation journal 的 Library root 不安全，請移除 symlink/junction 後重試：{error}"
        ))
    })?;
    reject_reparse_points(&journal_path).map_err(|error| {
        YtpmError::InvalidProject(format!(
            "operation journal 路徑不安全，請人工檢查 {}：{error}",
            journal_path.display()
        ))
    })?;

    const MAX_JOURNAL_BYTES: u64 = 64 * 1024;
    let metadata =
        fs::metadata(&journal_path).map_err(|error| YtpmError::io(&journal_path, error))?;
    if metadata.len() > MAX_JOURNAL_BYTES {
        return Err(YtpmError::InvalidProject(format!(
            "operation journal 超過 {} KiB，請人工檢查並移除或修復 {}",
            MAX_JOURNAL_BYTES / 1024,
            journal_path.display()
        )));
    }
    let content = fs::read_to_string(&journal_path).map_err(|error| {
        YtpmError::InvalidProject(format!(
            "無法讀取 operation journal {}：{error}；請人工檢查檔案",
            journal_path.display()
        ))
    })?;
    let journal: OperationJournal = serde_json::from_str(&content).map_err(|error| {
        YtpmError::InvalidProject(format!(
            "operation journal JSON 無效：{error}；請人工檢查 {}",
            journal_path.display()
        ))
    })?;

    if !matches!(journal.operation.as_str(), "archive" | "restore") {
        return Err(invalid_journal_state(
            &journal_path,
            format!(
                "不支援 operation={}，只允許 archive 或 restore",
                journal.operation
            ),
        ));
    }
    if !matches!(journal.phase.as_str(), "prepared" | "moved") {
        return Err(invalid_journal_state(
            &journal_path,
            format!("不支援 phase={}，只允許 prepared 或 moved", journal.phase),
        ));
    }

    let root = absolute_path_without_parent(library_root).map_err(|error| {
        invalid_journal_state(&journal_path, format!("Library root 無效：{error}"))
    })?;
    let source = validate_journal_path(&root, &journal.source, "source", &journal_path)?;
    let destination =
        validate_journal_path(&root, &journal.destination, "destination", &journal_path)?;
    let source_exists = path_exists(&source)?;
    let destination_exists = path_exists(&destination)?;

    let state_is_safe = match journal.phase.as_str() {
        "prepared" => source_exists && !destination_exists,
        "moved" => !source_exists && destination_exists,
        _ => false,
    };
    if !state_is_safe {
        return Err(invalid_journal_state(
            &journal_path,
            format!(
                "phase={} 與目前 filesystem 狀態不一致（source_exists={}, destination_exists={}）；請確認後手動完成或回復移動",
                journal.phase, source_exists, destination_exists
            ),
        ));
    }

    if journal.phase == "moved" {
        let project_path = destination.join("project.json");
        if path_exists(&project_path)? {
            let mut project = read_project(&project_path).map_err(|error| {
                invalid_journal_state(
                    &journal_path,
                    format!("已移動的專案 metadata 無法讀取：{error}"),
                )
            })?;
            let folder_name = destination
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    invalid_journal_state(&journal_path, "destination 資料夾名稱無效".into())
                })?
                .to_string();
            let changed = match journal.operation.as_str() {
                "archive" if project.status != ProjectStatus::Archived => {
                    project.folder_name = folder_name;
                    project.archived_from_status = Some(project.status.clone());
                    project.status = ProjectStatus::Archived;
                    project.updated_at = Utc::now();
                    true
                }
                "restore" if project.status == ProjectStatus::Archived => {
                    project.folder_name = folder_name;
                    project.status = project
                        .archived_from_status
                        .take()
                        .unwrap_or(ProjectStatus::Idea);
                    project.updated_at = Utc::now();
                    true
                }
                _ => false,
            };
            if changed {
                atomic_write_json(&project_path, &project).map_err(|error| {
                    invalid_journal_state(
                        &journal_path,
                        format!("已移動專案但 metadata 尚未完成：{error}"),
                    )
                })?;
            }
        }
    }

    fs::remove_file(&journal_path).map_err(|error| YtpmError::io(&journal_path, error))?;
    Ok(RecoveryReport {
        journal_found: true,
        journal_cleared: true,
        operation: Some(journal.operation),
        phase: Some(journal.phase),
    })
}

pub fn validate_project(project_dir: &Path) -> Result<ValidationReport> {
    let mut issues = Vec::new();
    let project_file = project_dir.join("project.json");
    let project = match read_project(&project_file) {
        Ok(project) => Some(project),
        Err(error) => {
            issues.push(ValidationIssue {
                code: "PROJECT_JSON_INVALID".into(),
                severity: ValidationSeverity::Error,
                message: error.to_string(),
                path: Some(project_file.display().to_string()),
                suggested_action: Some("從備份恢復，或修正 project.json 格式".into()),
            });
            None
        }
    };

    for relative in expected_directories() {
        let path = project_dir.join(relative);
        let is_reparse = fs::symlink_metadata(&path)
            .map(|metadata| metadata_is_reparse_point(&metadata))
            .unwrap_or(false);
        if is_reparse {
            issues.push(ValidationIssue {
                code: "REQUIRED_DIRECTORY_SYMLINK".into(),
                severity: ValidationSeverity::Error,
                message: format!("標準資料夾不可是 symlink/junction：{relative}"),
                path: Some(path.display().to_string()),
                suggested_action: Some(
                    "移除連結並建立實體資料夾；不要讓素材路徑指向 Library 外".into(),
                ),
            });
        } else if !path.is_dir() {
            issues.push(ValidationIssue {
                code: "REQUIRED_DIRECTORY_MISSING".into(),
                severity: ValidationSeverity::Error,
                message: format!("缺少標準資料夾：{relative}"),
                path: Some(path.display().to_string()),
                suggested_action: Some("建立缺少的資料夾；不要移動既有素材".into()),
            });
        }
    }

    let valid = !issues
        .iter()
        .any(|issue| matches!(issue.severity, ValidationSeverity::Error));
    Ok(ValidationReport {
        valid,
        project,
        issues,
    })
}

/// Updates a project's workflow status while preserving the portable project.json source of truth.
pub fn update_project_status(project_dir: &Path, status: ProjectStatus) -> Result<Project> {
    ensure_no_parent_components(project_dir)?;
    reject_reparse_points(project_dir)?;

    let project_file = project_dir.join("project.json");
    let mut project = read_project(&project_file)?;
    if matches!(&status, ProjectStatus::Archived) {
        return Err(YtpmError::InvalidInput(
            "不可直接將狀態設為 archived；請使用 archive 操作".into(),
        ));
    }

    project.status = status;
    project.progress = project
        .progress
        .max(minimum_progress_for_status(&project.status));
    project.updated_at = Utc::now();
    atomic_write_json(&project_file, &project)?;
    Ok(project)
}

/// Migrates a v1 project in place after creating a local backup snapshot.
pub fn migrate_project(project_dir: &Path) -> Result<Project> {
    ensure_no_parent_components(project_dir)?;
    reject_reparse_points(project_dir)?;
    let project_file = project_dir.join("project.json");
    let content = fs::read_to_string(&project_file).map_err(|e| YtpmError::io(&project_file, e))?;
    let value: Value = serde_json::from_str(&content)?;
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| YtpmError::InvalidProject("缺少有效的 schema_version".into()))?;
    let project = read_project(&project_file)?;
    if version == u64::from(CURRENT_SCHEMA_VERSION) {
        return Ok(project);
    }

    let backup_dir = project_dir.join(".ytpm-backup");
    fs::create_dir_all(&backup_dir).map_err(|e| YtpmError::io(&backup_dir, e))?;
    let backup_path = backup_dir.join(format!("project.json.{}.bak", Uuid::new_v4().simple()));
    fs::copy(&project_file, &backup_path).map_err(|e| YtpmError::io(&backup_path, e))?;
    atomic_write_json(&project_file, &project)?;
    Ok(project)
}

/// Archives a project by moving its complete portable folder into `_archive`.
/// The project remains the source of truth; no media files are copied into an index.
pub fn archive_project(project_dir: &Path) -> Result<Project> {
    ensure_no_parent_components(project_dir)?;
    reject_reparse_points(project_dir)?;

    let library_root = project_dir.parent().ok_or_else(|| {
        YtpmError::InvalidInput("專案路徑必須是 Library root 的直接子資料夾".into())
    })?;
    reject_reparse_points(library_root)?;
    if project_dir.file_name().and_then(|name| name.to_str()) == Some("_archive") {
        return Err(YtpmError::InvalidInput("不可封存 _archive 資料夾".into()));
    }

    let mut project = read_project(&project_dir.join("project.json"))?;
    if project.status == ProjectStatus::Archived {
        return Err(YtpmError::InvalidInput("專案已封存".into()));
    }

    let folder_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| YtpmError::InvalidInput("專案資料夾名稱不是有效的 Windows 檔名".into()))?
        .to_string();
    let archive_root = library_root.join("_archive");
    fs::create_dir_all(&archive_root).map_err(|e| YtpmError::io(&archive_root, e))?;
    reject_reparse_points(&archive_root)?;
    let destination = archive_root.join(&folder_name);
    if destination.exists() {
        return Err(YtpmError::InvalidInput(format!(
            "封存目的地已存在：{}；請先處理衝突，不會覆寫既有專案",
            destination.display()
        )));
    }

    let journal_path = library_root.join(".ytpm-operation.json");
    if journal_path.exists() {
        return Err(YtpmError::InvalidProject(
            "Library 有尚未完成的 operation journal；請先人工檢查後再操作".into(),
        ));
    }
    let journal_id = Uuid::new_v4().to_string();
    write_operation_journal(
        &journal_path,
        &OperationJournal {
            id: journal_id.clone(),
            operation: "archive".into(),
            source: project_dir.to_path_buf(),
            destination: destination.clone(),
            phase: "prepared".into(),
        },
    )?;

    if let Err(error) = fs::rename(project_dir, &destination) {
        let _ = fs::remove_file(&journal_path);
        return Err(YtpmError::io(&destination, error));
    }

    if let Err(error) = write_operation_journal(
        &journal_path,
        &OperationJournal {
            id: journal_id.clone(),
            operation: "archive".into(),
            source: project_dir.to_path_buf(),
            destination: destination.clone(),
            phase: "moved".into(),
        },
    ) {
        if let Err(rollback_error) = fs::rename(&destination, project_dir) {
            return Err(YtpmError::InvalidProject(format!(
                "封存 journal 更新失敗，且 rollback 也失敗：{error}；{rollback_error}。請保留 {} 供人工恢復",
                journal_path.display()
            )));
        }
        let _ = fs::remove_file(&journal_path);
        return Err(error);
    }

    project.folder_name = folder_name;
    project.archived_from_status = Some(project.status.clone());
    project.status = ProjectStatus::Archived;
    project.updated_at = Utc::now();
    if let Err(error) = atomic_write_json(&destination.join("project.json"), &project) {
        if let Err(rollback_error) = fs::rename(&destination, project_dir) {
            return Err(YtpmError::InvalidProject(format!(
                "封存 metadata 寫入失敗，且 rollback 也失敗：{error}；{rollback_error}。請保留 {} 供人工恢復",
                journal_path.display()
            )));
        }
        let _ = fs::remove_file(&journal_path);
        return Err(error);
    }

    let _ = fs::remove_file(&journal_path);
    Ok(project)
}

/// Restores a project from a Library root's `_archive` folder without overwriting another project.
pub fn restore_project(archived_project_dir: &Path) -> Result<Project> {
    ensure_no_parent_components(archived_project_dir)?;
    reject_reparse_points(archived_project_dir)?;

    let archive_root = archived_project_dir
        .parent()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("_archive"))
        .ok_or_else(|| {
            YtpmError::InvalidInput("還原路徑必須是 Library root\\_archive 的直接子資料夾".into())
        })?;
    let library_root = archive_root
        .parent()
        .ok_or_else(|| YtpmError::InvalidInput("找不到 Library root".into()))?;
    reject_reparse_points(archive_root)?;
    reject_reparse_points(library_root)?;
    let mut project = read_project(&archived_project_dir.join("project.json"))?;
    if project.status != ProjectStatus::Archived {
        return Err(YtpmError::InvalidInput(
            "只有 status=archived 的專案可以還原".into(),
        ));
    }

    let folder_name = archived_project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| YtpmError::InvalidInput("專案資料夾名稱不是有效的 Windows 檔名".into()))?
        .to_string();
    let destination = library_root.join(&folder_name);
    if destination.exists() {
        return Err(YtpmError::InvalidInput(format!(
            "還原目的地已存在：{}；請先處理衝突，不會覆寫既有專案",
            destination.display()
        )));
    }

    let journal_path = library_root.join(".ytpm-operation.json");
    if journal_path.exists() {
        return Err(YtpmError::InvalidProject(
            "Library 有尚未完成的 operation journal；請先人工檢查後再操作".into(),
        ));
    }
    let journal_id = Uuid::new_v4().to_string();
    write_operation_journal(
        &journal_path,
        &OperationJournal {
            id: journal_id.clone(),
            operation: "restore".into(),
            source: archived_project_dir.to_path_buf(),
            destination: destination.clone(),
            phase: "prepared".into(),
        },
    )?;

    if let Err(error) = fs::rename(archived_project_dir, &destination) {
        let _ = fs::remove_file(&journal_path);
        return Err(YtpmError::io(&destination, error));
    }

    if let Err(error) = write_operation_journal(
        &journal_path,
        &OperationJournal {
            id: journal_id,
            operation: "restore".into(),
            source: archived_project_dir.to_path_buf(),
            destination: destination.clone(),
            phase: "moved".into(),
        },
    ) {
        if let Err(rollback_error) = fs::rename(&destination, archived_project_dir) {
            return Err(YtpmError::InvalidProject(format!(
                "還原 journal 更新失敗，且 rollback 也失敗：{error}；{rollback_error}。請保留 {} 供人工恢復",
                journal_path.display()
            )));
        }
        let _ = fs::remove_file(&journal_path);
        return Err(error);
    }

    project.folder_name = folder_name;
    project.status = project
        .archived_from_status
        .take()
        .unwrap_or(ProjectStatus::Idea);
    project.updated_at = Utc::now();
    if let Err(error) = atomic_write_json(&destination.join("project.json"), &project) {
        if let Err(rollback_error) = fs::rename(&destination, archived_project_dir) {
            return Err(YtpmError::InvalidProject(format!(
                "還原 metadata 寫入失敗，且 rollback 也失敗：{error}；{rollback_error}。請保留 {} 供人工恢復",
                journal_path.display()
            )));
        }
        let _ = fs::remove_file(&journal_path);
        return Err(error);
    }

    let _ = fs::remove_file(&journal_path);
    Ok(project)
}

fn populate_project_directory(project_dir: &Path, title: &str) -> Result<()> {
    for relative in expected_directories() {
        let path = project_dir.join(relative);
        fs::create_dir_all(&path).map_err(|e| YtpmError::io(&path, e))?;
    }

    for (relative, template) in template_files() {
        let path = project_dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| YtpmError::io(parent, e))?;
        }
        let content = template.replace("{{TITLE}}", title);
        fs::write(&path, content).map_err(|e| YtpmError::io(&path, e))?;
    }
    Ok(())
}

fn read_project(path: &Path) -> Result<Project> {
    reject_reparse_points(path)?;
    let metadata = fs::metadata(path).map_err(|e| YtpmError::io(path, e))?;
    const MAX_PROJECT_JSON_BYTES: u64 = 1024 * 1024;
    if metadata.len() > MAX_PROJECT_JSON_BYTES {
        return Err(YtpmError::InvalidProject(format!(
            "project.json 超過 {} MiB 上限",
            MAX_PROJECT_JSON_BYTES / (1024 * 1024)
        )));
    }
    let content = fs::read_to_string(path).map_err(|e| YtpmError::io(path, e))?;
    let mut value: Value = serde_json::from_str(&content)?;
    migrate_project_value(&mut value)?;
    let project: Project = serde_json::from_value(value)?;
    if project.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(YtpmError::InvalidProject(format!(
            "不支援 schema_version {}",
            project.schema_version
        )));
    }
    validate_project_fields(&project)?;
    Ok(project)
}

fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 JSON parent：{}", path.display()))
    })?;
    reject_reparse_points(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| YtpmError::InvalidInput(format!("JSON 檔名無效：{}", path.display())))?;
    let temp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| YtpmError::io(&temp_path, e))?;
        file.write_all(&bytes)
            .map_err(|e| YtpmError::io(&temp_path, e))?;
        file.write_all(b"\n")
            .map_err(|e| YtpmError::io(&temp_path, e))?;
        file.sync_all().map_err(|e| YtpmError::io(&temp_path, e))?;
        fs::rename(&temp_path, path).map_err(|e| YtpmError::io(path, e))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn minimum_progress_for_status(status: &ProjectStatus) -> u8 {
    match status {
        ProjectStatus::Idea => 0,
        ProjectStatus::Research => 5,
        ProjectStatus::Script => 25,
        ProjectStatus::Voice => 40,
        ProjectStatus::Visuals => 55,
        ProjectStatus::Editing => 75,
        ProjectStatus::Subtitles => 85,
        ProjectStatus::Thumbnail => 90,
        ProjectStatus::Review => 95,
        ProjectStatus::Scheduled | ProjectStatus::Published => 100,
        ProjectStatus::Archived => 0,
    }
}

fn write_operation_journal(path: &Path, journal: &OperationJournal) -> Result<()> {
    atomic_write_json(path, journal)
}

fn invalid_journal_state(journal_path: &Path, detail: String) -> YtpmError {
    YtpmError::InvalidProject(format!(
        "operation journal {} 需要人工處理：{}",
        journal_path.display(),
        detail
    ))
}

fn path_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(YtpmError::io(path, error)),
    }
}

fn absolute_path_without_parent(path: &Path) -> Result<PathBuf> {
    ensure_no_parent_components(path)?;
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let current_dir = std::env::current_dir().map_err(|error| YtpmError::io(".", error))?;
        Ok(current_dir.join(path))
    }
}

fn validate_journal_path(
    library_root: &Path,
    candidate: &Path,
    label: &str,
    journal_path: &Path,
) -> Result<PathBuf> {
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        || (!candidate.is_absolute()
            && candidate
                .components()
                .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir)))
    {
        return Err(invalid_journal_state(
            journal_path,
            format!(
                "{label} 路徑不可包含 .、.. 或 drive prefix：{}",
                candidate.display()
            ),
        ));
    }
    let candidate = absolute_path_without_parent(candidate).map_err(|error| {
        invalid_journal_state(
            journal_path,
            format!("{label} 路徑無效：{}：{error}", candidate.display()),
        )
    })?;
    if candidate == library_root || !candidate.starts_with(library_root) {
        return Err(invalid_journal_state(
            journal_path,
            format!("{label} 必須位於 Library root 內：{}", candidate.display()),
        ));
    }
    reject_reparse_points(&candidate).map_err(|error| {
        invalid_journal_state(
            journal_path,
            format!("{label} 路徑含有 symlink/junction/reparse parent：{error}"),
        )
    })?;
    Ok(candidate)
}

fn ensure_no_parent_components(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(format!(
            "路徑不可包含 ..：{}",
            path.display()
        )));
    }
    Ok(())
}

fn reject_reparse_points(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) => {
                if metadata_is_reparse_point(&metadata) {
                    return Err(YtpmError::InvalidInput(format!(
                        "拒絕操作 symlink/junction/reparse path：{}",
                        candidate.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(YtpmError::io(candidate, error)),
        }
        current = candidate.parent();
    }
    Ok(())
}

fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn allocate_unique_directory(root: &Path, base_name: &str) -> PathBuf {
    let first = root.join(base_name);
    if !first.exists() {
        return first;
    }
    for number in 2..=999 {
        let candidate = root.join(format!("{base_name}-{number:02}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    root.join(format!("{base_name}-{}", Uuid::new_v4().simple()))
}

fn sanitize_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            output.push('_');
        } else {
            output.push(ch);
        }
    }
    let trimmed = output.trim().trim_end_matches([' ', '.']);
    let mut result = if trimmed.is_empty() {
        "未命名影片"
    } else {
        trimmed
    }
    .to_string();
    if is_windows_reserved_name(&result) {
        result.insert(0, '_');
    }
    if result.chars().count() > 80 {
        result = result.chars().take(80).collect();
    }
    result = result.trim_end_matches([' ', '.']).to_string();
    if result.is_empty() {
        result = "未命名影片".to_string();
    }
    result
}

fn validate_project_fields(project: &Project) -> Result<()> {
    if project.title.trim().is_empty() || project.title.chars().count() > 200 {
        return Err(YtpmError::InvalidProject(
            "title 長度不符合 project schema".into(),
        ));
    }
    if !is_supported_aspect_ratio(&project.aspect_ratio) {
        return Err(YtpmError::InvalidProject(
            "aspect_ratio 不符合 project schema".into(),
        ));
    }
    if project.language.trim().chars().count() < 2 {
        return Err(YtpmError::InvalidProject(
            "language 不符合 project schema".into(),
        ));
    }
    if project.target_duration_seconds == Some(0) {
        return Err(YtpmError::InvalidProject(
            "target_duration_seconds 必須大於 0".into(),
        ));
    }
    if Uuid::parse_str(&project.id).is_err() {
        return Err(YtpmError::InvalidProject("id 不是有效 UUID".into()));
    }
    if project
        .folder_name
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
        || project.folder_name == "."
        || project.folder_name == ".."
    {
        return Err(YtpmError::InvalidProject(
            "folder_name 不是安全的單一資料夾名稱".into(),
        ));
    }
    let mut seen = HashSet::new();
    if project.tags.iter().any(|tag| !seen.insert(tag)) {
        return Err(YtpmError::InvalidProject("tags 不可包含重複值".into()));
    }
    Ok(())
}

fn is_supported_aspect_ratio(value: &str) -> bool {
    matches!(value, "16:9" | "9:16" | "1:1" | "4:3" | "custom")
}

fn is_windows_reserved_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem[3..].parse::<u8>().is_ok_and(|n| (1..=9).contains(&n))
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_windows_characters_and_reserved_names() {
        assert_eq!(sanitize_component("測試:影片?"), "測試_影片_");
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component("title. "), "title");
    }
}
