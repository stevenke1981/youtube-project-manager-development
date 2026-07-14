#![allow(dead_code)]

#[path = "../src/error.rs"]
mod error;
#[path = "../src/folder_template.rs"]
mod folder_template;
#[path = "../src/migration.rs"]
mod migration;
#[path = "../src/model.rs"]
mod model;
#[path = "../src/project_service.rs"]
mod project_service;

use model::CreateProjectRequest;
use project_service::{create_project, list_projects, recover_operation_journal, validate_project};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_journal(
    library_root: &Path,
    operation: &str,
    source: &Path,
    destination: &Path,
    phase: &str,
) {
    let journal = json!({
        "id": "test-journal",
        "operation": operation,
        "source": source,
        "destination": destination,
        "phase": phase,
    });
    fs::write(
        library_root.join(".ytpm-operation.json"),
        serde_json::to_vec_pretty(&journal).expect("serialize journal"),
    )
    .expect("write journal");
}

fn request(title: &str) -> CreateProjectRequest {
    CreateProjectRequest {
        title: title.into(),
        channel: Some("測試頻道".into()),
        series: None,
        aspect_ratio: "16:9".into(),
        language: "zh-TW".into(),
        target_duration_seconds: None,
        planned_publish_at: None,
        tags: Vec::new(),
    }
}

#[test]
fn no_journal_is_a_noop() {
    let temp = tempdir().expect("tempdir");
    let report = recover_operation_journal(temp.path()).expect("no-op recovery");
    assert!(!report.journal_found);
    assert!(!report.journal_cleared);
    assert!(report.operation.is_none());
}

#[test]
fn prepared_journal_with_source_only_is_cleared_without_moving() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("project");
    let destination = temp.path().join("_archive/project");
    fs::create_dir_all(&source).expect("create source");
    write_journal(temp.path(), "archive", &source, &destination, "prepared");

    let report = recover_operation_journal(temp.path()).expect("recover prepared journal");
    assert!(report.journal_found);
    assert!(report.journal_cleared);
    assert_eq!(report.operation.as_deref(), Some("archive"));
    assert!(source.is_dir());
    assert!(!destination.exists());
    assert!(!temp.path().join(".ytpm-operation.json").exists());
}

#[test]
fn moved_journal_with_destination_only_is_cleared_as_completed() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("_archive/project");
    let destination = temp.path().join("project");
    fs::create_dir_all(&destination).expect("create destination");
    write_journal(temp.path(), "restore", &source, &destination, "moved");

    let report = recover_operation_journal(temp.path()).expect("recover moved journal");
    assert!(report.journal_cleared);
    assert!(destination.is_dir());
    assert!(!source.exists());
    assert!(!temp.path().join(".ytpm-operation.json").exists());
}

#[test]
fn ambiguous_states_preserve_journal_and_return_actionable_invalid_project() {
    for both_exist in [true, false] {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        if both_exist {
            fs::create_dir_all(&source).expect("create source");
            fs::create_dir_all(&destination).expect("create destination");
        }
        write_journal(temp.path(), "archive", &source, &destination, "prepared");

        let result = recover_operation_journal(temp.path());
        assert!(matches!(result, Err(error::YtpmError::InvalidProject(_))));
        assert!(temp.path().join(".ytpm-operation.json").is_file());
    }
}

#[test]
fn journal_paths_must_be_inside_library_root() {
    let temp = tempdir().expect("tempdir");
    let library = temp.path().join("中文 Library");
    fs::create_dir_all(&library).expect("create library");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).expect("create outside");
    let destination = library.join("project");
    write_journal(&library, "archive", &outside, &destination, "prepared");

    let error = recover_operation_journal(&library).expect_err("outside path must fail");
    assert!(matches!(&error, error::YtpmError::InvalidProject(_)));
    assert!(library.join(".ytpm-operation.json").is_file());
    assert!(error.to_string().contains("人工"));
}

#[test]
fn unsupported_operation_preserves_journal() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    fs::create_dir_all(&source).expect("create source");
    write_journal(temp.path(), "delete", &source, &destination, "prepared");

    let result = recover_operation_journal(temp.path());
    assert!(matches!(result, Err(error::YtpmError::InvalidProject(_))));
    assert!(temp.path().join(".ytpm-operation.json").is_file());
}

#[test]
fn list_projects_recovers_before_scanning() {
    let temp = tempdir().expect("tempdir");
    let project = create_project(temp.path(), request("啟動恢復測試")).expect("create project");
    let source = temp.path().join(&project.folder_name);
    let destination = temp.path().join("_archive").join(&project.folder_name);
    write_journal(temp.path(), "archive", &source, &destination, "prepared");

    let projects = list_projects(temp.path()).expect("list projects with recovery");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project.id);
    assert!(!temp.path().join(".ytpm-operation.json").exists());
}

#[cfg(windows)]
#[test]
fn validation_rejects_required_directory_junction() {
    use std::process::Command;

    let temp = tempdir().expect("tempdir");
    let library = temp.path().join("中文 Library");
    fs::create_dir_all(&library).expect("create library");
    let project = create_project(&library, request("junction fixture")).expect("create project");
    let project_dir = library.join(&project.folder_name);
    let required = project_dir.join("06_subtitles/translations");
    let target = temp.path().join("中文 junction target");
    fs::create_dir_all(&target).expect("create junction target");
    fs::remove_dir(&required).expect("remove required directory");

    let output = Command::new("cmd.exe")
        .args([
            "/C",
            "mklink",
            "/J",
            required.to_str().expect("junction path"),
            target.to_str().expect("target path"),
        ])
        .output()
        .expect("run mklink");
    assert!(
        output.status.success(),
        "mklink failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = validate_project(&project_dir).expect("validate junction fixture");
    assert!(!report.valid);
    assert!(report.issues.iter().any(|issue| {
        issue.code == "REQUIRED_DIRECTORY_SYMLINK"
            && matches!(issue.severity, model::ValidationSeverity::Error)
    }));
}

#[cfg(not(windows))]
#[test]
fn junction_validation_fixture_is_skipped_on_non_windows() {
    eprintln!("SKIP: mklink /J junction validation fixture requires Windows");
}
