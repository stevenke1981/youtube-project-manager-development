mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::project_create,
            commands::project_list,
            commands::project_validate,
            commands::project_update_status,
            commands::project_archive,
            commands::project_restore,
            commands::project_migrate
        ])
        .run(tauri::generate_context!())
        .expect("error while running YouTube Project Manager");
}
