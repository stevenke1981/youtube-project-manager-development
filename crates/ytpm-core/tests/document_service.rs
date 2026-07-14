#[path = "../src/document_service.rs"]
mod document_service;
#[path = "../src/error.rs"]
mod error;

use document_service::{read_document, write_document};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn project_fixture() -> tempfile::TempDir {
    let temp = tempdir().expect("tempdir");
    for directory in ["02_script", "08_metadata", "06_subtitles/translations"] {
        fs::create_dir_all(temp.path().join(directory)).expect("create fixture directory");
    }
    temp
}

#[test]
fn reads_and_writes_all_allowed_document_families() {
    let temp = project_fixture();
    let documents = [
        ("02_script/script.md", "# 腳本\n\n中文內容\n"),
        ("08_metadata/title.md", "標題候選\n"),
        ("08_metadata/description.md", "影片描述\n"),
        ("08_metadata/pinned-comment.md", "置頂留言\n"),
        ("08_metadata/chapters.txt", "00:00 開始\n"),
        ("08_metadata/tags.txt", "Rust, YouTube\n"),
        (
            "06_subtitles/translations/中文.srt",
            "1\n00:00:00,000 --> 00:00:01,000\n你好\n",
        ),
        (
            "06_subtitles/translations/english.vtt",
            "WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n",
        ),
        (
            "06_subtitles/translations/voice.ass",
            "[Script Info]\nTitle: test\n",
        ),
    ];

    for (relative, content) in documents {
        let relative = Path::new(relative);
        write_document(temp.path(), relative, content).expect("write allowed document");
        assert_eq!(
            read_document(temp.path(), relative).expect("read allowed document"),
            content
        );
    }
}

#[test]
fn rejects_absolute_parent_and_non_allowlisted_paths() {
    let temp = project_fixture();
    let mut rejected = vec![
        Path::new("../02_script/script.md"),
        Path::new("02_script/outline.md"),
        Path::new("08_metadata/README.md"),
        Path::new("06_subtitles/translations/nested/captions.srt"),
        Path::new("06_subtitles/translations/captions.txt"),
    ];
    #[cfg(windows)]
    rejected.push(Path::new(r"C:\outside\script.md"));
    #[cfg(not(windows))]
    rejected.push(Path::new("/outside/script.md"));

    for relative in rejected {
        assert!(
            write_document(temp.path(), relative, "must fail").is_err(),
            "path should be rejected: {}",
            relative.display()
        );
    }
}

#[test]
fn enforces_the_four_mib_limit_and_keeps_original_content() {
    let temp = project_fixture();
    let relative = Path::new("02_script/script.md");
    write_document(temp.path(), relative, "原始內容").expect("write original");
    let original = read_document(temp.path(), relative).expect("read original");

    let oversized = "x".repeat(4 * 1024 * 1024 + 1);
    assert!(write_document(temp.path(), relative, &oversized).is_err());
    assert_eq!(
        read_document(temp.path(), relative).expect("read preserved content"),
        original
    );

    fs::write(temp.path().join(relative), oversized).expect("write oversized fixture");
    assert!(read_document(temp.path(), relative).is_err());
}

#[test]
fn failed_replacement_does_not_remove_existing_target_entry() {
    let temp = project_fixture();
    let relative = Path::new("02_script/script.md");
    let target = temp.path().join(relative);
    fs::create_dir(&target).expect("create directory target");

    assert!(write_document(temp.path(), relative, "new content").is_err());
    assert!(
        target.is_dir(),
        "failed replacement must leave the target entry"
    );
    assert!(!fs::read_dir(target.parent().expect("target parent"))
        .expect("read parent")
        .any(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".script.md.")
        }));
}

#[cfg(not(windows))]
#[test]
fn rejects_a_symlinked_document_parent() {
    use std::os::unix::fs::symlink;

    let temp = project_fixture();
    let real_script = temp.path().join("real-script");
    fs::create_dir_all(real_script.join("02_script")).expect("create real parent");
    fs::write(real_script.join("02_script/script.md"), "outside").expect("write real file");
    fs::remove_dir(temp.path().join("02_script")).expect("remove original parent");
    symlink(real_script.join("02_script"), temp.path().join("02_script")).expect("symlink parent");

    assert!(read_document(temp.path(), Path::new("02_script/script.md")).is_err());
    assert!(write_document(temp.path(), Path::new("02_script/script.md"), "blocked").is_err());
}
