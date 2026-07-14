mod error {
    pub use ytpm_core::{Result, YtpmError};
}

#[path = "../src/task_service.rs"]
mod task_service;

use chrono::{DateTime, Utc};
use std::fs;
use task_service::{
    create_task, list_tasks, move_task, update_task, TaskFile, TaskPatch, TaskPriority,
    TaskRequest, TaskStatus,
};
use tempfile::tempdir;

fn request(title: &str) -> TaskRequest {
    TaskRequest {
        title: title.to_string(),
        description: Some("  原始描述  ".into()),
        status: TaskStatus::Todo,
        priority: TaskPriority::Normal,
        order_key: 10.0,
        due_at: None,
        related_asset_ids: Vec::new(),
        acceptance_criteria: vec!["  完成測試  ".into(), "".into()],
    }
}

#[test]
fn missing_tasks_file_is_created_and_lists_empty() {
    let temp = tempdir().expect("tempdir");
    let project_dir = temp.path().join("中文影片");

    let tasks = list_tasks(&project_dir).expect("list missing tasks");

    assert!(tasks.is_empty());
    assert!(project_dir.join("tasks.json").is_file());
    let file: TaskFile = serde_json::from_str(
        &fs::read_to_string(project_dir.join("tasks.json")).expect("read tasks"),
    )
    .expect("parse tasks");
    assert_eq!(file.schema_version, 1);
}

#[test]
fn creates_and_lists_trimmed_chinese_task() {
    let temp = tempdir().expect("tempdir");
    let task = create_task(temp.path(), request("  完成中文旁白  ")).expect("create task");

    assert_eq!(task.title, "完成中文旁白");
    assert_eq!(task.description.as_deref(), Some("原始描述"));
    assert_eq!(task.acceptance_criteria, vec!["完成測試"]);
    assert!(uuid::Uuid::parse_str(&task.id).is_ok());

    let listed = list_tasks(temp.path()).expect("list tasks");
    assert_eq!(listed, vec![task]);
}

#[test]
fn patch_updates_title_description_priority_due_date_and_acceptance() {
    let temp = tempdir().expect("tempdir");
    let task = create_task(temp.path(), request("原標題")).expect("create task");
    let due_at = DateTime::parse_from_rfc3339("2026-07-20T12:00:00Z")
        .expect("parse due date")
        .with_timezone(&Utc);

    let updated = update_task(
        temp.path(),
        &task.id,
        TaskPatch {
            title: Some("  新標題  ".into()),
            description: Some(Some("  新描述  ".into())),
            priority: Some(TaskPriority::Urgent),
            due_at: Some(Some(due_at)),
            acceptance_criteria: Some(vec!["  可驗收  ".into()]),
        },
    )
    .expect("patch task");

    assert_eq!(updated.title, "新標題");
    assert_eq!(updated.description.as_deref(), Some("新描述"));
    assert_eq!(updated.priority, TaskPriority::Urgent);
    assert_eq!(updated.due_at, Some(due_at));
    assert_eq!(updated.acceptance_criteria, vec!["可驗收"]);
    assert_eq!(list_tasks(temp.path()).expect("list")[0], updated);
}

#[test]
fn move_updates_status_and_order_key() {
    let temp = tempdir().expect("tempdir");
    let task = create_task(temp.path(), request("拖曳任務")).expect("create task");

    let moved = move_task(temp.path(), &task.id, TaskStatus::Doing, 2.5).expect("move task");

    assert_eq!(moved.status, TaskStatus::Doing);
    assert_eq!(moved.order_key, 2.5);
    assert_eq!(
        list_tasks(temp.path()).expect("list")[0].status,
        TaskStatus::Doing
    );
}

#[test]
fn done_timestamp_is_set_and_cleared_when_leaving_done() {
    let temp = tempdir().expect("tempdir");
    let task = create_task(temp.path(), request("完成狀態")).expect("create task");

    let done = move_task(temp.path(), &task.id, TaskStatus::Done, 1.0).expect("mark done");
    let completed_at = done.completed_at.expect("completed timestamp");
    assert!(completed_at >= done.created_at);

    let reopened = move_task(temp.path(), &task.id, TaskStatus::Doing, 1.0).expect("reopen");
    assert_eq!(reopened.completed_at, None);
}

#[test]
fn invalid_id_is_rejected() {
    let temp = tempdir().expect("tempdir");
    create_task(temp.path(), request("ID 測試")).expect("create task");

    let error = update_task(temp.path(), "not-a-uuid", TaskPatch::default())
        .expect_err("invalid id should fail");

    assert!(error.to_string().contains("不是有效 UUID"));
}

#[test]
fn corrupt_tasks_json_error_is_not_swallowed() {
    let temp = tempdir().expect("tempdir");
    fs::write(temp.path().join("tasks.json"), "{ this is not json").expect("write corrupt tasks");

    let error = list_tasks(temp.path()).expect_err("corrupt JSON should fail");

    assert!(error.to_string().contains("JSON 格式錯誤"));
}

#[test]
fn unknown_status_and_negative_order_key_are_rejected() {
    let temp = tempdir().expect("tempdir");
    let task = create_task(temp.path(), request("輸入驗證")).expect("create task");

    let negative = move_task(temp.path(), &task.id, TaskStatus::Doing, -1.0)
        .expect_err("negative order key should fail");
    assert!(negative.to_string().contains("order_key"));

    let json = fs::read_to_string(temp.path().join("tasks.json")).expect("read tasks");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse tasks");
    value["tasks"][0]["status"] = serde_json::Value::String("unknown".into());
    fs::write(
        temp.path().join("tasks.json"),
        serde_json::to_vec_pretty(&value).expect("serialize corrupt status"),
    )
    .expect("write unknown status");

    let unknown = list_tasks(temp.path()).expect_err("unknown status should fail");
    assert!(unknown.to_string().contains("JSON 格式錯誤"));
}

#[test]
fn atomic_write_leaves_valid_json_without_temp_files() {
    let temp = tempdir().expect("tempdir");
    create_task(temp.path(), request("原子落盤")).expect("create task");
    let tasks_path = temp.path().join("tasks.json");

    let content = fs::read_to_string(&tasks_path).expect("read tasks");
    let _: TaskFile = serde_json::from_str(&content).expect("valid complete JSON");
    let temporary_files: Vec<_> = fs::read_dir(temp.path())
        .expect("read project dir")
        .map(|entry| entry.expect("directory entry").file_name())
        .filter(|name| name.to_string_lossy().starts_with(".tasks.json."))
        .collect();

    assert!(temporary_files.is_empty());
    assert!(content.ends_with('\n'));
}
