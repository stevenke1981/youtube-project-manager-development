use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ytpm_core::{
    Asset, AssetCatalog, AssetState, CreateProjectRequest, MediaJobQueue, MediaJobRecord,
    MediaProbe, OAuthCallbackResult, OAuthStart, Project, ProjectStatus, PublishConfigReference,
    PublishMetadata, PublishResult, RecoveryReport, Task, TaskPatch, TaskRequest, TaskStatus,
    Timeline, ValidationReport, YtpmError,
};

#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub human_message: String,
    pub technical_detail: Option<String>,
    pub recoverable: bool,
    pub suggested_action: Option<&'static str>,
}

impl From<YtpmError> for CommandError {
    fn from(error: YtpmError) -> Self {
        let (code, recoverable, suggested_action) = match &error {
            YtpmError::InvalidInput(_) => ("INVALID_INPUT", true, Some("修正輸入後重試")),
            YtpmError::Io { .. } => ("FILESYSTEM_ERROR", true, Some("選擇可寫入的 Library root")),
            YtpmError::Json(_) => (
                "JSON_INVALID",
                false,
                Some("從備份恢復 project.json 或相關資料檔"),
            ),
            YtpmError::InvalidProject(_) => (
                "INVALID_PROJECT",
                true,
                Some("先執行 validate 或修正專案資料，再重試操作"),
            ),
        };
        Self {
            code,
            human_message: error.to_string(),
            technical_detail: Some(format!("{error:?}")),
            recoverable,
            suggested_action,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexReportDto {
    pub db_path: String,
    pub scanned: usize,
    pub indexed: usize,
    pub invalid: usize,
    pub rebuilt_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssetDto {
    pub id: String,
    pub kind: ytpm_core::AssetKind,
    pub relative_path: String,
    pub display_name: Option<String>,
    pub state: AssetState,
    pub source_type: Option<String>,
    pub generator: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub duration_ms: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub version_group_id: Option<String>,
    pub version_number: u32,
    pub is_adopted: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AssetCatalogDto {
    pub project_path: String,
    pub scanned_at: String,
    pub assets: Vec<AssetDto>,
    pub total: usize,
    pub available: usize,
    pub missing: usize,
    pub invalid: usize,
}

#[derive(Debug, Serialize)]
pub struct DocumentWriteResultDto {
    pub saved_at: String,
}

#[derive(Debug, Serialize)]
pub struct RecoveryReportDto {
    pub recovered: bool,
    pub had_journal: bool,
    pub journal_path: Option<String>,
    pub message: Option<String>,
    pub actions: Vec<String>,
}

fn require_confirmation(operation: &str, confirm: bool) -> Result<(), CommandError> {
    if confirm {
        Ok(())
    } else {
        Err(CommandError {
            code: "CONFIRMATION_REQUIRED",
            human_message: format!("{operation} 需要明確確認才能執行。"),
            technical_detail: Some("The command requires confirm=true.".to_string()),
            recoverable: true,
            suggested_action: Some("確認輸出或發布內容後，以 confirm=true 重試"),
        })
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn timeline_load(project_path: String) -> Result<Timeline, CommandError> {
    ytpm_core::read_timeline(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn timeline_save(
    project_path: String,
    timeline: Timeline,
) -> Result<Timeline, CommandError> {
    ytpm_core::write_timeline(Path::new(&project_path), &timeline).map_err(CommandError::from)
}

#[derive(Debug, Deserialize)]
pub struct MediaExportRequestDto {
    pub source_asset_id: Option<String>,
    pub output_relative_path: String,
    pub format: String,
    pub timeline: Timeline,
}

fn require_mp4_export(request: &MediaExportRequestDto) -> Result<(), CommandError> {
    let output_is_mp4 = Path::new(&request.output_relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"));
    if request.format.eq_ignore_ascii_case("mp4") && output_is_mp4 {
        return Ok(());
    }

    Err(YtpmError::InvalidInput("桌面匯出目前只支援 MP4，輸出路徑必須以 .mp4 結尾。".into()).into())
}

#[derive(Debug, Serialize)]
pub struct MediaMetadataDto {
    pub asset_id: Option<String>,
    pub relative_path: String,
    pub format_name: String,
    pub duration_seconds: Option<f64>,
    pub size_bytes: Option<u64>,
    pub bitrate_bps: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub frame_rate: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub probed_at: String,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_probe(
    project_path: String,
    asset_id: Option<String>,
    relative_path: String,
) -> Result<MediaMetadataDto, CommandError> {
    let probe = ytpm_core::probe_media(Path::new(&project_path), &relative_path)
        .map_err(CommandError::from)?;
    Ok(media_metadata_dto(asset_id, probe))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_export(
    project_path: String,
    request: MediaExportRequestDto,
    confirm: bool,
) -> Result<ytpm_core::MediaExportResult, CommandError> {
    require_confirmation("media.export", confirm)?;
    require_mp4_export(&request)?;
    let _ = request.source_asset_id;
    ytpm_core::export_timeline(
        Path::new(&project_path),
        &request.timeline,
        &request.output_relative_path,
        None,
    )
    .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_export_enqueue(
    queue: tauri::State<'_, MediaJobQueue>,
    project_path: String,
    request: MediaExportRequestDto,
    confirm: bool,
) -> Result<MediaJobRecord, CommandError> {
    require_confirmation("media.export.enqueue", confirm)?;
    require_mp4_export(&request)?;
    let _ = request.source_asset_id;
    queue
        .enqueue_export(
            PathBuf::from(project_path),
            request.timeline,
            request.output_relative_path,
        )
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_job_status(
    queue: tauri::State<'_, MediaJobQueue>,
    project_path: String,
    job_id: String,
) -> Result<MediaJobRecord, CommandError> {
    queue
        .status_for_project(Path::new(&project_path), &job_id)
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_job_list(
    queue: tauri::State<'_, MediaJobQueue>,
    project_path: String,
) -> Result<Vec<MediaJobRecord>, CommandError> {
    queue
        .list_for_project(Path::new(&project_path))
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_job_cancel(
    queue: tauri::State<'_, MediaJobQueue>,
    project_path: String,
    job_id: String,
) -> Result<MediaJobRecord, CommandError> {
    queue
        .cancel_for_project(Path::new(&project_path), &job_id)
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn media_operation_cancel(
    project_path: String,
    operation_id: String,
    kind: String,
) -> Result<(), CommandError> {
    let _ = kind;
    ytpm_core::cancel_marker(Path::new(&project_path), &operation_id)
        .map(|_| ())
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_config_reference(
    _project_path: String,
) -> Result<PublishConfigReference, CommandError> {
    Ok(ytpm_core::config_reference())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishOAuthCallbackRequest {
    pub callback_url: String,
    pub expected_state: String,
    pub code_verifier: String,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_auth_start() -> Result<OAuthStart, CommandError> {
    ytpm_core::start_oauth().map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_auth_callback(
    request: PublishOAuthCallbackRequest,
) -> Result<OAuthCallbackResult, CommandError> {
    ytpm_core::complete_oauth(
        &request.callback_url,
        &request.expected_state,
        &request.code_verifier,
    )
    .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_metadata_load(project_path: String) -> Result<PublishMetadata, CommandError> {
    ytpm_core::load_publish_metadata(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_metadata_save(
    project_path: String,
    metadata: PublishMetadata,
) -> Result<PublishMetadata, CommandError> {
    ytpm_core::save_publish_metadata(Path::new(&project_path), &metadata)
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_dry_run(
    project_path: String,
    metadata: PublishMetadata,
) -> Result<PublishResult, CommandError> {
    ytpm_core::publish_dry_run(Path::new(&project_path), &metadata).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_upload(
    project_path: String,
    metadata: PublishMetadata,
    confirm: bool,
) -> Result<PublishResult, CommandError> {
    require_confirmation("publish.upload", confirm)?;
    ytpm_core::upload_video(Path::new(&project_path), &metadata, None).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn publish_cancel(
    project_path: String,
    operation_id: String,
) -> Result<(), CommandError> {
    ytpm_core::cancel_marker(Path::new(&project_path), &operation_id)
        .map(|_| ())
        .map_err(CommandError::from)
}

fn media_metadata_dto(asset_id: Option<String>, probe: MediaProbe) -> MediaMetadataDto {
    let video = probe
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"));
    let audio = probe
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));
    MediaMetadataDto {
        asset_id,
        relative_path: probe.relative_path,
        format_name: probe.format_name.unwrap_or_default(),
        duration_seconds: probe.duration_seconds,
        size_bytes: probe.size_bytes,
        bitrate_bps: probe.bitrate_bps,
        width: video.and_then(|stream| stream.width),
        height: video.and_then(|stream| stream.height),
        video_codec: video.and_then(|stream| stream.codec_name.clone()),
        audio_codec: audio.and_then(|stream| stream.codec_name.clone()),
        frame_rate: video.and_then(|stream| stream.r_frame_rate.clone()),
        sample_rate: audio.and_then(|stream| stream.sample_rate),
        channels: audio.and_then(|stream| stream.channels),
        probed_at: probe.probed_at,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_create(
    root_path: String,
    request: CreateProjectRequest,
) -> Result<Project, CommandError> {
    ytpm_core::create_project(Path::new(&root_path), request).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_list(root_path: String) -> Result<Vec<Project>, CommandError> {
    ytpm_core::list_projects(Path::new(&root_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_validate(project_path: String) -> Result<ValidationReport, CommandError> {
    ytpm_core::validate_project(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_update_status(
    project_path: String,
    status: ProjectStatus,
) -> Result<Project, CommandError> {
    ytpm_core::update_project_status(Path::new(&project_path), status).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_archive(project_path: String) -> Result<Project, CommandError> {
    ytpm_core::archive_project(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_restore(project_path: String) -> Result<Project, CommandError> {
    ytpm_core::restore_project(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_migrate(project_path: String) -> Result<Project, CommandError> {
    ytpm_core::migrate_project(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_index_rebuild(root_path: String) -> Result<IndexReportDto, CommandError> {
    let report = ytpm_core::rebuild_index(Path::new(&root_path)).map_err(CommandError::from)?;
    Ok(IndexReportDto {
        db_path: report.db_path.to_string_lossy().into_owned(),
        scanned: report.scanned,
        indexed: report.indexed,
        invalid: report.invalid,
        rebuilt_at: Some(now_rfc3339()),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_index_search(
    root_path: String,
    query: Option<String>,
    status: Option<ProjectStatus>,
) -> Result<Vec<Project>, CommandError> {
    ytpm_core::search_index(Path::new(&root_path), query.as_deref(), status)
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn task_list(project_path: String) -> Result<Vec<Task>, CommandError> {
    ytpm_core::list_tasks(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn task_create(project_path: String, request: TaskRequest) -> Result<Task, CommandError> {
    ytpm_core::create_task(Path::new(&project_path), request).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn task_update(
    project_path: String,
    task_id: String,
    patch: TaskPatch,
) -> Result<Task, CommandError> {
    ytpm_core::update_task(Path::new(&project_path), &task_id, patch).map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn task_move(
    project_path: String,
    task_id: String,
    status: TaskStatus,
    order_key: f64,
) -> Result<Task, CommandError> {
    ytpm_core::move_task(Path::new(&project_path), &task_id, status, order_key)
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn asset_scan(project_path: String) -> Result<AssetCatalogDto, CommandError> {
    let path = Path::new(&project_path);
    let catalog = ytpm_core::scan_assets(path).map_err(CommandError::from)?;
    Ok(asset_catalog_dto(path, catalog))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn asset_list(project_path: String) -> Result<Vec<AssetDto>, CommandError> {
    ytpm_core::list_assets(Path::new(&project_path))
        .map(|assets| assets.into_iter().map(asset_dto).collect())
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn document_read(
    project_path: String,
    relative_path: String,
) -> Result<String, CommandError> {
    ytpm_core::read_document(Path::new(&project_path), Path::new(&relative_path))
        .map_err(CommandError::from)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn document_write(
    project_path: String,
    relative_path: String,
    content: String,
) -> Result<DocumentWriteResultDto, CommandError> {
    ytpm_core::write_document(
        Path::new(&project_path),
        Path::new(&relative_path),
        &content,
    )
    .map_err(CommandError::from)?;
    Ok(DocumentWriteResultDto {
        saved_at: now_rfc3339(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn project_recover_journal(root_path: String) -> Result<RecoveryReportDto, CommandError> {
    let root = PathBuf::from(&root_path);
    let report = ytpm_core::recover_operation_journal(&root).map_err(CommandError::from)?;
    Ok(recovery_report_dto(&root, report))
}

fn asset_dto(asset: Asset) -> AssetDto {
    AssetDto {
        id: asset.id,
        kind: asset.kind,
        relative_path: asset.relative_path,
        display_name: asset.display_name,
        state: asset.state,
        source_type: asset.source_type,
        generator: asset.generator,
        model: asset.model,
        prompt: asset.prompt,
        sha256: asset.sha256,
        size_bytes: asset.size_bytes,
        duration_ms: asset.duration_ms,
        width: asset.width,
        height: asset.height,
        version_group_id: asset.version_group_id,
        version_number: asset.version_number.unwrap_or(1),
        is_adopted: asset.is_adopted,
        created_at: asset.created_at.to_rfc3339(),
        updated_at: asset.updated_at.to_rfc3339(),
    }
}

fn asset_catalog_dto(project_path: &Path, catalog: AssetCatalog) -> AssetCatalogDto {
    let assets = catalog
        .assets
        .into_iter()
        .map(asset_dto)
        .collect::<Vec<_>>();
    let available = assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Available))
        .count();
    let missing = assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Missing))
        .count();
    let invalid = assets
        .iter()
        .filter(|asset| matches!(asset.state, AssetState::Error))
        .count();
    AssetCatalogDto {
        project_path: project_path.to_string_lossy().into_owned(),
        scanned_at: now_rfc3339(),
        total: assets.len(),
        available,
        missing,
        invalid,
        assets,
    }
}

fn recovery_report_dto(root: &Path, report: RecoveryReport) -> RecoveryReportDto {
    let journal_path = root.join(".ytpm-operation.json");
    let had_journal = report.journal_found;
    let recovered = report.journal_cleared;
    let message = if had_journal {
        Some(if recovered {
            "已驗證並清除 operation journal。".to_string()
        } else {
            "找到 operation journal，但尚未完成恢復。".to_string()
        })
    } else {
        Some("沒有待恢復的操作 journal。".to_string())
    };
    let actions = if had_journal {
        vec![
            "validated operation journal state".to_string(),
            if recovered {
                "cleared operation journal".to_string()
            } else {
                "left operation journal for manual recovery".to_string()
            },
        ]
    } else {
        Vec::new()
    };
    RecoveryReportDto {
        recovered,
        had_journal,
        journal_path: had_journal.then(|| journal_path.to_string_lossy().into_owned()),
        message,
        actions,
    }
}

fn now_rfc3339() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let seconds = duration.as_secs();
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;
    let milliseconds = duration.subsec_millis();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, i64, i64) {
    let adjusted = days_since_unix_epoch + 719_468;
    let era = if adjusted >= 0 {
        adjusted / 146_097
    } else {
        (adjusted - 146_096) / 146_097
    };
    let day_of_era = adjusted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_rfc3339_utc() {
        let timestamp = now_rfc3339();
        assert!(timestamp.ends_with('Z'));
        assert_eq!(timestamp.len(), 24);
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[10..11], "T");
    }

    #[test]
    fn invalid_input_maps_to_structured_command_error() {
        let error = CommandError::from(YtpmError::InvalidInput("bad input".into()));
        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.recoverable);
        assert!(error.technical_detail.is_some());
    }
}
