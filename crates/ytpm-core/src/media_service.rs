//! FFprobe/FFmpeg adapters.
//!
//! External tools are always started with `Command` argument vectors. No shell
//! command string is accepted, and all project paths are validated as relative
//! paths before they reach the process boundary.

use crate::{
    render_manifest, Clip, RenderSettings, Result, Timeline, TimelineTrackKind, YtpmError,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaStream {
    pub index: Option<u32>,
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub r_frame_rate: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaProbe {
    pub relative_path: String,
    pub format_name: Option<String>,
    pub duration_seconds: Option<f64>,
    pub size_bytes: Option<u64>,
    pub bitrate_bps: Option<u64>,
    pub streams: Vec<MediaStream>,
    pub probed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaJobStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaExportResult {
    pub operation_id: String,
    pub status: MediaJobStatus,
    pub progress: u8,
    pub output_relative_path: Option<String>,
    pub message: Option<String>,
}

pub fn probe_media(project_dir: &Path, relative_path: &str) -> Result<MediaProbe> {
    let input = resolve_project_file(project_dir, relative_path)?;
    let ffprobe = tool_from_env("YTPM_FFPROBE_PATH", "ffprobe");
    let output = Command::new(&ffprobe)
        .args([
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-print_format"),
            OsString::from("json"),
            OsString::from("-show_format"),
            OsString::from("-show_streams"),
            input.as_os_str().to_os_string(),
        ])
        .output()
        .map_err(|error| tool_error("ffprobe", &ffprobe, error))?;
    if !output.status.success() {
        return Err(YtpmError::InvalidInput(format!(
            "FFprobe 無法讀取 {}：{}",
            relative_path,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let document: Value = serde_json::from_slice(&output.stdout)?;
    let format = document.get("format").cloned().unwrap_or(Value::Null);
    let streams = document
        .get("streams")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(parse_stream).collect())
        .unwrap_or_default();
    Ok(MediaProbe {
        relative_path: relative_path.to_owned(),
        format_name: string_value(format.get("format_name")),
        duration_seconds: number_value(format.get("duration")),
        size_bytes: integer_value(format.get("size")),
        bitrate_bps: integer_value(format.get("bit_rate")),
        streams,
        probed_at: Utc::now().to_rfc3339(),
    })
}

pub fn export_timeline(
    project_dir: &Path,
    timeline: &Timeline,
    output_relative_path: &str,
    cancel_marker: Option<&Path>,
) -> Result<MediaExportResult> {
    let operation_id = Uuid::new_v4().to_string();
    let output = resolve_project_output(project_dir, output_relative_path)?;
    reject_reparse_points(&output)?;
    if output.exists() {
        return Err(YtpmError::InvalidInput(format!(
            "拒絕覆寫既有匯出檔：{}",
            output_relative_path
        )));
    }
    let _manifest = render_manifest(timeline)?;
    let clips: Vec<_> = timeline
        .tracks
        .iter()
        .filter(|track| track.kind == TimelineTrackKind::Video)
        .flat_map(|track| track.clips.iter())
        .collect();
    let audio_clips: Vec<_> = timeline
        .tracks
        .iter()
        .filter(|track| track.kind == TimelineTrackKind::Audio)
        .flat_map(|track| track.clips.iter())
        .collect();
    if clips.is_empty() {
        return Err(YtpmError::InvalidInput(
            "timeline 沒有可匯出的 video clip".into(),
        ));
    }
    if is_cancelled(cancel_marker) {
        return Ok(cancelled_result(operation_id));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| YtpmError::io(parent, error))?;
        reject_reparse_points(parent)?;
    }
    let work_dir = project_dir.join(format!(".ytpm-media-{operation_id}"));
    fs::create_dir_all(&work_dir).map_err(|error| YtpmError::io(&work_dir, error))?;
    let result = export_segments(
        project_dir,
        &work_dir,
        &output,
        &clips,
        &audio_clips,
        &timeline.output,
        cancel_marker,
    );
    let cleanup = fs::remove_dir_all(&work_dir);
    if let Err(error) = cleanup {
        if result.is_ok() {
            return Err(YtpmError::io(&work_dir, error));
        }
    }
    result.map(|status| match status {
        MediaJobStatus::Completed => MediaExportResult {
            operation_id,
            status,
            progress: 100,
            output_relative_path: Some(output_relative_path.to_owned()),
            message: Some(if audio_clips.is_empty() {
                "FFmpeg 匯出完成。".into()
            } else {
                "FFmpeg 匯出完成，已混合 audio track。".into()
            }),
        },
        MediaJobStatus::Cancelled => cancelled_result(operation_id),
        MediaJobStatus::Failed => MediaExportResult {
            operation_id,
            status,
            progress: 0,
            output_relative_path: None,
            message: Some("FFmpeg 匯出失敗。".into()),
        },
    })
}

pub fn cancel_marker(project_dir: &Path, operation_id: &str) -> Result<PathBuf> {
    validate_identifier(operation_id)?;
    reject_reparse_points(project_dir)?;
    let directory = project_dir.join(".ytpm").join("cancel");
    fs::create_dir_all(&directory).map_err(|error| YtpmError::io(&directory, error))?;
    reject_reparse_points(&directory)?;
    let marker = directory.join(format!("{operation_id}.cancel"));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
        .map_err(|error| YtpmError::io(&marker, error))?;
    Ok(marker)
}

fn export_segments(
    project_dir: &Path,
    work_dir: &Path,
    output: &Path,
    clips: &[&Clip],
    audio_clips: &[&Clip],
    settings: &RenderSettings,
    cancel_marker: Option<&Path>,
) -> Result<MediaJobStatus> {
    let ffmpeg = tool_from_env("YTPM_FFMPEG_PATH", "ffmpeg");
    let dedicated_audio = !audio_clips.is_empty();
    let mut segments = Vec::with_capacity(clips.len());
    for (index, clip) in clips.iter().enumerate() {
        if is_cancelled(cancel_marker) {
            return Ok(MediaJobStatus::Cancelled);
        }
        let input = resolve_project_file(project_dir, &clip.relative_path)?;
        let segment = work_dir.join(format!("segment-{index:04}.mp4"));
        let duration = clip.out_ms - clip.in_ms;
        let mut arguments = vec![
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-y"),
            OsString::from("-ss"),
            OsString::from(format_seconds(clip.in_ms)),
            OsString::from("-i"),
            input.as_os_str().to_os_string(),
            OsString::from("-t"),
            OsString::from(format_seconds(duration)),
            OsString::from("-map"),
            OsString::from("0:v:0?"),
            OsString::from("-vf"),
            OsString::from(format!("scale={}:{}", settings.width, settings.height)),
            OsString::from("-r"),
            OsString::from(format!("{}", settings.frame_rate)),
            OsString::from("-c:v"),
            OsString::from("libx264"),
        ];
        if dedicated_audio {
            arguments.push(OsString::from("-an"));
        } else {
            arguments.extend([
                OsString::from("-map"),
                OsString::from("0:a:0?"),
                OsString::from("-c:a"),
                OsString::from("aac"),
            ]);
        }
        arguments.extend([
            OsString::from("-movflags"),
            OsString::from("+faststart"),
            segment.as_os_str().to_os_string(),
        ]);
        let output_result = Command::new(&ffmpeg)
            .args(arguments)
            .output()
            .map_err(|error| tool_error("ffmpeg", &ffmpeg, error))?;
        if !output_result.status.success() {
            return Err(YtpmError::InvalidInput(format!(
                "FFmpeg 裁切 {} 失敗：{}",
                clip.relative_path,
                String::from_utf8_lossy(&output_result.stderr).trim()
            )));
        }
        segments.push(segment);
    }
    if is_cancelled(cancel_marker) {
        return Ok(MediaJobStatus::Cancelled);
    }
    let video_concat = work_dir.join("video-concat.mp4");
    concat_media(&ffmpeg, work_dir, "video", &segments, &video_concat)?;
    if audio_clips.is_empty() {
        fs::rename(&video_concat, output).map_err(|error| YtpmError::io(output, error))?;
        return Ok(MediaJobStatus::Completed);
    }
    let mut audio_segments = Vec::with_capacity(audio_clips.len());
    for (index, clip) in audio_clips.iter().enumerate() {
        if is_cancelled(cancel_marker) {
            return Ok(MediaJobStatus::Cancelled);
        }
        let input = resolve_project_file(project_dir, &clip.relative_path)?;
        let segment = work_dir.join(format!("audio-{index:04}.m4a"));
        let volume = if clip.muted { 0.0 } else { clip.volume };
        let output_result = Command::new(&ffmpeg)
            .args([
                OsString::from("-hide_banner"),
                OsString::from("-loglevel"),
                OsString::from("error"),
                OsString::from("-y"),
                OsString::from("-ss"),
                OsString::from(format_seconds(clip.in_ms)),
                OsString::from("-i"),
                input.as_os_str().to_os_string(),
                OsString::from("-t"),
                OsString::from(format_seconds(clip.out_ms - clip.in_ms)),
                OsString::from("-vn"),
                OsString::from("-map"),
                OsString::from("0:a:0?"),
                OsString::from("-af"),
                OsString::from(format!("volume={volume}")),
                OsString::from("-c:a"),
                OsString::from("aac"),
                segment.as_os_str().to_os_string(),
            ])
            .output()
            .map_err(|error| tool_error("ffmpeg", &ffmpeg, error))?;
        if !output_result.status.success() {
            return Err(YtpmError::InvalidInput(format!(
                "FFmpeg 音訊裁切 {} 失敗：{}",
                clip.relative_path,
                String::from_utf8_lossy(&output_result.stderr).trim()
            )));
        }
        audio_segments.push(segment);
    }
    let audio_concat = work_dir.join("audio-concat.m4a");
    concat_media(&ffmpeg, work_dir, "audio", &audio_segments, &audio_concat)?;
    let output_result = Command::new(&ffmpeg)
        .args([
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-y"),
            OsString::from("-i"),
            video_concat.as_os_str().to_os_string(),
            OsString::from("-i"),
            audio_concat.as_os_str().to_os_string(),
            OsString::from("-map"),
            OsString::from("0:v:0"),
            OsString::from("-map"),
            OsString::from("1:a:0"),
            OsString::from("-c:v"),
            OsString::from("copy"),
            OsString::from("-c:a"),
            OsString::from("aac"),
            OsString::from("-shortest"),
            OsString::from("-movflags"),
            OsString::from("+faststart"),
            output.as_os_str().to_os_string(),
        ])
        .output()
        .map_err(|error| tool_error("ffmpeg", &ffmpeg, error))?;
    if !output_result.status.success() {
        return Err(YtpmError::InvalidInput(format!(
            "FFmpeg 混合 audio/video 失敗：{}",
            String::from_utf8_lossy(&output_result.stderr).trim()
        )));
    }
    Ok(MediaJobStatus::Completed)
}

fn concat_media(
    ffmpeg: &Path,
    work_dir: &Path,
    name: &str,
    segments: &[PathBuf],
    output: &Path,
) -> Result<()> {
    let list_path = work_dir.join(format!("{name}-concat.txt"));
    let list_content = segments
        .iter()
        .map(|path| format!("file '{}'", path.to_string_lossy().replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&list_path, format!("{list_content}\n"))
        .map_err(|error| YtpmError::io(&list_path, error))?;
    let output_result = Command::new(ffmpeg)
        .args([
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-y"),
            OsString::from("-f"),
            OsString::from("concat"),
            OsString::from("-safe"),
            OsString::from("0"),
            OsString::from("-i"),
            list_path.as_os_str().to_os_string(),
            OsString::from("-c"),
            OsString::from("copy"),
            output.as_os_str().to_os_string(),
        ])
        .output()
        .map_err(|error| tool_error("ffmpeg", ffmpeg, error))?;
    if !output_result.status.success() {
        return Err(YtpmError::InvalidInput(format!(
            "FFmpeg 合併 {name} 失敗：{}",
            String::from_utf8_lossy(&output_result.stderr).trim()
        )));
    }
    Ok(())
}

fn parse_stream(value: &Value) -> MediaStream {
    MediaStream {
        index: integer_value(value.get("index")),
        codec_type: string_value(value.get("codec_type")),
        codec_name: string_value(value.get("codec_name")),
        width: integer_value(value.get("width")),
        height: integer_value(value.get("height")),
        r_frame_rate: string_value(value.get("r_frame_rate")),
        sample_rate: integer_value(value.get("sample_rate")),
        channels: integer_value(value.get("channels")),
    }
}

fn string_value(value: Option<&Value>) -> Option<String> {
    value.and_then(|item| item.as_str().map(str::to_owned))
}

fn integer_value<T>(value: Option<&Value>) -> Option<T>
where
    T: TryFrom<u64>,
{
    value.and_then(|item| {
        item.as_u64()
            .or_else(|| item.as_str().and_then(|text| text.parse().ok()))
            .and_then(|number| T::try_from(number).ok())
    })
}

fn number_value(value: Option<&Value>) -> Option<f64> {
    value.and_then(|item| {
        item.as_f64()
            .or_else(|| item.as_str().and_then(|text| text.parse().ok()))
    })
}

fn resolve_project_file(project_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    if project_dir
        .components()
        .any(|item| item == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput("專案路徑不可包含 ..".into()));
    }
    reject_reparse_points(project_dir)?;
    let relative = Path::new(relative_path);
    if relative_path.trim().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|item| !matches!(item, Component::Normal(_)))
    {
        return Err(YtpmError::InvalidInput(format!(
            "不安全相對路徑：{relative_path}"
        )));
    }
    let path = project_dir.join(relative);
    reject_reparse_points(&path)?;
    if !path.exists() {
        return Err(YtpmError::InvalidInput(format!(
            "找不到媒體檔案：{relative_path}"
        )));
    }
    Ok(path)
}

fn resolve_project_output(project_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    if project_dir
        .components()
        .any(|item| item == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput("專案路徑不可包含 ..".into()));
    }
    reject_reparse_points(project_dir)?;
    let relative = Path::new(relative_path);
    if relative_path.trim().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|item| !matches!(item, Component::Normal(_)))
    {
        return Err(YtpmError::InvalidInput(format!(
            "不安全相對路徑：{relative_path}"
        )));
    }
    Ok(project_dir.join(relative))
}

fn reject_reparse_points(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if metadata_is_reparse_point(&metadata) => {
                return Err(YtpmError::InvalidInput(format!(
                    "拒絕操作 symlink/junction/reparse path：{}",
                    candidate.display()
                )))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(YtpmError::io(candidate, error)),
        }
        current = candidate.parent();
    }
    Ok(())
}

fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn tool_from_env(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(fallback))
}

fn tool_error(tool: &str, path: &Path, error: std::io::Error) -> YtpmError {
    YtpmError::InvalidInput(format!(
        "無法啟動 {tool}（{}）：{error}。請安裝 FFmpeg 並確認 PATH 或設定對應環境變數。",
        path.display()
    ))
}

fn format_seconds(value_ms: u64) -> String {
    format!("{:.6}", value_ms as f64 / 1000.0)
}

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(YtpmError::InvalidInput("operation id 格式無效".into()));
    }
    Ok(())
}

fn is_cancelled(marker: Option<&Path>) -> bool {
    marker.is_some_and(Path::exists)
}

fn cancelled_result(operation_id: String) -> MediaExportResult {
    MediaExportResult {
        operation_id,
        status: MediaJobStatus::Cancelled,
        progress: 0,
        output_relative_path: None,
        message: Some("匯出已取消，未覆寫既有檔案。".into()),
    }
}
