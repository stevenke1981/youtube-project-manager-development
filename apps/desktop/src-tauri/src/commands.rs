use serde::Serialize;
use std::path::Path;
use ytpm_core::{CreateProjectRequest, Project, ProjectStatus, ValidationReport, YtpmError};

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
            YtpmError::Json(_) => ("JSON_INVALID", false, Some("從備份恢復 project.json")),
            YtpmError::InvalidProject(_) => (
                "INVALID_PROJECT",
                true,
                Some("先執行 validate 或 migrate，再重試操作"),
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

#[tauri::command]
pub async fn project_create(
    root_path: String,
    request: CreateProjectRequest,
) -> Result<Project, CommandError> {
    ytpm_core::create_project(Path::new(&root_path), request).map_err(CommandError::from)
}

#[tauri::command]
pub async fn project_list(root_path: String) -> Result<Vec<Project>, CommandError> {
    ytpm_core::list_projects(Path::new(&root_path)).map_err(CommandError::from)
}

#[tauri::command]
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

#[tauri::command]
pub async fn project_restore(project_path: String) -> Result<Project, CommandError> {
    ytpm_core::restore_project(Path::new(&project_path)).map_err(CommandError::from)
}

#[tauri::command]
pub async fn project_migrate(project_path: String) -> Result<Project, CommandError> {
    ytpm_core::migrate_project(Path::new(&project_path)).map_err(CommandError::from)
}
