use std::fs;
use tempfile::tempdir;
use ytpm_core::{
    archive_project, create_project, list_projects, migrate_project, restore_project,
    validate_project, CreateProjectRequest, ProjectStatus,
};

fn request(title: &str) -> CreateProjectRequest {
    CreateProjectRequest {
        title: title.to_string(),
        channel: Some("測試頻道".into()),
        series: None,
        aspect_ratio: "16:9".into(),
        language: "zh-TW".into(),
        target_duration_seconds: Some(480),
        planned_publish_at: None,
        tags: vec!["AI".into()],
    }
}

#[test]
fn creates_lists_and_validates_a_project() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("中文影片：第一集")).expect("create project");

    let project_dir = temp.path().join(&project.folder_name);
    assert!(project_dir.join("project.json").is_file());
    assert!(project_dir.join("07_thumbnail/final").is_dir());

    let listed = list_projects(temp.path()).expect("list projects");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, project.id);

    let report = validate_project(&project_dir).expect("validate project");
    assert!(report.valid);
    assert!(report.issues.is_empty());
}

#[test]
fn allocates_unique_folder_for_same_title() {
    let temp = tempdir().expect("tempdir");
    let first = create_project(temp.path(), request("同名影片")).expect("first");
    let second = create_project(temp.path(), request("同名影片")).expect("second");
    assert_ne!(first.folder_name, second.folder_name);
    assert!(second.folder_name.ends_with("-02"));
}

#[test]
fn validation_reports_missing_directory() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("驗證測試")).expect("create");
    let project_dir = temp.path().join(project.folder_name);
    fs::remove_dir(project_dir.join("06_subtitles/translations")).expect("remove empty dir");

    let report = validate_project(&project_dir).expect("validate");
    assert!(report.valid);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.code == "REQUIRED_DIRECTORY_MISSING"));
}

#[test]
fn archives_and_restores_without_overwriting_project_data() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("可封存影片")).expect("create");
    let project_dir = temp.path().join(&project.folder_name);

    let archived = archive_project(&project_dir).expect("archive");
    assert_eq!(archived.status, ProjectStatus::Archived);
    assert_eq!(archived.archived_from_status, Some(ProjectStatus::Idea));
    assert!(!project_dir.exists());
    let archived_dir = temp.path().join("_archive").join(&project.folder_name);
    assert!(archived_dir.join("project.json").is_file());
    assert!(!temp.path().join(".ytpm-operation.json").exists());

    let restored = restore_project(&archived_dir).expect("restore");
    assert_eq!(restored.status, ProjectStatus::Idea);
    assert_eq!(restored.archived_from_status, None);
    assert!(temp
        .path()
        .join(&project.folder_name)
        .join("project.json")
        .is_file());
    assert!(!archived_dir.exists());
}

#[test]
fn archive_conflict_does_not_overwrite_existing_destination() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("衝突測試")).expect("create");
    let project_dir = temp.path().join(&project.folder_name);
    let archive_dir = temp.path().join("_archive").join(&project.folder_name);
    fs::create_dir_all(&archive_dir).expect("create conflict");
    fs::write(archive_dir.join("sentinel.txt"), "keep").expect("sentinel");

    let error = archive_project(&project_dir).expect_err("conflict should fail");
    assert!(error.to_string().contains("不會覆寫"));
    assert!(project_dir.join("project.json").is_file());
    assert_eq!(
        fs::read_to_string(archive_dir.join("sentinel.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn rejects_parent_components_for_archive_paths() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("路徑測試")).expect("create");
    let unsafe_path = temp
        .path()
        .join("safe")
        .join("..")
        .join(&project.folder_name);
    let error = archive_project(&unsafe_path).expect_err("parent component should fail");
    assert!(error.to_string().contains("不可包含 .."));
}

#[test]
fn migrates_schema_v1_project_to_current_version() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("版本遷移")).expect("create");
    let project_file = temp.path().join(&project.folder_name).join("project.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project_file).expect("read project"))
            .expect("parse project");
    value["schema_version"] = serde_json::Value::from(1);
    value
        .as_object_mut()
        .expect("object")
        .remove("archived_from_status");
    fs::write(
        &project_file,
        serde_json::to_vec_pretty(&value).expect("serialize"),
    )
    .expect("write v1 fixture");

    let project_dir = temp.path().join(&project.folder_name);
    let report = validate_project(&project_dir).expect("validate");
    assert!(report.valid);
    assert_eq!(report.project.expect("migrated project").schema_version, 2);
    let migrated = migrate_project(&project_dir).expect("migrate");
    assert_eq!(migrated.schema_version, 2);
    let migrated_json =
        fs::read_to_string(project_dir.join("project.json")).expect("read migrated");
    assert!(migrated_json.contains("\"schema_version\": 2"));
    assert!(fs::read_dir(project_dir.join(".ytpm-backup"))
        .expect("backup")
        .next()
        .is_some());
}

#[test]
fn rejects_schema_versions_that_overflow_current_version_type() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("版本上限")).expect("create");
    let project_file = temp.path().join(&project.folder_name).join("project.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project_file).expect("read project"))
            .expect("parse project");
    value["schema_version"] = serde_json::Value::from(4_294_967_297_u64);
    fs::write(
        &project_file,
        serde_json::to_vec_pretty(&value).expect("serialize"),
    )
    .expect("write future fixture");

    let report = validate_project(&temp.path().join(&project.folder_name)).expect("validate");
    assert!(!report.valid);
    assert!(report.issues[0].message.contains("不支援 schema_version"));
}
