#[path = "../src/error.rs"]
mod error;

pub use error::{Result, YtpmError};

#[path = "../src/asset_service.rs"]
mod asset_service;

use asset_service::{list_assets, scan_assets, AssetKind, AssetState};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write_file(project_dir: &Path, relative_path: &str, contents: &[u8]) {
    let path = project_dir.join(relative_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    fs::create_dir_all(path.parent().expect("file parent")).expect("create parent");
    fs::write(path, contents).expect("write file");
}

fn write_catalog_record(project_dir: &Path, relative_path: &str) {
    let catalog = serde_json::json!({
        "schema_version": 1,
        "assets": [{
            "id": "asset-1",
            "kind": "other",
            "relative_path": relative_path,
            "display_name": "asset",
            "state": "available",
            "source_type": null,
            "size_bytes": null,
            "sha256": null,
            "duration_ms": null,
            "width": null,
            "height": null,
            "generator": null,
            "model": null,
            "prompt": null,
            "version_group_id": null,
            "version_number": 1,
            "is_adopted": false,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }]
    });
    fs::write(
        project_dir.join("assets.json"),
        serde_json::to_vec_pretty(&catalog).expect("serialize catalog"),
    )
    .expect("write catalog");
}

#[test]
fn scans_chinese_filename() {
    let temp = tempdir().expect("tempdir");
    write_file(
        temp.path(),
        "01_research/研究筆記.md",
        "中文內容".as_bytes(),
    );

    let catalog = scan_assets(temp.path()).expect("scan");

    assert_eq!(catalog.assets.len(), 1);
    assert_eq!(catalog.assets[0].relative_path, "01_research/研究筆記.md");
    assert_eq!(
        catalog.assets[0].display_name.as_deref(),
        Some("研究筆記.md")
    );
}

#[test]
fn maps_kinds_from_relative_paths() {
    let temp = tempdir().expect("tempdir");
    for relative_path in [
        "01_research/a.md",
        "02_script/b.md",
        "03_voice/raw/c.wav",
        "03_voice/music/d.mp3",
        "03_voice/sound_effects/e.wav",
        "04_images/f.png",
        "05_video/g.mp4",
        "06_subtitles/h.srt",
        "07_thumbnail/i.png",
        "08_metadata/j.txt",
        "09_exports/k.mp4",
        "unclassified/l.bin",
    ] {
        write_file(temp.path(), relative_path, b"x");
    }

    let catalog = scan_assets(temp.path()).expect("scan");
    let kind = |relative_path: &str| {
        catalog
            .assets
            .iter()
            .find(|asset| asset.relative_path == relative_path)
            .expect("asset")
            .kind
            .clone()
    };

    assert_eq!(kind("01_research/a.md"), AssetKind::Research);
    assert_eq!(kind("02_script/b.md"), AssetKind::Script);
    assert_eq!(kind("03_voice/raw/c.wav"), AssetKind::Voice);
    assert_eq!(kind("03_voice/music/d.mp3"), AssetKind::Music);
    assert_eq!(kind("03_voice/sound_effects/e.wav"), AssetKind::SoundEffect);
    assert_eq!(kind("04_images/f.png"), AssetKind::Image);
    assert_eq!(kind("05_video/g.mp4"), AssetKind::Video);
    assert_eq!(kind("06_subtitles/h.srt"), AssetKind::Subtitle);
    assert_eq!(kind("07_thumbnail/i.png"), AssetKind::Thumbnail);
    assert_eq!(kind("08_metadata/j.txt"), AssetKind::Metadata);
    assert_eq!(kind("09_exports/k.mp4"), AssetKind::Export);
    assert_eq!(kind("unclassified/l.bin"), AssetKind::Other);
}

#[test]
fn records_streaming_hash_and_size() {
    let temp = tempdir().expect("tempdir");
    let contents = b"hello asset catalog";
    write_file(temp.path(), "05_video/sample.bin", contents);

    let catalog = scan_assets(temp.path()).expect("scan");
    let asset = &catalog.assets[0];

    assert_eq!(asset.size_bytes, Some(contents.len() as u64));
    assert_eq!(
        asset.sha256.as_deref(),
        Some("4743434ad2d27d93d2166d1c4cba2fb1fcd8d189a9ceb98c86e026335a336cd5")
    );
    assert_eq!(asset.source_type.as_deref(), Some("imported"));
}

#[test]
fn excludes_project_metadata_and_internal_directories() {
    let temp = tempdir().expect("tempdir");
    for relative_path in [
        "project.json",
        "tasks.json",
        "README.md",
        "activity.log",
        ".ytpm/state.json",
        ".ytpm-backup/project.json.bak",
        "01_research/kept.md",
    ] {
        write_file(temp.path(), relative_path, b"metadata");
    }
    fs::write(
        temp.path().join("assets.json"),
        br#"{"schema_version":1,"assets":[]}"#,
    )
    .expect("write empty catalog");

    let catalog = scan_assets(temp.path()).expect("scan");

    assert_eq!(catalog.assets.len(), 1);
    assert_eq!(catalog.assets[0].relative_path, "01_research/kept.md");
}

#[test]
fn preserves_missing_record_without_deleting_it() {
    let temp = tempdir().expect("tempdir");
    write_file(temp.path(), "02_script/script.md", b"script");
    let first = scan_assets(temp.path()).expect("initial scan");
    let id = first.assets[0].id.clone();
    fs::remove_file(temp.path().join("02_script/script.md")).expect("remove asset");

    let listed = list_assets(temp.path()).expect("list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].relative_path, "02_script/script.md");
    assert_eq!(listed[0].state, AssetState::Missing);
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(temp.path().join("assets.json")).expect("read"))
            .expect("json");
    assert_eq!(persisted["assets"][0]["state"], "missing");
}

#[test]
fn scan_writes_portable_catalog_atomically() {
    let temp = tempdir().expect("tempdir");
    write_file(temp.path(), "04_images/cover.png", b"image");

    let catalog = scan_assets(temp.path()).expect("scan");
    let catalog_path = temp.path().join("assets.json");
    let persisted: asset_service::AssetCatalog =
        serde_json::from_str(&fs::read_to_string(&catalog_path).expect("read catalog"))
            .expect("parse catalog");

    assert_eq!(persisted.schema_version, 1);
    assert_eq!(persisted.assets.len(), catalog.assets.len());
    assert!(!fs::read_dir(temp.path())
        .expect("read project")
        .filter_map(|entry| entry.ok())
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(".assets.json.")));
}

#[test]
fn lists_assets_by_scanning_when_catalog_is_absent() {
    let temp = tempdir().expect("tempdir");
    write_file(
        temp.path(),
        "06_subtitles/translations/繁中.srt",
        b"subtitle",
    );

    let assets = list_assets(temp.path()).expect("list");

    assert_eq!(assets.len(), 1);
    assert!(temp.path().join("assets.json").is_file());
}

#[test]
fn rejects_unsafe_catalog_relative_path() {
    let temp = tempdir().expect("tempdir");
    write_catalog_record(temp.path(), "../outside.txt");

    assert!(list_assets(temp.path()).is_err());
}

#[test]
fn rejects_windows_reserved_filename_in_catalog() {
    let temp = tempdir().expect("tempdir");
    write_catalog_record(temp.path(), "01_research/CON.txt");

    assert!(list_assets(temp.path()).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_entries() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().expect("tempdir");
    let outside = tempdir().expect("outside tempdir");
    write_file(outside.path(), "outside.txt", b"outside");
    fs::create_dir_all(temp.path().join("01_research")).expect("create directory");
    symlink(
        outside.path().join("outside.txt"),
        temp.path().join("01_research/link.txt"),
    )
    .expect("symlink");

    assert!(scan_assets(temp.path()).is_err());
}

#[cfg(windows)]
#[test]
fn rejects_reparse_entries_on_windows() {
    use std::os::windows::fs::symlink_file;

    let temp = tempdir().expect("tempdir");
    let outside = tempdir().expect("outside tempdir");
    write_file(outside.path(), "outside.txt", b"outside");
    fs::create_dir_all(temp.path().join("01_research")).expect("create directory");
    symlink_file(
        outside.path().join("outside.txt"),
        temp.path().join("01_research/link.txt"),
    )
    .expect("symlink");

    assert!(scan_assets(temp.path()).is_err());
}
