use std::fs;
use std::process::Command;
use tempfile::tempdir;
use ytpm_core::{probe_media, Clip, Timeline};

#[test]
fn probe_rejects_path_traversal_before_starting_external_tool() {
    let project = tempdir().unwrap();
    let error = probe_media(project.path(), "..\\secret.mp4").unwrap_err();
    assert!(error.to_string().contains("不安全相對路徑"));
}

#[test]
fn ffprobe_reads_a_real_generated_wav_when_available() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return;
    }
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("03_voice")).unwrap();
    let input = project.path().join("03_voice/test.wav");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.2",
            "-y",
        ])
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());
    let probe = probe_media(project.path(), "03_voice/test.wav").unwrap();
    assert_eq!(probe.relative_path, "03_voice/test.wav");
    assert!(probe.duration_seconds.unwrap_or_default() > 0.0);
    assert!(probe
        .streams
        .iter()
        .any(|stream| stream.codec_type.as_deref() == Some("audio")));
}

#[test]
fn empty_timeline_has_no_renderable_video() {
    let timeline = Timeline::default();
    assert!(ytpm_core::media_service::export_timeline(
        tempfile::tempdir().unwrap().path(),
        &timeline,
        "09_exports/out.mp4",
        None,
    )
    .is_err());
}

#[test]
fn ffmpeg_renders_a_real_trimmed_timeline_when_available() {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return;
    }
    let project = tempdir().unwrap();
    fs::create_dir_all(project.path().join("05_video")).unwrap();
    fs::create_dir_all(project.path().join("09_exports")).unwrap();
    let input = project.path().join("05_video/source.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=320x240:d=0.4",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.4",
            "-shortest",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-pix_fmt",
            "yuv420p",
            "-y",
        ])
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());
    let mut timeline = Timeline {
        duration_ms: 200,
        ..Default::default()
    };
    timeline.tracks[0].clips.push(Clip {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: uuid::Uuid::new_v4().to_string(),
        relative_path: "05_video/source.mp4".into(),
        label: "source".into(),
        start_ms: 0,
        in_ms: 0,
        out_ms: 200,
        duration_ms: 200,
        volume: 1.0,
        muted: false,
        transition: None,
        effects: Vec::new(),
    });
    timeline.tracks[1].clips.push(Clip {
        id: uuid::Uuid::new_v4().to_string(),
        asset_id: uuid::Uuid::new_v4().to_string(),
        relative_path: "05_video/source.mp4".into(),
        label: "audio".into(),
        start_ms: 0,
        in_ms: 0,
        out_ms: 200,
        duration_ms: 200,
        volume: 0.5,
        muted: false,
        transition: None,
        effects: Vec::new(),
    });
    let result =
        ytpm_core::export_timeline(project.path(), &timeline, "09_exports/final.mp4", None)
            .unwrap();
    assert_eq!(result.progress, 100);
    assert!(project.path().join("09_exports/final.mp4").is_file());
    let output_probe = probe_media(project.path(), "09_exports/final.mp4").unwrap();
    assert!(output_probe
        .streams
        .iter()
        .any(|stream| stream.codec_type.as_deref() == Some("audio")));
}
