use std::fs;
use tempfile::tempdir;
use ytpm_core::{
    add_clip, read_timeline, remove_clip, render_manifest, update_clip, validate_timeline,
    write_timeline, TimelineClip, TimelineClipPatch, TimelineClipRequest, TimelineIssueSeverity,
    TimelineTrackKind,
};

const VIDEO_TRACK: &str = "00000000-0000-0000-0000-000000000001";
const AUDIO_TRACK: &str = "00000000-0000-0000-0000-000000000002";

#[test]
fn timeline_is_atomic_and_supports_non_destructive_trim_and_move() {
    let project = tempdir().expect("temp project");
    let mut timeline = read_timeline(project.path()).expect("default timeline");
    timeline.duration_ms = 20_000;
    write_timeline(project.path(), &timeline).expect("write timeline");

    let clip = add_clip(
        project.path(),
        VIDEO_TRACK,
        TimelineClipRequest {
            asset_id: uuid::Uuid::new_v4().to_string(),
            relative_path: "05_video/source.mp4".into(),
            label: "Source".into(),
            start_ms: 0,
            in_ms: 2_000,
            out_ms: 12_000,
            volume: 1.0,
            muted: false,
            transition: None,
        },
    )
    .expect("add clip");
    let moved = update_clip(
        project.path(),
        &clip.id,
        TimelineClipPatch {
            track_id: Some(AUDIO_TRACK.into()),
            start_ms: Some(1_000),
            in_ms: Some(3_000),
            out_ms: Some(10_000),
            label: Some("Trimmed".into()),
            ..Default::default()
        },
    )
    .expect("move and trim clip");
    assert_eq!(moved.label, "Trimmed");
    assert_eq!(moved.start_ms, 1_000);
    assert_eq!(
        read_timeline(project.path()).unwrap().tracks[1].clips.len(),
        1
    );
    assert_eq!(
        render_manifest(&read_timeline(project.path()).unwrap())
            .unwrap()
            .inputs
            .len(),
        1
    );

    remove_clip(project.path(), &clip.id).expect("remove clip reference");
    assert!(read_timeline(project.path())
        .unwrap()
        .tracks
        .iter()
        .all(|track| track.clips.is_empty()));
    assert!(fs::read_dir(project.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .contains(".timeline.")));
}

#[test]
fn timeline_rejects_unsafe_paths_and_overlapping_clips() {
    let project = tempdir().expect("temp project");
    let mut timeline = read_timeline(project.path()).unwrap();
    timeline.duration_ms = 5_000;
    let clip = |id: String, path: &str, start_ms: u64, duration_ms: u64| TimelineClip {
        id,
        asset_id: uuid::Uuid::new_v4().to_string(),
        relative_path: path.into(),
        label: "clip".into(),
        start_ms,
        in_ms: 0,
        out_ms: duration_ms,
        duration_ms,
        volume: 1.0,
        muted: false,
        transition: None,
    };
    timeline.tracks[0].clips.push(clip(
        uuid::Uuid::new_v4().to_string(),
        "../outside.mp4",
        0,
        3_000,
    ));
    timeline.tracks[0].clips.push(clip(
        uuid::Uuid::new_v4().to_string(),
        "05_video/ok.mp4",
        2_000,
        2_000,
    ));
    let report = validate_timeline(&timeline);
    assert!(!report.valid);
    assert_eq!(report.issues.len(), 1);
    assert_eq!(report.issues[0].severity, TimelineIssueSeverity::Error);
    assert_eq!(timeline.tracks[0].kind, TimelineTrackKind::Video);
}
