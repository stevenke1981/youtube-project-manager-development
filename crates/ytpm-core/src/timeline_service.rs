//! Portable, non-destructive timeline service.
//!
//! `timeline.json` is the source of truth.  This module stores only project
//! relative paths and UUID references; it never copies or removes media files.

use crate::error::{Result, YtpmError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub const CURRENT_TIMELINE_SCHEMA_VERSION: u32 = 1;
pub const TIMELINE_SCHEMA_VERSION: u32 = CURRENT_TIMELINE_SCHEMA_VERSION;
const TIMELINE_FILE_NAME: &str = "timeline.json";
const MAX_TIMELINE_JSON_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub kind: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderSettings {
    pub output_relative_path: String,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            output_relative_path: "09_exports/timeline.mp4".into(),
            format: "mp4".into(),
            width: 1920,
            height: 1080,
            frame_rate: 30.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: String,
    pub asset_id: String,
    pub relative_path: String,
    pub label: String,
    pub start_ms: u64,
    pub in_ms: u64,
    pub out_ms: u64,
    pub duration_ms: u64,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: String,
    pub label: String,
    pub kind: TrackKind,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    pub schema_version: u32,
    pub duration_ms: u64,
    #[serde(default)]
    pub tracks: Vec<Track>,
    pub output: RenderSettings,
    pub updated_at: DateTime<Utc>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_TIMELINE_SCHEMA_VERSION,
            duration_ms: 0,
            tracks: vec![
                Track {
                    id: "00000000-0000-0000-0000-000000000001".into(),
                    label: "V1 · 主畫面".into(),
                    kind: TrackKind::Video,
                    clips: Vec::new(),
                },
                Track {
                    id: "00000000-0000-0000-0000-000000000002".into(),
                    label: "A1 · 主音訊".into(),
                    kind: TrackKind::Audio,
                    clips: Vec::new(),
                },
                Track {
                    id: "00000000-0000-0000-0000-000000000003".into(),
                    label: "S1 · 字幕".into(),
                    kind: TrackKind::Subtitle,
                    clips: Vec::new(),
                },
            ],
            output: RenderSettings::default(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipRequest {
    pub asset_id: String,
    pub relative_path: String,
    pub label: String,
    pub start_ms: u64,
    pub in_ms: u64,
    pub out_ms: u64,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClipPatch {
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub relative_path: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub start_ms: Option<u64>,
    #[serde(default)]
    pub in_ms: Option<u64>,
    #[serde(default)]
    pub out_ms: Option<u64>,
    #[serde(default)]
    pub volume: Option<f32>,
    #[serde(default)]
    pub muted: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub transition: Option<Option<Transition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderInput {
    pub track_id: String,
    pub clip_id: String,
    pub asset_id: String,
    pub relative_path: String,
    pub start_ms: u64,
    pub in_ms: u64,
    pub out_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub muted: bool,
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderManifest {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub output: RenderSettings,
    pub inputs: Vec<RenderInput>,
}

// Compatibility names used by the existing core re-export boundary.
pub type TimelineClip = Clip;
pub type TimelineTrack = Track;
pub type TimelineTrackKind = TrackKind;
pub type TimelineClipRequest = ClipRequest;
pub type TimelineClipPatch = ClipPatch;
pub type RenderManifestClip = RenderInput;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineIssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimelineValidationIssue {
    pub code: String,
    pub severity: TimelineIssueSeverity,
    pub message: String,
    pub clip_id: Option<String>,
    pub track_id: Option<String>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimelineValidationReport {
    pub valid: bool,
    pub duration_ms: u64,
    pub issues: Vec<TimelineValidationIssue>,
}

/// Reads `timeline.json`, creating an empty portable timeline when absent.
pub fn read_timeline(project_dir: &Path) -> Result<Timeline> {
    ensure_project_dir(project_dir)?;
    let path = timeline_path(project_dir);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            reject_reparse_points(&path)?;
            if !metadata.is_file() {
                return Err(YtpmError::InvalidInput(format!(
                    "timeline.json 不是一般檔案：{}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let timeline = Timeline::default();
            return write_timeline(project_dir, &timeline);
        }
        Err(error) => return Err(YtpmError::io(&path, error)),
    }

    let metadata = fs::metadata(&path).map_err(|error| YtpmError::io(&path, error))?;
    if metadata.len() > MAX_TIMELINE_JSON_BYTES {
        return Err(YtpmError::InvalidProject(format!(
            "timeline.json 超過 {} MiB",
            MAX_TIMELINE_JSON_BYTES / (1024 * 1024)
        )));
    }
    let content = fs::read_to_string(&path).map_err(|error| YtpmError::io(&path, error))?;
    let timeline: Timeline = serde_json::from_str(&content)?;
    validate(&timeline)?;
    Ok(timeline)
}

/// Validates and atomically replaces `timeline.json`.
pub fn write_timeline(project_dir: &Path, timeline: &Timeline) -> Result<Timeline> {
    ensure_project_dir(project_dir)?;
    let mut persisted = timeline.clone();
    validate(&persisted)?;
    persisted.updated_at = Utc::now();
    atomic_write_json(&timeline_path(project_dir), &persisted)?;
    Ok(persisted)
}

/// Adds a newly identified clip to an existing track and keeps media untouched.
pub fn add_clip(project_dir: &Path, track_id: &str, request: ClipRequest) -> Result<Clip> {
    validate_uuid(track_id, "track id")?;
    let mut timeline = read_timeline(project_dir)?;
    let track_index = find_track_index(&timeline, track_id)?;
    let clip = Clip {
        id: Uuid::new_v4().to_string(),
        asset_id: request.asset_id,
        relative_path: request.relative_path,
        label: request.label,
        start_ms: request.start_ms,
        in_ms: request.in_ms,
        out_ms: request.out_ms,
        duration_ms: request
            .out_ms
            .checked_sub(request.in_ms)
            .ok_or_else(|| YtpmError::InvalidInput("clip out_ms 必須大於 in_ms".into()))?,
        volume: request.volume,
        muted: request.muted,
        transition: request.transition,
    };
    validate_clip(&clip)?;
    timeline.duration_ms = timeline.duration_ms.max(clip_end(&clip)?);
    timeline.tracks[track_index].clips.push(clip.clone());
    validate(&timeline)?;
    write_timeline(project_dir, &timeline)?;
    Ok(clip)
}

/// Applies a partial clip update, optionally moving the clip to another track.
pub fn update_clip(project_dir: &Path, clip_id: &str, patch: ClipPatch) -> Result<Clip> {
    validate_uuid(clip_id, "clip id")?;
    let mut timeline = read_timeline(project_dir)?;
    let (old_track_index, clip_index) = find_clip_index(&timeline, clip_id)?;
    let mut clip = timeline.tracks[old_track_index].clips[clip_index].clone();

    if let Some(asset_id) = patch.asset_id {
        clip.asset_id = asset_id;
    }
    if let Some(relative_path) = patch.relative_path {
        clip.relative_path = relative_path;
    }
    if let Some(label) = patch.label {
        clip.label = label;
    }
    if let Some(start_ms) = patch.start_ms {
        clip.start_ms = start_ms;
    }
    if let Some(in_ms) = patch.in_ms {
        clip.in_ms = in_ms;
    }
    if let Some(out_ms) = patch.out_ms {
        clip.out_ms = out_ms;
    }
    clip.duration_ms = clip
        .out_ms
        .checked_sub(clip.in_ms)
        .ok_or_else(|| YtpmError::InvalidInput("clip out_ms 必須大於 in_ms".into()))?;
    if let Some(volume) = patch.volume {
        clip.volume = volume;
    }
    if let Some(muted) = patch.muted {
        clip.muted = muted;
    }
    if let Some(transition) = patch.transition {
        clip.transition = transition;
    }
    validate_clip(&clip)?;

    let target_track_index = match patch.track_id {
        Some(track_id) => {
            validate_uuid(&track_id, "track id")?;
            find_track_index(&timeline, &track_id)?
        }
        None => old_track_index,
    };
    let updated = if old_track_index == target_track_index {
        timeline.tracks[old_track_index].clips[clip_index] = clip.clone();
        clip
    } else {
        timeline.tracks[old_track_index].clips.remove(clip_index);
        timeline.tracks[target_track_index].clips.push(clip.clone());
        clip
    };
    timeline.duration_ms = timeline.duration_ms.max(clip_end(&updated)?);
    validate(&timeline)?;
    write_timeline(project_dir, &timeline)?;
    Ok(updated)
}

/// Removes a clip record only; the referenced media file remains in place.
pub fn remove_clip(project_dir: &Path, clip_id: &str) -> Result<Clip> {
    validate_uuid(clip_id, "clip id")?;
    let mut timeline = read_timeline(project_dir)?;
    let (track_index, clip_index) = find_clip_index(&timeline, clip_id)?;
    let removed = timeline.tracks[track_index].clips.remove(clip_index);
    validate(&timeline)?;
    write_timeline(project_dir, &timeline)?;
    Ok(removed)
}

/// Produces a deterministic, portable input list for a future media adapter.
pub fn render_manifest(timeline: &Timeline) -> Result<RenderManifest> {
    validate(timeline)?;
    let inputs = timeline
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter().map(move |clip| (track, clip)))
        .map(|(track, clip)| RenderInput {
            track_id: track.id.clone(),
            clip_id: clip.id.clone(),
            asset_id: clip.asset_id.clone(),
            relative_path: clip.relative_path.clone(),
            start_ms: clip.start_ms,
            in_ms: clip.in_ms,
            out_ms: clip.out_ms,
            duration_ms: clip.duration_ms,
            volume: clip.volume,
            muted: clip.muted,
            transition: clip.transition.clone(),
        })
        .collect();

    Ok(RenderManifest {
        schema_version: CURRENT_TIMELINE_SCHEMA_VERSION,
        generated_at: Utc::now(),
        duration_ms: timeline.duration_ms,
        output: timeline.output.clone(),
        inputs,
    })
}

/// Returns a UI-friendly validation report while `validate` remains the
/// fail-fast API used by reads and writes.
pub fn validate_timeline(timeline: &Timeline) -> TimelineValidationReport {
    match validate(timeline) {
        Ok(()) => TimelineValidationReport {
            valid: true,
            duration_ms: timeline.duration_ms,
            issues: Vec::new(),
        },
        Err(error) => TimelineValidationReport {
            valid: false,
            duration_ms: timeline.duration_ms,
            issues: vec![TimelineValidationIssue {
                code: "TIMELINE_INVALID".into(),
                severity: TimelineIssueSeverity::Error,
                message: error.to_string(),
                clip_id: None,
                track_id: None,
                suggested_action: "修正 timeline.json 的 UUID、相對路徑、trim 或同軌重疊後重試"
                    .into(),
            }],
        },
    }
}

/// Validates schema, UUIDs, portable paths, trim ranges, and same-track overlap.
pub fn validate(timeline: &Timeline) -> Result<()> {
    if timeline.schema_version != CURRENT_TIMELINE_SCHEMA_VERSION {
        return Err(YtpmError::InvalidProject(format!(
            "不支援 timeline.json schema_version {}，目前版本為 {}",
            timeline.schema_version, CURRENT_TIMELINE_SCHEMA_VERSION
        )));
    }
    validate_render_settings(&timeline.output)?;

    let mut track_ids = HashSet::with_capacity(timeline.tracks.len());
    let mut clip_ids = HashSet::new();
    for track in &timeline.tracks {
        validate_uuid(&track.id, "track id")?;
        if !track_ids.insert(track.id.as_str()) {
            return Err(YtpmError::InvalidProject(format!(
                "track id 不可重複：{}",
                track.id
            )));
        }
        if track.label.trim().is_empty() {
            return Err(YtpmError::InvalidProject(format!(
                "track label 不可為空：{}",
                track.id
            )));
        }

        let mut clips = Vec::with_capacity(track.clips.len());
        for clip in &track.clips {
            validate_clip(clip)?;
            if !clip_ids.insert(clip.id.as_str()) {
                return Err(YtpmError::InvalidProject(format!(
                    "clip id 不可重複：{}",
                    clip.id
                )));
            }
            let end = clip_end(clip)?;
            if end > timeline.duration_ms {
                return Err(YtpmError::InvalidProject(format!(
                    "clip 超出 timeline duration_ms：{}",
                    clip.id
                )));
            }
            clips.push((clip.start_ms, end, clip.id.as_str()));
        }
        clips.sort_by_key(|(start, _, _)| *start);
        for pair in clips.windows(2) {
            if pair[0].1 > pair[1].0 {
                return Err(YtpmError::InvalidProject(format!(
                    "同一 track 的 clips 不可重疊：{} 與 {}",
                    pair[0].2, pair[1].2
                )));
            }
        }
    }
    Ok(())
}

fn validate_clip(clip: &Clip) -> Result<()> {
    validate_uuid(&clip.id, "clip id")?;
    validate_uuid(&clip.asset_id, "asset id")?;
    validate_relative_path(&clip.relative_path, "clip relative_path")?;
    if clip.label.trim().is_empty() {
        return Err(YtpmError::InvalidInput(format!(
            "clip label 不可為空：{}",
            clip.id
        )));
    }
    if clip.out_ms <= clip.in_ms {
        return Err(YtpmError::InvalidInput(format!(
            "clip out_ms 必須大於 in_ms：{}",
            clip.id
        )));
    }
    let expected_duration = clip.out_ms - clip.in_ms;
    if clip.duration_ms != expected_duration {
        return Err(YtpmError::InvalidInput(format!(
            "clip duration_ms 必須等於 out_ms - in_ms：{}",
            clip.id
        )));
    }
    if !clip.volume.is_finite() || clip.volume < 0.0 {
        return Err(YtpmError::InvalidInput(format!(
            "clip volume 必須是非負有限數字：{}",
            clip.id
        )));
    }
    if let Some(transition) = &clip.transition {
        if transition.kind.trim().is_empty() {
            return Err(YtpmError::InvalidInput(format!(
                "transition kind 不可為空：{}",
                clip.id
            )));
        }
        if transition.duration_ms > clip.duration_ms {
            return Err(YtpmError::InvalidInput(format!(
                "transition duration_ms 不可超過 clip duration_ms：{}",
                clip.id
            )));
        }
    }
    Ok(())
}

fn validate_render_settings(settings: &RenderSettings) -> Result<()> {
    validate_relative_path(
        &settings.output_relative_path,
        "output output_relative_path",
    )?;
    if settings.format.trim().is_empty()
        || settings.format.chars().any(|character| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '-'
        })
    {
        return Err(YtpmError::InvalidInput(
            "output format 必須是簡單的檔案格式名稱".into(),
        ));
    }
    if settings.width == 0 || settings.height == 0 {
        return Err(YtpmError::InvalidInput(
            "output width 與 height 必須大於 0".into(),
        ));
    }
    if !settings.frame_rate.is_finite() || settings.frame_rate <= 0.0 {
        return Err(YtpmError::InvalidInput(
            "output frame_rate 必須是正的有限數字".into(),
        ));
    }
    Ok(())
}

fn clip_end(clip: &Clip) -> Result<u64> {
    clip.start_ms
        .checked_add(clip.duration_ms)
        .ok_or_else(|| YtpmError::InvalidInput(format!("clip 時間超過 u64 上限：{}", clip.id)))
}

fn find_track_index(timeline: &Timeline, track_id: &str) -> Result<usize> {
    timeline
        .tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| YtpmError::InvalidInput(format!("找不到 track id：{track_id}")))
}

fn find_clip_index(timeline: &Timeline, clip_id: &str) -> Result<(usize, usize)> {
    timeline
        .tracks
        .iter()
        .enumerate()
        .find_map(|(track_index, track)| {
            track
                .clips
                .iter()
                .position(|clip| clip.id == clip_id)
                .map(|clip_index| (track_index, clip_index))
        })
        .ok_or_else(|| YtpmError::InvalidInput(format!("找不到 clip id：{clip_id}")))
}

fn validate_uuid(value: &str, field: &str) -> Result<()> {
    if Uuid::parse_str(value).is_err() {
        return Err(YtpmError::InvalidInput(format!(
            "{field} 不是有效 UUID：{value}"
        )));
    }
    Ok(())
}

fn validate_relative_path(value: &str, field: &str) -> Result<()> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains("//")
        || Path::new(value).is_absolute()
    {
        return Err(YtpmError::InvalidInput(format!(
            "{field} 必須是安全的專案相對路徑：{value}"
        )));
    }
    for component in value.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(YtpmError::InvalidInput(format!(
                "{field} 不可包含 .、.. 或空白路徑元件：{value}"
            )));
        }
        if component
            .chars()
            .any(|character| character.is_control() || r#"<>:\"|?*"#.contains(character))
            || component.ends_with(' ')
            || component.ends_with('.')
        {
            return Err(YtpmError::InvalidInput(format!(
                "{field} 包含非法 Windows 檔名元件：{value}"
            )));
        }
        let stem = component
            .split_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(component)
            .to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(YtpmError::InvalidInput(format!(
                "{field} 使用 Windows 保留名稱：{value}"
            )));
        }
    }
    Ok(())
}

fn ensure_project_dir(project_dir: &Path) -> Result<()> {
    if project_dir
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(format!(
            "專案路徑不可包含 ..：{}",
            project_dir.display()
        )));
    }
    if !project_dir.exists() {
        fs::create_dir_all(project_dir).map_err(|error| YtpmError::io(project_dir, error))?;
    }
    reject_reparse_points(project_dir)?;
    let metadata = fs::metadata(project_dir).map_err(|error| YtpmError::io(project_dir, error))?;
    if !metadata.is_dir() {
        return Err(YtpmError::InvalidInput(format!(
            "專案路徑不是資料夾：{}",
            project_dir.display()
        )));
    }
    Ok(())
}

fn timeline_path(project_dir: &Path) -> PathBuf {
    project_dir.join(TIMELINE_FILE_NAME)
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path.parent().ok_or_else(|| {
        YtpmError::InvalidInput(format!("找不到 JSON parent：{}", path.display()))
    })?;
    reject_reparse_points(parent)?;
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata_is_reparse_point(&metadata) {
            return Err(YtpmError::InvalidInput(format!(
                "拒絕覆寫 symlink/junction/reparse timeline：{}",
                path.display()
            )));
        }
    }
    let file_name = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
        YtpmError::InvalidInput(format!("timeline.json 檔名無效：{}", path.display()))
    })?;
    let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        file.write_all(&bytes)
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        file.write_all(b"\n")
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        file.sync_all()
            .map_err(|error| YtpmError::io(&temporary_path, error))?;
        replace_file(&temporary_path, path).map_err(|error| YtpmError::io(path, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temporary_path, path)
}

#[cfg(windows)]
fn replace_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(existing: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let existing: Vec<u16> = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            existing.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
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

fn default_volume() -> f32 {
    1.0
}

fn deserialize_double_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}
