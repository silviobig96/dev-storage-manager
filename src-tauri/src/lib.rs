mod commands;
mod registration;
mod scans;
mod workspace_runtime;
mod workspaces;

use std::{fs, sync::Arc};

use dev_storage_application::ApplicationService;
use dev_storage_persistence::Database;
use dev_storage_platform::MacOsAdapter;
use registration::{application_context, application_handler};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(workspace_runtime::WorkspaceRuntime::default())
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_directory)?;
            let database = Database::open(&app_data_directory.join("dev-storage-manager.db"))?;
            let service = ApplicationService::new(MacOsAdapter, database);
            app.manage(Arc::new(service));
            Ok(())
        })
        .invoke_handler(application_handler())
        .run(application_context())
        .expect("failed to run dev-storage-manager");
}
