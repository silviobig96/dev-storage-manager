mod commands;

use std::{fs, sync::Mutex};

use dev_storage_application::ApplicationService;
use dev_storage_persistence::Database;
use dev_storage_platform::MacOsAdapter;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_directory)?;
            let database = Database::open(&app_data_directory.join("dev-storage-manager.db"))?;
            let service = ApplicationService::new(MacOsAdapter, database);
            app.manage(Mutex::new(service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_app_health])
        .run(tauri::generate_context!())
        .expect("failed to run dev-storage-manager");
}
