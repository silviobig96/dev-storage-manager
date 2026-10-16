mod commands;
mod workspace_runtime;
mod workspaces;

use std::{fs, sync::Arc};

use dev_storage_application::ApplicationService;
use dev_storage_persistence::Database;
use dev_storage_platform::MacOsAdapter;
use tauri::Manager;

fn application_context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

fn application_handler<R: tauri::Runtime>(
) -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::get_app_health,
        workspaces::list_workspaces,
        workspaces::select_and_add_workspace,
        workspaces::remove_workspace,
        workspaces::set_workspace_enabled,
        workspaces::check_workspace,
        workspaces::skip_workspace_setup
    ]
}

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
