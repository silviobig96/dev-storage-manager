use dev_storage_application::{ApplicationService, MetadataScanRunner};
use dev_storage_persistence::Database;
use dev_storage_platform::{MacOsAdapter, PlatformAdapter};
use std::sync::Arc;
use tauri::Manager;

pub(crate) fn application_service(database: Database) -> ApplicationService<MacOsAdapter> {
    ApplicationService::new_with_scan_runner(
        MacOsAdapter,
        database,
        Arc::new(MetadataScanRunner::new(MacOsAdapter.scan_filesystem())),
    )
}

/// Request cancellation/publication stop without preventing exit or joining I/O.
pub(crate) fn on_run_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: &tauri::RunEvent) {
    if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
        if let Some(service) = app.try_state::<Arc<ApplicationService<MacOsAdapter>>>() {
            service.shutdown_scans();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspaces::tests::invoke;
    use serde_json::{json, Value};
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    // Catches normal setup retaining the unwired constructor despite working IPC/engine tests.
    #[cfg(target_os = "macos")]
    #[test]
    fn scan_native_integration_production_factory_and_registered_commands() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("selected");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("sentinel"), b"native-test").unwrap();
        let db = fixture.path().join("fixture.sqlite");
        let service = Arc::new(application_service(Database::open(&db).unwrap()));
        service
            .add_workspace(service.prepare_workspace(&root).unwrap(), 1)
            .unwrap();
        let before = std::fs::read(&db).unwrap();
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .manage(crate::workspace_runtime::WorkspaceRuntime::default())
            .invoke_handler(crate::application_handler())
            .build(crate::application_context())
            .unwrap();
        let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let call = |command, args| invoke(main.as_ref(), command, "tauri://localhost", args);
        let status = call("get_workspace_scan", json!({})).unwrap();
        assert_eq!(status["session"], Value::Null);
        let started = call("start_workspace_scan", json!({"request":{
            "requestId":"01234567-89ab-cdef-0123-456789abcdef", "expectedEpoch":status["epoch"], "workspaceIds":["1"], "mode":"quick"
        }})).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let done = loop {
            let snapshot = call(
                "get_workspace_scan_session",
                json!({"sessionId":started["sessionId"]}),
            )
            .unwrap();
            if snapshot["workerActive"] == false {
                break snapshot;
            }
            assert!(Instant::now() < deadline, "metadata worker timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(done["state"], "completed");
        assert_eq!(done["entryAttempts"], "2");
        assert_eq!(done["metrics"]["observedLogicalBytes"], "11");
        assert_eq!(done["metrics"]["uniqueFiles"], "1");
        assert_eq!(done["metrics"]["uniqueDirectories"], "1");
        assert_eq!(
            call("get_app_health", json!({})).unwrap()["databaseSchemaVersion"],
            2
        );
        assert_eq!(std::fs::read(&db).unwrap(), before);
        assert_eq!(
            std::fs::read(root.join("sentinel")).unwrap(),
            b"native-test"
        );
    }
}
