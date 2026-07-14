//! Portable task/Kanban service backed by the project's `tasks.json` file.
//!
//! `tasks.json` is the source of truth and is written through a temporary file
//! followed by a rename. This service intentionally has no delete API: tasks
//! and media are not permanently deleted by the task layer.

use crate::error::{Result, YtpmError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const TASK_SCHEMA_VERSION: u32 = 1;
const MAX_TASKS_JSON_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Todo,
    Doing,
    Review,
    Blocked,
    Done,
}

impl TaskStatus {
    fn sort_rank(self) -> u8 {
        match self {
            Self::Todo => 0,
            Self::Doing => 1,
            Self::Review => 2,
            Self::Blocked => 3,
            Self::Done => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    #[serde(default)]
    pub order_key: f64,
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub related_asset_ids: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaskRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub priority: TaskPriority,
    #[serde(default)]
    pub order_key: f64,
    #[serde(default)]
    pub due_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub related_asset_ids: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaskPatch {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub due_at: Option<Option<DateTime<Utc>>>,
    #[serde(default)]
    pub acceptance_criteria: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaskFile {
    pub schema_version: u32,
    pub tasks: Vec<Task>,
}

impl Default for TaskFile {
    fn default() -> Self {
        Self {
            schema_version: TASK_SCHEMA_VERSION,
            tasks: Vec::new(),
        }
    }
}

/// Lists tasks in Kanban status/order order, creating an empty `tasks.json`
/// when the file does not exist.
pub fn list_tasks(project_dir: &Path) -> Result<Vec<Task>> {
    let mut task_file = read_task_file(project_dir)?;
    sort_tasks(&mut task_file.tasks);
    Ok(task_file.tasks)
}

/// Creates a task with a UUID and normalized text, then atomically persists it.
pub fn create_task(project_dir: &Path, request: TaskRequest) -> Result<Task> {
    let mut task_file = read_task_file(project_dir)?;
    let title = normalize_title(request.title)?;
    validate_order_key(request.order_key)?;
    let related_asset_ids = normalize_asset_ids(request.related_asset_ids)?;
    let acceptance_criteria = normalize_acceptance_criteria(request.acceptance_criteria);
    let now = Utc::now();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title,
        description: normalize_optional_text(request.description),
        status: request.status,
        priority: request.priority,
        order_key: request.order_key,
        due_at: request.due_at,
        completed_at: (request.status == TaskStatus::Done).then_some(now),
        related_asset_ids,
        acceptance_criteria,
        created_at: now,
        updated_at: now,
    };

    task_file.tasks.push(task.clone());
    validate_task_file(&task_file)?;
    sort_tasks(&mut task_file.tasks);
    write_task_file(project_dir, &task_file)?;
    Ok(task)
}

/// Applies metadata fields to a task and atomically persists the task file.
pub fn update_task(project_dir: &Path, id: &str, patch: TaskPatch) -> Result<Task> {
    let task_id = parse_task_id(id)?;
    let mut task_file = read_task_file(project_dir)?;
    let task = task_file
        .tasks
        .iter_mut()
        .find(|task| task.id == task_id)
        .ok_or_else(|| YtpmError::InvalidInput(format!("找不到 task id：{task_id}")))?;

    if let Some(title) = patch.title {
        task.title = normalize_title(title)?;
    }
    if let Some(description) = patch.description {
        task.description = description.and_then(|value| normalize_optional_text(Some(value)));
    }
    if let Some(priority) = patch.priority {
        task.priority = priority;
    }
    if let Some(due_at) = patch.due_at {
        task.due_at = due_at;
    }
    if let Some(acceptance_criteria) = patch.acceptance_criteria {
        task.acceptance_criteria = normalize_acceptance_criteria(acceptance_criteria);
    }
    task.updated_at = Utc::now();
    let updated = task.clone();

    validate_task_file(&task_file)?;
    sort_tasks(&mut task_file.tasks);
    write_task_file(project_dir, &task_file)?;
    Ok(updated)
}

/// Moves a task for Kanban drag/drop and manages its completion timestamp.
pub fn move_task(project_dir: &Path, id: &str, status: TaskStatus, order_key: f64) -> Result<Task> {
    let task_id = parse_task_id(id)?;
    validate_order_key(order_key)?;
    let mut task_file = read_task_file(project_dir)?;
    let task = task_file
        .tasks
        .iter_mut()
        .find(|task| task.id == task_id)
        .ok_or_else(|| YtpmError::InvalidInput(format!("找不到 task id：{task_id}")))?;
    let was_done = task.status == TaskStatus::Done;
    let is_done = status == TaskStatus::Done;
    let now = Utc::now();

    task.status = status;
    task.order_key = order_key;
    match (was_done, is_done) {
        (false, true) => task.completed_at = Some(now),
        (true, false) => task.completed_at = None,
        _ => {}
    }
    task.updated_at = now;
    let moved = task.clone();

    validate_task_file(&task_file)?;
    sort_tasks(&mut task_file.tasks);
    write_task_file(project_dir, &task_file)?;
    Ok(moved)
}

fn read_task_file(project_dir: &Path) -> Result<TaskFile> {
    ensure_safe_project_dir(project_dir)?;
    let path = tasks_path(project_dir);
    reject_reparse_points(&path)?;

    if !path.exists() {
        fs::create_dir_all(project_dir).map_err(|error| YtpmError::io(project_dir, error))?;
        reject_reparse_points(project_dir)?;
        let task_file = TaskFile::default();
        write_task_file(project_dir, &task_file)?;
        return Ok(task_file);
    }

    let metadata = fs::metadata(&path).map_err(|error| YtpmError::io(&path, error))?;
    if metadata.len() > MAX_TASKS_JSON_BYTES {
        return Err(YtpmError::InvalidProject(format!(
            "tasks.json 超過 {} MiB 上限",
            MAX_TASKS_JSON_BYTES / (1024 * 1024)
        )));
    }
    let content = fs::read_to_string(&path).map_err(|error| YtpmError::io(&path, error))?;
    let task_file: TaskFile = serde_json::from_str(&content)?;
    validate_task_file(&task_file)?;
    Ok(task_file)
}

fn write_task_file(project_dir: &Path, task_file: &TaskFile) -> Result<()> {
    ensure_safe_project_dir(project_dir)?;
    let path = tasks_path(project_dir);
    reject_reparse_points(project_dir)?;
    atomic_write_json(&path, task_file)
}

fn tasks_path(project_dir: &Path) -> PathBuf {
    project_dir.join("tasks.json")
}

fn validate_task_file(task_file: &TaskFile) -> Result<()> {
    if task_file.schema_version != TASK_SCHEMA_VERSION {
        return Err(YtpmError::InvalidProject(format!(
            "不支援 tasks.json schema_version {}，目前版本為 {TASK_SCHEMA_VERSION}",
            task_file.schema_version
        )));
    }

    let mut task_ids = HashSet::with_capacity(task_file.tasks.len());
    for task in &task_file.tasks {
        if Uuid::parse_str(&task.id).is_err() {
            return Err(YtpmError::InvalidProject(format!(
                "task id 不是有效 UUID：{}",
                task.id
            )));
        }
        if !task_ids.insert(task.id.as_str()) {
            return Err(YtpmError::InvalidProject(format!(
                "task id 不可重複：{}",
                task.id
            )));
        }
        if task.title.trim().is_empty() {
            return Err(YtpmError::InvalidProject("task title 不可為空".into()));
        }
        validate_order_key(task.order_key)?;
        for asset_id in &task.related_asset_ids {
            if Uuid::parse_str(asset_id).is_err() {
                return Err(YtpmError::InvalidProject(format!(
                    "related_asset_ids 含無效 UUID：{asset_id}"
                )));
            }
        }
    }
    Ok(())
}

fn normalize_title(title: String) -> Result<String> {
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(YtpmError::InvalidInput("任務標題不可為空".into()));
    }
    Ok(title)
}

fn deserialize_double_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    })
}

fn normalize_acceptance_criteria(criteria: Vec<String>) -> Vec<String> {
    criteria
        .into_iter()
        .map(|criterion| criterion.trim().to_string())
        .filter(|criterion| !criterion.is_empty())
        .collect()
}

fn normalize_asset_ids(asset_ids: Vec<String>) -> Result<Vec<String>> {
    asset_ids
        .into_iter()
        .map(|asset_id| {
            let asset_id = asset_id.trim().to_string();
            if Uuid::parse_str(&asset_id).is_err() {
                return Err(YtpmError::InvalidInput(format!(
                    "related asset id 不是有效 UUID：{asset_id}"
                )));
            }
            Ok(asset_id)
        })
        .collect()
}

fn parse_task_id(id: &str) -> Result<String> {
    let id = id.trim();
    if Uuid::parse_str(id).is_err() {
        return Err(YtpmError::InvalidInput(format!(
            "task id 不是有效 UUID：{id}"
        )));
    }
    Ok(id.to_string())
}

fn validate_order_key(order_key: f64) -> Result<()> {
    if !order_key.is_finite() || order_key < 0.0 {
        return Err(YtpmError::InvalidInput(
            "order_key 必須是有限且不小於 0 的數字".into(),
        ));
    }
    Ok(())
}

fn sort_tasks(tasks: &mut [Task]) {
    tasks.sort_by(|left, right| {
        left.status
            .sort_rank()
            .cmp(&right.status.sort_rank())
            .then_with(|| left.order_key.total_cmp(&right.order_key))
    });
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 JSON parent：{}", path.display()))
    })?;
    reject_reparse_points(parent)?;
    fs::create_dir_all(parent).map_err(|error| YtpmError::io(parent, error))?;
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
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.write_all(&bytes)
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.write_all(b"\n")
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temp_path, error))?;
        fs::rename(&temp_path, path).map_err(|error| YtpmError::io(path, error))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn ensure_safe_project_dir(project_dir: &Path) -> Result<()> {
    if project_dir
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(format!(
            "路徑不可包含 ..：{}",
            project_dir.display()
        )));
    }
    reject_reparse_points(project_dir)
}

fn reject_reparse_points(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if let Ok(metadata) = fs::symlink_metadata(candidate) {
            if metadata_is_reparse_point(&metadata) {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse path：{}",
                    candidate.display()
                )));
            }
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
