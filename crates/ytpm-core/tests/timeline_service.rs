use std::fs;
use std::sync::{Arc, Barrier};
use tempfile::tempdir;
use ytpm_core::timeline_service::{validate, ClipEffect, CURRENT_TIMELINE_SCHEMA_VERSION};
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
            effects: Vec::new(),
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
        effects: Vec::new(),
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

#[test]
fn read_timeline_migrates_v1_atomically_after_exact_backup() {
    let project = tempdir().expect("temp project");
    let v1 = r#"{
  "schema_version": 1,
  "duration_ms": 1000,
  "tracks": [{
    "id": "00000000-0000-0000-0000-000000000001",
    "label": "V1",
    "kind": "video",
    "clips": [{
      "id": "10000000-0000-0000-0000-000000000001",
      "asset_id": "20000000-0000-0000-0000-000000000001",
      "relative_path": "05_video/source.mp4",
      "label": "source",
      "start_ms": 0,
      "in_ms": 0,
      "out_ms": 1000,
      "duration_ms": 1000,
      "volume": 1.0,
      "muted": false,
      "transition": null
    }]
  }],
  "output": {
    "output_relative_path": "09_exports/timeline.mp4",
    "format": "mp4",
    "width": 1920,
    "height": 1080,
    "frame_rate": 30
  },
  "updated_at": "1970-01-01T00:00:00Z"
}"#;
    fs::write(project.path().join("timeline.json"), v1).unwrap();

    let migrated = read_timeline(project.path()).expect("migrate timeline");
    assert_eq!(migrated.schema_version, CURRENT_TIMELINE_SCHEMA_VERSION);
    assert!(migrated.tracks[0].clips[0].effects.is_empty());
    assert_eq!(migrated.output.subtitle_style.font_size, 48);

    let backups: Vec<_> = fs::read_dir(project.path().join(".ytpm-backup"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), v1);
    assert!(backups[0]
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("timeline-v1-"));

    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(project.path().join("timeline.json")).unwrap())
            .unwrap();
    assert_eq!(persisted["schema_version"], 2);
    assert_eq!(
        persisted["tracks"][0]["clips"][0]["effects"],
        serde_json::json!([])
    );
    assert!(persisted["output"]["subtitle_style"].is_object());
    read_timeline(project.path()).expect("v2 reread");
    assert_eq!(
        fs::read_dir(project.path().join(".ytpm-backup"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn timeline_v2_accepts_typed_effects_and_rejects_unsafe_parameters() {
    let project = tempdir().expect("temp project");
    let mut timeline = read_timeline(project.path()).unwrap();
    timeline.duration_ms = 2_000;
    timeline.tracks[0].clips.push(TimelineClip {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: uuid::Uuid::new_v4().to_string(),
        relative_path: "05_video/effects.mp4".into(),
        label: "effects".into(),
        start_ms: 0,
        in_ms: 0,
        out_ms: 2_000,
        duration_ms: 2_000,
        volume: 1.0,
        muted: false,
        transition: None,
        effects: vec![
            ClipEffect::ColorAdjust {
                brightness: 0.1,
                contrast: 1.2,
                saturation: 0.9,
                gamma: 1.0,
            },
            ClipEffect::Blur { radius: 2.0 },
            ClipEffect::Sharpen { amount: 1.0 },
            ClipEffect::Vignette { angle: 0.5 },
            ClipEffect::ChromaKey {
                color: "#00FF00".into(),
                similarity: 0.2,
                blend: 0.1,
            },
            ClipEffect::FadeIn { duration_ms: 250 },
            ClipEffect::FadeOut { duration_ms: 250 },
            ClipEffect::Transform {
                x: 10.0,
                y: -5.0,
                scale: 1.25,
                rotation_degrees: 2.0,
                opacity: 0.9,
            },
        ],
    });
    validate(&timeline).expect("typed effects are valid");
    let persisted = write_timeline(project.path(), &timeline).unwrap();
    assert_eq!(persisted.tracks[0].clips[0].effects.len(), 8);

    timeline.tracks[0].clips[0].effects = vec![ClipEffect::ChromaKey {
        color: "green;movie=outside.mp4".into(),
        similarity: 0.2,
        blend: 0.1,
    }];
    assert!(validate(&timeline)
        .unwrap_err()
        .to_string()
        .contains("#RRGGBB"));

    timeline.tracks[0].clips[0].effects = vec![ClipEffect::FadeOut { duration_ms: 2_001 }];
    assert!(validate(&timeline)
        .unwrap_err()
        .to_string()
        .contains("fade duration_ms"));
}

#[test]
fn schema_and_project_template_are_timeline_v2() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("schemas/timeline.schema.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], 2);
    assert!(schema["$defs"]["effect"]["oneOf"].is_array());
    assert_eq!(
        schema["$defs"]["output"]["properties"]["format"]["const"],
        "mp4"
    );
    assert!(schema["$defs"]["track"]["allOf"].is_array());

    let template: ytpm_core::Timeline = serde_json::from_str(
        &fs::read_to_string(root.join("templates/project/timeline.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(template.schema_version, CURRENT_TIMELINE_SCHEMA_VERSION);
    assert_eq!(template.output.output_relative_path, "09_exports/final.mp4");
    assert_eq!(template.output.format, "mp4");
    validate(&template).unwrap();
}

#[test]
fn timeline_rejects_non_mp4_outputs_and_effects_on_non_video_tracks() {
    let mut timeline = ytpm_core::Timeline {
        duration_ms: 1_000,
        ..Default::default()
    };
    timeline.output.format = "webm".into();
    assert!(validate(&timeline).unwrap_err().to_string().contains("mp4"));

    timeline.output.format = "mp4".into();
    timeline.output.output_relative_path = "09_exports/final.webm".into();
    assert!(validate(&timeline)
        .unwrap_err()
        .to_string()
        .contains(".mp4"));

    timeline.output.output_relative_path = "09_exports/final.mp4".into();
    timeline.tracks[1].clips.push(TimelineClip {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: uuid::Uuid::new_v4().to_string(),
        relative_path: "03_voice/audio.wav".into(),
        label: "audio".into(),
        start_ms: 0,
        in_ms: 0,
        out_ms: 1_000,
        duration_ms: 1_000,
        volume: 1.0,
        muted: false,
        transition: None,
        effects: vec![ClipEffect::Blur { radius: 1.0 }],
    });
    assert!(validate(&timeline)
        .unwrap_err()
        .to_string()
        .contains("video track"));
}

#[test]
fn concurrent_clip_additions_are_serialized_per_project() {
    let project = tempdir().unwrap();
    let project_path = Arc::new(project.path().to_path_buf());
    let barrier = Arc::new(Barrier::new(9));
    let mut workers = Vec::new();
    for index in 0..8u64 {
        let project_path = Arc::clone(&project_path);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            add_clip(
                &project_path,
                VIDEO_TRACK,
                TimelineClipRequest {
                    asset_id: uuid::Uuid::new_v4().to_string(),
                    relative_path: format!("05_video/source-{index}.mp4"),
                    label: format!("clip-{index}"),
                    start_ms: index * 100,
                    in_ms: 0,
                    out_ms: 50,
                    volume: 1.0,
                    muted: false,
                    transition: None,
                    effects: Vec::new(),
                },
            )
            .unwrap();
        }));
    }
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(
        read_timeline(&project_path).unwrap().tracks[0].clips.len(),
        8
    );
}
