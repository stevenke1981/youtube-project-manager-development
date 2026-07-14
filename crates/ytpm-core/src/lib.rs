pub mod asset_service;
pub mod document_service;
mod error;
mod folder_template;
pub mod index;
mod migration;
mod model;
pub mod project_service;
pub mod task_service;

pub use asset_service::{list_assets, scan_assets, Asset, AssetCatalog, AssetKind, AssetState};
pub use document_service::{read_document, write_document};
pub use error::{Result, YtpmError};
pub use folder_template::{expected_directories, template_files};
pub use index::{rebuild_index, search_index, IndexReport};
pub use migration::CURRENT_SCHEMA_VERSION;
pub use model::{CreateProjectRequest, Project, ProjectStatus, ValidationIssue, ValidationReport};
pub use project_service::{
    archive_project, create_project, list_projects, migrate_project, recover_operation_journal,
    restore_project, update_project_status, validate_project, RecoveryReport,
};
pub use task_service::{
    create_task, list_tasks, move_task, update_task, Task, TaskFile, TaskPatch, TaskPriority,
    TaskRequest, TaskStatus,
};
