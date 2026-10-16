use dev_storage_application::ApplicationService;
use dev_storage_core::scanner::{
    ScanErrorCode, ScanFailure, ScanRequest, ScanSessionId, ScanSnapshot, ScanStatus,
};
use dev_storage_platform::MacOsAdapter;
use std::sync::Arc;
use tauri::{Runtime, State, Webview};

fn unavailable() -> ScanFailure {
    ScanFailure {
        code: ScanErrorCode::ApplicationUnavailable,
    }
}

async fn scan_job<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ScanFailure> + Send + 'static,
) -> Result<T, ScanFailure> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| unavailable())?
}

#[tauri::command]
pub async fn start_workspace_scan<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    request: ScanRequest,
) -> Result<ScanSnapshot, ScanFailure> {
    crate::workspaces::ensure_main(&webview).map_err(|_| unavailable())?;
    let service = state.inner().clone();
    scan_job(move || service.start_scan(request)).await
}

#[tauri::command]
pub async fn get_workspace_scan<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
) -> Result<ScanStatus, ScanFailure> {
    crate::workspaces::ensure_main(&webview).map_err(|_| unavailable())?;
    let service = state.inner().clone();
    scan_job(move || service.current_scan()).await
}

#[tauri::command]
pub async fn get_workspace_scan_session<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    session_id: ScanSessionId,
) -> Result<ScanSnapshot, ScanFailure> {
    crate::workspaces::ensure_main(&webview).map_err(|_| unavailable())?;
    let service = state.inner().clone();
    scan_job(move || service.scan_session(&session_id)).await
}

#[tauri::command]
pub async fn cancel_workspace_scan<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    session_id: ScanSessionId,
) -> Result<ScanSnapshot, ScanFailure> {
    crate::workspaces::ensure_main(&webview).map_err(|_| unavailable())?;
    let service = state.inner().clone();
    scan_job(move || service.cancel_scan(&session_id)).await
}

#[cfg(test)]
mod tests {
    use crate::workspaces::tests::invoke;
    use dev_storage_application::{ApplicationService, ScanRunner};
    use dev_storage_core::{scanner::*, workspaces::*};
    use dev_storage_persistence::Database;
    use dev_storage_platform::MacOsAdapter;
    use serde_json::{json, Value};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    };
    use std::time::Duration;

    struct Runner {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
        calls: AtomicUsize,
    }
    impl ScanRunner for Runner {
        fn run(
            &self,
            roots: Vec<StoredWorkspace>,
            mode: ScanMode,
            _: &ScanControl,
            publish: &mut dyn FnMut(ScanProgress),
        ) -> ScanReport {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].id.get(), 1);
            assert_eq!(mode, ScanMode::Quick);
            let metrics: ScanMetrics = serde_json::from_value(json!({
                    "observedLogicalBytes":"9007199254740993", "observedAllocatedBytes":null,
                    "uniqueFiles":"1", "uniqueDirectories":"1", "referencesDeduplicated":"0",
                    "allocationMeasuredFiles":"0", "allocationUnknownFiles":"1", "policyExcludedEntries":"0",
                    "unmeasuredEntriesLowerBound":"0", "unmeasuredBytes":null,
                    "logicalCoverage":"partial", "allocatedCoverage":"unavailable"
            })).unwrap();
            let workspaces = roots
                .iter()
                .map(|r| WorkspaceScanResult {
                    workspace_id: r.id,
                    display_name: WorkspaceRecord::from(r).display_name,
                    selected_path: r.root.selected_path.clone(),
                    state: WorkspaceScanState::Running,
                    metrics: Some(metrics.clone()),
                    coverage: Coverage::Partial,
                    covered_by_workspace_ids: vec![],
                })
                .collect();
            let progress = ScanProgress {
                workspaces,
                metrics,
                coverage: Coverage::Partial,
                diagnostics: vec![],
                entry_attempts: DecimalQuantity(3),
            };
            publish(progress.clone());
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            ScanReport {
                progress,
                state: ScanState::Completed,
                stop_reason: None,
                failure: None,
            }
        }
    }

    // Catches wrong request nesting, widened typed inputs, facade-only replay, and lossy quantities.
    #[test]
    fn scan_ipc_contract() {
        let (entered, started) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let runner = Arc::new(Runner {
            entered,
            release: Mutex::new(released),
            calls: AtomicUsize::new(0),
        });
        let service = Arc::new(ApplicationService::new_with_scan_runner(
            MacOsAdapter,
            Database::open_in_memory().unwrap(),
            runner.clone(),
        ));
        service
            .add_workspace(
                ValidatedWorkspace {
                    selected_path: "/controlled/fixture".into(),
                    canonical_path: "/controlled/fixture".into(),
                    identity: DirectoryIdentity::new("fixture"),
                },
                1,
            )
            .unwrap();
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
        assert_eq!(
            call("get_app_health", json!({})).unwrap()["databaseSchemaVersion"],
            2
        );
        let request = json!({"requestId":"01234567-89ab-cdef-0123-456789abcdef", "expectedEpoch":status["epoch"], "workspaceIds":["1"], "mode":"quick"});
        assert!(call("start_workspace_scan", request.clone())
            .unwrap_err()
            .as_str()
            .unwrap()
            .contains("invalid args"));
        for (field, value) in [
            ("path", json!("/never/read")),
            ("paths", json!(["/never/read"])),
            ("options", json!({})),
            ("budget", json!(1)),
            ("maxEntryAttempts", json!(1)),
            ("mode", json!("turbo")),
            ("workspaceIds", json!([1])),
            ("workspaceIds", json!(["01"])),
            ("expectedEpoch", json!("p01-t1-g0")),
            ("requestId", json!("not-a-token")),
        ] {
            let mut invalid = request.clone();
            invalid[field] = value;
            let error = call("start_workspace_scan", json!({"request":invalid})).unwrap_err();
            assert!(
                error.as_str().unwrap().contains("invalid args"),
                "{field}: {error}"
            );
        }
        for command in ["get_workspace_scan_session", "cancel_workspace_scan"] {
            for id in [json!(1), json!("s:p1-t1-g0:0"), json!("/private/path")] {
                assert!(call(command, json!({"sessionId":id}))
                    .unwrap_err()
                    .as_str()
                    .unwrap()
                    .contains("invalid args"));
            }
            assert_eq!(
                call(command, json!({"sessionId":"s:p1-t1-g0:1"})).unwrap_err()["code"],
                "session_not_found"
            );
        }
        assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
        let first = call("start_workspace_scan", json!({"request":request})).unwrap();
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        let session = first["sessionId"].clone();
        assert_eq!(
            call("select_and_add_workspace", json!({})).unwrap_err()["code"],
            "scan_in_progress"
        );
        let current = call("get_workspace_scan", json!({})).unwrap();
        assert_eq!(current["session"]["sessionId"], session);
        assert_eq!(current["session"]["entryAttempts"], "3");
        assert_eq!(
            current["session"]["metrics"]["observedLogicalBytes"],
            "9007199254740993"
        );
        assert!(current["session"]["revision"].is_string());
        assert!(current["session"]["startedAt"].is_string());
        assert_eq!(
            call("start_workspace_scan", json!({"request":request})).unwrap()["sessionId"],
            session
        );
        let mut other = request.clone();
        other["requestId"] = json!("11234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(
            call("start_workspace_scan", json!({"request":other})).unwrap_err()["code"],
            "stale_epoch"
        );
        let mut conflicting = request.clone();
        conflicting["mode"] = json!("deep");
        assert_eq!(
            call("start_workspace_scan", json!({"request":conflicting})).unwrap_err()["code"],
            "request_conflict"
        );
        let cancelled = call("cancel_workspace_scan", json!({"sessionId":session})).unwrap();
        assert_eq!(cancelled["state"], "cancelling");
        assert_eq!(cancelled["workerActive"], true);
        assert_eq!(cancelled["finishedAt"], Value::Null);
        assert_eq!(
            call("get_workspace_scan_session", json!({"sessionId":session})).unwrap(),
            cancelled
        );
        assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            service
                .remove_workspace(WorkspaceId::try_from(1).unwrap())
                .unwrap_err()
                .code,
            WorkspaceErrorCode::ScanInProgress
        );
        release.send(()).unwrap();
    }
}
