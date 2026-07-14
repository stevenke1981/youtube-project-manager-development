use std::fs;
use tempfile::tempdir;
use ytpm_core::{
    create_project, update_project_status, CreateProjectRequest, Project, ProjectStatus, Result,
    YtpmError, CURRENT_SCHEMA_VERSION,
};

#[path = "../src/index.rs"]
mod index;

use index::{rebuild_index, search_index};

fn request(title: &str, channel: &str, tags: &[&str]) -> CreateProjectRequest {
    CreateProjectRequest {
        title: title.into(),
        channel: Some(channel.into()),
        series: Some("離線系列".into()),
        aspect_ratio: "16:9".into(),
        language: "zh-TW".into(),
        target_duration_seconds: Some(300),
        planned_publish_at: None,
        tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
    }
}

#[test]
fn rebuild_creates_sqlite_without_changing_project_json() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(
        temp.path(),
        request("中文索引測試", "測試頻道", &["SQLite"]),
    )
    .expect("create project");
    let project_file = temp.path().join(&project.folder_name).join("project.json");
    let before = fs::read(&project_file).expect("read project before rebuild");

    let report = rebuild_index(temp.path()).expect("rebuild index");

    assert_eq!(report.scanned, 1);
    assert_eq!(report.indexed, 1);
    assert_eq!(report.invalid, 0);
    assert_eq!(report.db_path, temp.path().join(".ytpm/index.sqlite3"));
    assert!(report.db_path.is_file());
    assert_eq!(
        fs::read(&project_file).expect("read project after rebuild"),
        before
    );
}

#[test]
fn search_uses_title_tags_and_status_from_index_and_source_json() {
    let temp = tempdir().expect("tempdir");
    let matching = create_project(
        temp.path(),
        request("尋找中文節目", "科技頻道", &["SQLite", "離線"]),
    )
    .expect("matching project");
    let other = create_project(temp.path(), request("另一個節目", "生活頻道", &["日常"]))
        .expect("other project");
    update_project_status(
        &temp.path().join(&matching.folder_name),
        ProjectStatus::Review,
    )
    .expect("update matching status");
    update_project_status(&temp.path().join(&other.folder_name), ProjectStatus::Script)
        .expect("update other status");
    rebuild_index(temp.path()).expect("rebuild index");

    let title_results = search_index(temp.path(), Some("中文節目"), None).expect("title search");
    assert_eq!(
        title_results
            .iter()
            .map(|project| &project.id)
            .collect::<Vec<_>>(),
        vec![&matching.id]
    );

    let tag_results = search_index(temp.path(), Some("SQLite"), None).expect("tag search");
    assert_eq!(
        tag_results
            .iter()
            .map(|project| &project.id)
            .collect::<Vec<_>>(),
        vec![&matching.id]
    );

    let status_results =
        search_index(temp.path(), None, Some(ProjectStatus::Review)).expect("status search");
    assert_eq!(
        status_results
            .iter()
            .map(|project| &project.id)
            .collect::<Vec<_>>(),
        vec![&matching.id]
    );
    assert_eq!(status_results[0].status, ProjectStatus::Review);
}

#[test]
fn deleted_index_can_be_rebuilt_and_recovers_search_results() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("可重建索引", "頻道", &[])).expect("create");
    let report = rebuild_index(temp.path()).expect("first rebuild");
    fs::remove_file(&report.db_path).expect("delete derived index");

    let results =
        search_index(temp.path(), Some("可重建"), None).expect("rebuild on missing index");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, project.id);
    assert!(temp.path().join(".ytpm/index.sqlite3").is_file());
}

#[test]
fn rebuild_supports_chinese_and_whitespace_library_paths() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("中文 Library 空白 路徑");
    fs::create_dir_all(&root).expect("create library root");
    let project =
        create_project(&root, request("空白路徑影片", "中文頻道", &["路徑"])).expect("create");

    let report = rebuild_index(&root).expect("rebuild index");
    let results = search_index(&root, Some("空白路徑"), None).expect("search index");

    assert_eq!(report.indexed, 1);
    assert_eq!(results[0].id, project.id);
    assert!(report.db_path.is_file());
}

#[test]
fn rebuild_reports_invalid_project_json_without_indexing_it() {
    let temp = tempdir().expect("tempdir");
    let invalid_dir = temp.path().join("invalid project");
    fs::create_dir_all(&invalid_dir).expect("create invalid project directory");
    fs::write(invalid_dir.join("project.json"), "{ not valid json").expect("write invalid project");

    let report = rebuild_index(temp.path()).expect("rebuild index");

    assert_eq!(report.scanned, 1);
    assert_eq!(report.indexed, 0);
    assert_eq!(report.invalid, 1);
}

#[test]
fn corrupted_cache_returns_actionable_error_instead_of_rebuilding_silently() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join(".ytpm/index.sqlite3");
    fs::create_dir_all(db_path.parent().expect("index parent")).expect("create index directory");
    fs::write(&db_path, b"not a sqlite database").expect("write corrupt cache");

    let error = search_index(temp.path(), None, None).expect_err("corrupt cache must fail");
    let message = error.to_string();
    assert!(message.contains("SQLite index"));
    assert!(message.contains("重新執行 rebuild_index"));
}
