use std::fs;
use tempfile::tempdir;
use ytpm_core::publish_service::{
    config_reference, dry_run, load_metadata, save_metadata, start_oauth, PublishMetadata,
    PublishVisibility,
};

#[test]
fn metadata_is_atomic_and_dry_run_never_requires_network_credentials() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("09_exports")).unwrap();
    fs::write(
        project.path().join("09_exports/final.mp4"),
        b"not a real video",
    )
    .unwrap();
    let metadata = PublishMetadata {
        title: "測試影片".into(),
        description: "離線 dry-run".into(),
        tags: vec!["ytpm".into()],
        visibility: PublishVisibility::Private,
        scheduled_at: None,
        channel: None,
    };
    save_metadata(project.path(), &metadata).unwrap();
    assert_eq!(load_metadata(project.path()).unwrap(), metadata);
    let result = dry_run(project.path(), &metadata).unwrap();
    assert!(result.dry_run);
    assert!(!result.uploaded);
    assert!(matches!(
        result.status,
        ytpm_core::publish_service::PublishJobStatus::Completed
    ));
}

#[test]
fn oauth_start_uses_pkce_and_redacts_credentials_from_config_reference() {
    std::env::set_var("YTPM_YOUTUBE_CLIENT_ID", "test-client-id");
    std::env::remove_var("YTPM_YOUTUBE_CLIENT_SECRET");
    std::env::remove_var("YTPM_YOUTUBE_REFRESH_TOKEN");
    let oauth = start_oauth().unwrap();
    assert!(oauth.authorize_url.contains("code_challenge="));
    assert!(oauth.authorize_url.contains("state="));
    assert!(!config_reference().oauth_ready);
    assert!(!config_reference().config_path.contains("token"));
    std::env::remove_var("YTPM_YOUTUBE_CLIENT_ID");
}

#[test]
fn scheduled_publish_requires_private_visibility() {
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("09_exports")).unwrap();
    fs::write(project.path().join("09_exports/final.mp4"), b"video").unwrap();
    let metadata = PublishMetadata {
        title: "排程測試".into(),
        description: String::new(),
        tags: Vec::new(),
        visibility: PublishVisibility::Public,
        scheduled_at: Some("2099-01-01T00:00:00Z".into()),
        channel: None,
    };
    let readiness = ytpm_core::publish_service::validate_metadata(&metadata, project.path());
    let schedule = readiness.checks.iter().find(|check| check.id == "schedule");
    assert!(schedule.is_some_and(|check| !check.ok));
}
