mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ytpm_core::MediaJobQueue::new())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::project_create,
            commands::project_list,
            commands::project_validate,
            commands::project_update_status,
            commands::project_archive,
            commands::project_restore,
            commands::project_migrate,
            commands::project_index_rebuild,
            commands::project_index_search,
            commands::task_list,
            commands::task_create,
            commands::task_update,
            commands::task_move,
            commands::asset_scan,
            commands::asset_list,
            commands::document_read,
            commands::document_write,
            commands::project_recover_journal,
            commands::timeline_load,
            commands::timeline_save,
            commands::media_probe,
            commands::media_export,
            commands::media_export_enqueue,
            commands::media_job_status,
            commands::media_job_list,
            commands::media_job_cancel,
            commands::media_operation_cancel,
            commands::publish_config_reference,
            commands::publish_auth_start,
            commands::publish_auth_callback,
            commands::publish_metadata_load,
            commands::publish_metadata_save,
            commands::publish_dry_run,
            commands::publish_upload,
            commands::publish_cancel
        ])
        .run(tauri::generate_context!())
        .expect("error while running YouTube Project Manager");
}
