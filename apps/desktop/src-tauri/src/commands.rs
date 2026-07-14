use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ytpm_core::{
    Asset, AssetCatalog, AssetState, CreateProjectRequest, Project, ProjectStatus, RecoveryReport,
    Task, TaskPatch, TaskRequest, TaskStatus, ValidationReport, YtpmError,
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
