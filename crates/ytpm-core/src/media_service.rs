//! FFprobe/FFmpeg adapters.
//!
//! External tools are always started with `Command` argument vectors. No shell
//! command string is accepted, and all project paths are validated as relative
//! paths before they reach the process boundary.

use crate::filter_graph::compile_filter_graph;
use crate::subtitle_service::render_timeline_ass;
use crate::timeline_service::validate_output_relative_path;
use crate::{render_manifest, Result, Timeline, TimelineTrackKind, YtpmError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
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
    export_timeline_with_controls(
        project_dir,
        timeline,
        output_relative_path,
        &operation_id,
        None,
        cancel_marker,
        &|_| {},
    )
}

/// Runs one cancellable FFmpeg export while reporting percentage progress.
///
/// This is the queue-facing API. The established `export_timeline` entrypoint
/// remains a synchronous compatibility wrapper above it.
pub fn export_timeline_controlled(
    project_dir: &Path,
    timeline: &Timeline,
    output_relative_path: &str,
    operation_id: &str,
    cancel: &AtomicBool,
    progress: &dyn Fn(u8),
) -> Result<MediaExportResult> {
    export_timeline_with_controls(
        project_dir,
        timeline,
        output_relative_path,
        operation_id,
        Some(cancel),
        None,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn export_timeline_with_controls(
    project_dir: &Path,
    timeline: &Timeline,
    output_relative_path: &str,
    operation_id: &str,
    cancel: Option<&AtomicBool>,
    cancel_marker: Option<&Path>,
    progress: &dyn Fn(u8),
) -> Result<MediaExportResult> {
    validate_identifier(operation_id)?;
    validate_output_relative_path(output_relative_path)?;
    let output = resolve_project_output(project_dir, output_relative_path)?;
    reject_reparse_points(&output)?;
    if output.exists() {
        return Err(YtpmError::InvalidInput(format!(
            "拒絕覆寫既有匯出檔：{}",
            output_relative_path
        )));
    }
    let _manifest = render_manifest(timeline)?;
    let has_video = timeline
        .tracks
        .iter()
        .filter(|track| track.kind == TimelineTrackKind::Video)
        .any(|track| !track.clips.is_empty());
    if !has_video {
        return Err(YtpmError::InvalidInput(
            "timeline 沒有可匯出的 video clip".into(),
        ));
    }
    if cancellation_requested(cancel, cancel_marker) {
        return Ok(cancelled_result(operation_id.to_owned()));
    }
    if let Some(parent) = output.parent() {
        create_dir_all_checked(parent)?;
    }
    let work_dir = project_dir.join(format!(".ytpm-media-{operation_id}"));
    reject_reparse_points(&work_dir)?;
    fs::create_dir(&work_dir).map_err(|error| YtpmError::io(&work_dir, error))?;
    reject_reparse_points(&work_dir)?;
    let temporary_output = work_dir.join("render.mp4");
    let render_result = render_filter_graph_with_progress(
        project_dir,
        &work_dir,
        &temporary_output,
        timeline,
        cancel,
        cancel_marker,
        progress,
    );
    let result = match render_result {
        Ok(MediaJobStatus::Completed) => {
            publish_output_atomically(&temporary_output, &output, output_relative_path)
                .map(|()| MediaJobStatus::Completed)
        }
        other => other,
    };
    let cleanup = cleanup_work_dir(project_dir, &work_dir, operation_id);
    if let Err(error) = cleanup {
        if result.is_ok() {
            return Err(error);
        }
    }
    result.map(|status| match status {
        MediaJobStatus::Completed => MediaExportResult {
            operation_id: operation_id.to_owned(),
            status,
            progress: 100,
            output_relative_path: Some(output_relative_path.to_owned()),
            message: Some("FFmpeg filter graph 匯出完成。".into()),
        },
        MediaJobStatus::Cancelled => cancelled_result(operation_id.to_owned()),
        MediaJobStatus::Failed => MediaExportResult {
            operation_id: operation_id.to_owned(),
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
    create_dir_all_checked(&directory)?;
    let marker = directory.join(format!("{operation_id}.cancel"));
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
        .map_err(|error| YtpmError::io(&marker, error))?;
    Ok(marker)
}

#[allow(clippy::too_many_arguments)]
fn render_filter_graph_with_progress(
    project_dir: &Path,
    work_dir: &Path,
    output: &Path,
    timeline: &Timeline,
    cancel: Option<&AtomicBool>,
    cancel_marker: Option<&Path>,
    progress: &dyn Fn(u8),
) -> Result<MediaJobStatus> {
    let ffmpeg = tool_from_env("YTPM_FFMPEG_PATH", "ffmpeg");
    let ass_path = work_dir.join("burn-subtitles.ass");
    let burn_ass = render_timeline_ass(project_dir, timeline, &ass_path)?;
    let compiled = compile_filter_graph(timeline, burn_ass)?;
    if cancellation_requested(cancel, cancel_marker) {
        return Ok(MediaJobStatus::Cancelled);
    }

    let mut arguments = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-n"),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "color=c=black:s={}x{}:r={}:d={}",
            timeline.output.width,
            timeline.output.height,
            timeline.output.frame_rate,
            format_seconds(timeline.duration_ms)
        )),
    ];
    for input in &compiled.inputs {
        let path = resolve_project_file(project_dir, &input.relative_path)?;
        arguments.extend([OsString::from("-i"), path.as_os_str().to_os_string()]);
    }
    arguments.extend([
        OsString::from("-filter_complex"),
        OsString::from(&compiled.filter_complex),
        OsString::from("-map"),
        OsString::from(format!("[{}]", compiled.video_output_label)),
    ]);
    if let Some(audio_label) = &compiled.audio_output_label {
        arguments.extend([
            OsString::from("-map"),
            OsString::from(format!("[{audio_label}]")),
            OsString::from("-c:a"),
            OsString::from("aac"),
            OsString::from("-b:a"),
            OsString::from("192k"),
        ]);
    } else {
        arguments.push(OsString::from("-an"));
    }
    arguments.extend([
        OsString::from("-c:v"),
        OsString::from("libx264"),
        OsString::from("-preset"),
        OsString::from("medium"),
        OsString::from("-crf"),
        OsString::from("20"),
        OsString::from("-pix_fmt"),
        OsString::from("yuv420p"),
        OsString::from("-t"),
        OsString::from(format_seconds(timeline.duration_ms)),
        OsString::from("-movflags"),
        OsString::from("+faststart"),
        OsString::from("-progress"),
        OsString::from("pipe:1"),
        OsString::from("-nostats"),
        OsString::from("-f"),
        OsString::from("mp4"),
        output.as_os_str().to_os_string(),
    ]);

    run_ffmpeg_controlled(
        &ffmpeg,
        work_dir,
        arguments,
        timeline.duration_ms,
        cancel,
        cancel_marker,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_ffmpeg_controlled(
    ffmpeg: &Path,
    work_dir: &Path,
    arguments: Vec<OsString>,
    duration_ms: u64,
    cancel: Option<&AtomicBool>,
    cancel_marker: Option<&Path>,
    progress: &dyn Fn(u8),
) -> Result<MediaJobStatus> {
    let mut child = Command::new(ffmpeg)
        .current_dir(work_dir)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| tool_error("ffmpeg", ffmpeg, error))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| YtpmError::InvalidInput("無法讀取 FFmpeg progress pipe".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| YtpmError::InvalidInput("無法讀取 FFmpeg stderr".into()))?;
    let (sender, receiver) = mpsc::channel();
    let progress_reader = thread::spawn(move || {
        for line in BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let mut cancelled = false;
    let mut last_progress = 0u8;
    let status = loop {
        if cancellation_requested(cancel, cancel_marker) {
            cancelled = true;
            let _ = child.kill();
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if let Some(value) = parse_progress_line(&line, duration_ms) {
                    let value = value.clamp(1, 99);
                    if value > last_progress {
                        last_progress = value;
                        progress(value);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| YtpmError::io(ffmpeg, error))?
        {
            break status;
        }
    };
    let _ = progress_reader.join();
    let stderr_bytes = stderr_reader.join().unwrap_or_default();
    if cancelled {
        return Ok(MediaJobStatus::Cancelled);
    }
    if !status.success() {
        let message = String::from_utf8_lossy(&stderr_bytes).trim().to_owned();
        let hint = if message.contains("No such filter: 'subtitles'")
            || message.contains("Filter not found")
        {
            "目前 FFmpeg 未包含 libass subtitles filter，請安裝完整 FFmpeg build。"
        } else {
            "請檢查來源媒體 stream、effect 參數與 FFmpeg 安裝。"
        };
        return Err(YtpmError::InvalidInput(format!(
            "FFmpeg filter graph 匯出失敗：{message}。{hint}"
        )));
    }
    progress(100);
    Ok(MediaJobStatus::Completed)
}

fn parse_progress_line(line: &str, duration_ms: u64) -> Option<u8> {
    let (key, value) = line.split_once('=')?;
    if key == "progress" && value == "end" {
        return Some(100);
    }
    if !matches!(key, "out_time_us" | "out_time_ms") || duration_ms == 0 {
        return None;
    }
    let microseconds = value.parse::<u64>().ok()?;
    let elapsed_ms = microseconds / 1_000;
    Some(((elapsed_ms.saturating_mul(100) / duration_ms).min(100)) as u8)
}

fn cancellation_requested(cancel: Option<&AtomicBool>, marker: Option<&Path>) -> bool {
    cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) || marker.is_some_and(Path::exists)
}

fn cleanup_work_dir(project_dir: &Path, work_dir: &Path, operation_id: &str) -> Result<()> {
    let expected_name = format!(".ytpm-media-{operation_id}");
    let safe_name = work_dir.file_name().and_then(|value| value.to_str()) == Some(&expected_name);
    if work_dir.parent() != Some(project_dir) || !safe_name {
        return Err(YtpmError::InvalidInput(
            "拒絕清理專案外或名稱不符的 media 暫存目錄".into(),
        ));
    }
    reject_reparse_points(work_dir)?;
    fs::remove_dir_all(work_dir).map_err(|error| YtpmError::io(work_dir, error))
}

fn publish_output_atomically(
    temporary_output: &Path,
    output: &Path,
    output_relative_path: &str,
) -> Result<()> {
    reject_reparse_points(temporary_output)?;
    reject_reparse_points(output)?;
    let metadata = fs::symlink_metadata(temporary_output)
        .map_err(|error| YtpmError::io(temporary_output, error))?;
    if metadata_is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(YtpmError::InvalidInput(format!(
            "FFmpeg 暫存輸出不是一般檔案：{}",
            temporary_output.display()
        )));
    }
    match fs::hard_link(temporary_output, output) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(YtpmError::InvalidInput(format!(
                "拒絕覆寫既有匯出檔：{output_relative_path}"
            )));
        }
        Err(error) => return Err(YtpmError::io(output, error)),
    }
    let published = fs::symlink_metadata(output).map_err(|error| YtpmError::io(output, error))?;
    if metadata_is_reparse_point(&published) || !published.is_file() {
        return Err(YtpmError::InvalidInput(format!(
            "原子發布後輸出不是一般檔案：{}",
            output.display()
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

pub(crate) fn resolve_project_file(project_dir: &Path, relative_path: &str) -> Result<PathBuf> {
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
    validate_output_relative_path(relative_path)?;
    let relative = Path::new(relative_path);
    Ok(project_dir.join(relative))
}

pub(crate) fn create_dir_all_checked(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| YtpmError::io(path, error))?
            .join(path)
    };
    reject_reparse_points(&absolute)?;

    let mut missing = Vec::new();
    let mut current = absolute.as_path();
    loop {
        match fs::symlink_metadata(current) {
            Ok(metadata) => {
                if metadata_is_reparse_point(&metadata) || !metadata.is_dir() {
                    return Err(YtpmError::InvalidInput(format!(
                        "拒絕使用 reparse point 或非資料夾路徑：{}",
                        current.display()
                    )));
                }
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(current.to_path_buf());
                current = current.parent().ok_or_else(|| {
                    YtpmError::InvalidInput(format!("找不到可建立的上層資料夾：{}", path.display()))
                })?;
            }
            Err(error) => return Err(YtpmError::io(current, error)),
        }
    }

    for directory in missing.into_iter().rev() {
        let parent = directory.parent().ok_or_else(|| {
            YtpmError::InvalidInput(format!("找不到資料夾 parent：{}", directory.display()))
        })?;
        reject_reparse_points(parent)?;
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(YtpmError::io(&directory, error)),
        }
        let metadata =
            fs::symlink_metadata(&directory).map_err(|error| YtpmError::io(&directory, error))?;
        if metadata_is_reparse_point(&metadata) || !metadata.is_dir() {
            return Err(YtpmError::InvalidInput(format!(
                "建立後偵測到 reparse point 或非資料夾路徑：{}",
                directory.display()
            )));
        }
        reject_reparse_points(&directory)?;
    }
    Ok(())
}

pub(crate) fn reject_reparse_points(path: &Path) -> Result<()> {
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

pub(crate) fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
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

fn cancelled_result(operation_id: String) -> MediaExportResult {
    MediaExportResult {
        operation_id,
        status: MediaJobStatus::Cancelled,
        progress: 0,
        output_relative_path: None,
        message: Some("匯出已取消，未覆寫既有檔案。".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[test]
    fn concurrent_atomic_publish_never_overwrites_destination() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.mp4");
        let second = directory.path().join("second.mp4");
        let output = directory.path().join("final.mp4");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        let barrier = Arc::new(Barrier::new(3));

        let workers = [first.clone(), second.clone()]
            .into_iter()
            .map(|temporary| {
                let barrier = Arc::clone(&barrier);
                let output = output.clone();
                thread::spawn(move || {
                    barrier.wait();
                    publish_output_atomically(&temporary, &output, "final.mp4")
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let successes = workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap().is_ok()))
            .sum::<usize>();

        assert_eq!(successes, 1);
        let published = fs::read(&output).unwrap();
        assert!(published == b"first" || published == b"second");
        let third = directory.path().join("third.mp4");
        fs::write(&third, b"third").unwrap();
        assert!(publish_output_atomically(&third, &output, "final.mp4").is_err());
        assert_eq!(fs::read(&output).unwrap(), published);
    }
}
