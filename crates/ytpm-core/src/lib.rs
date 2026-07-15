pub mod asset_service;
pub mod document_service;
mod error;
pub mod filter_graph;
mod folder_template;
pub mod index;
pub mod job_service;
pub mod media_service;
mod migration;
mod model;
pub mod project_service;
pub mod publish_service;
pub mod subtitle_service;
pub mod task_service;
pub mod timeline_service;

pub use asset_service::{list_assets, scan_assets, Asset, AssetCatalog, AssetKind, AssetState};
pub use document_service::{read_document, write_document};
pub use error::{Result, YtpmError};
pub use folder_template::{expected_directories, template_files};
pub use index::{rebuild_index, search_index, IndexReport};
pub use job_service::{MediaJobKind, MediaJobQueue, MediaJobRecord, MediaQueueJobStatus};
pub use media_service::{
    cancel_marker, export_timeline, export_timeline_controlled, probe_media, MediaExportResult,
    MediaJobStatus, MediaProbe, MediaStream,
};
pub use migration::CURRENT_SCHEMA_VERSION;
pub use model::{CreateProjectRequest, Project, ProjectStatus, ValidationIssue, ValidationReport};
pub use project_service::{
    archive_project, create_project, list_projects, migrate_project, recover_operation_journal,
    restore_project, update_project_status, validate_project, RecoveryReport,
};
pub use publish_service::{
    complete_oauth, config_reference, dry_run as publish_dry_run,
    load_metadata as load_publish_metadata, save_metadata as save_publish_metadata, start_oauth,
    upload_video, OAuthCallbackResult, OAuthStart, PublishCheck, PublishConfigReference,
    PublishJobStatus, PublishMetadata, PublishReadiness, PublishResult, PublishVisibility,
};
pub use task_service::{
    create_task, list_tasks, move_task, update_task, Task, TaskFile, TaskPatch, TaskPriority,
    TaskRequest, TaskStatus,
};
pub use timeline_service::{
    add_clip, read_timeline, remove_clip, render_manifest, update_clip, validate_timeline,
    write_timeline, Clip, ClipEffect, RenderManifest, RenderManifestClip, RenderSettings,
    SubtitleStyle, Timeline, TimelineClip, TimelineClipPatch, TimelineClipRequest,
    TimelineIssueSeverity, TimelineTrack, TimelineTrackKind, TimelineValidationIssue,
    TimelineValidationReport, TIMELINE_SCHEMA_VERSION,
};
