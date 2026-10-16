use crate::workspace_runtime::{database_job, failure, WorkspaceRuntime};
use dev_storage_application::ApplicationService;
use dev_storage_core::workspaces::*;
use dev_storage_platform::{MacOsAdapter, PlatformAdapter};
use serde::Serialize;
use std::{path::PathBuf, sync::Arc};
use tauri::{Runtime, State, Webview};
use tauri_plugin_dialog::DialogExt;

type PickResult = Result<Option<PathBuf>, WorkspaceFailure>;
type PickCallback = Box<dyn FnOnce(PickResult) + Send>;
trait WorkspacePicker {
    fn pick(&self, callback: PickCallback);
}

struct NativePicker<R: Runtime> {
    window: tauri::Window<R>,
}
impl<R: Runtime> WorkspacePicker for NativePicker<R> {
    fn pick(&self, callback: PickCallback) {
        self.window
            .dialog()
            .file()
            .set_parent(&self.window)
            .set_title("Add workspace")
            .set_can_create_directories(false)
            .pick_folder(move |selected| {
                callback(
                    selected
                        .map(|path| {
                            path.into_path()
                                .map_err(|_| failure(WorkspaceErrorCode::InvalidSelection))
                        })
                        .transpose(),
                );
            });
    }
}

pub(crate) fn ensure_main<R: Runtime>(webview: &Webview<R>) -> Result<(), WorkspaceFailure> {
    if webview.label() == "main" && webview.window().label() == "main" {
        Ok(())
    } else {
        Err(failure(WorkspaceErrorCode::ApplicationUnavailable))
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AddWorkspaceOutcome {
    Cancelled,
    Added {
        workspace: WorkspaceRecord,
        availability: AvailabilityObservation,
    },
    Duplicate {
        existing_id: WorkspaceId,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveWorkspaceResult {
    pub removed_id: WorkspaceId,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupWorkspaceResult {
    pub setup_state: SetupState,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckWorkspaceResult {
    pub id: WorkspaceId,
    pub availability: AvailabilityObservation,
}

async fn select_with_picker<A: PlatformAdapter + 'static>(
    service: Arc<ApplicationService<A>>,
    runtime: &WorkspaceRuntime,
    picker: &impl WorkspacePicker,
    clock: impl FnOnce() -> i64,
) -> Result<AddWorkspaceOutcome, WorkspaceFailure> {
    let selection = runtime.try_begin_selection()?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    picker.pick(Box::new(move |result| {
        // The callback owns the gate while the panel is open, even if its receiver is gone.
        let _ = sender.send((result, selection));
    }));
    let (selected, selection) = receiver
        .await
        .map_err(|_| failure(WorkspaceErrorCode::ApplicationUnavailable))?;
    let Some(selected) = selected? else {
        return Ok(AddWorkspaceOutcome::Cancelled);
    };
    let preparing = service.clone();
    let root = runtime
        .validate(move || preparing.prepare_workspace(&selected))
        .await?;
    // Validation cannot persist; only a timely success reaches this untimed commit.
    let now = clock();
    match selection
        .commit(move || service.add_workspace(root, now))
        .await?
    {
        WorkspaceInsertOutcome::Added(workspace) => Ok(AddWorkspaceOutcome::Added {
            workspace,
            availability: AvailabilityObservation {
                state: AvailabilityState::Present,
                reason: None,
                checked_at: Some(now),
            },
        }),
        WorkspaceInsertOutcome::Duplicate(existing_id) => {
            Ok(AddWorkspaceOutcome::Duplicate { existing_id })
        }
    }
}

#[tauri::command]
pub async fn list_workspaces<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
) -> Result<WorkspaceListing, WorkspaceFailure> {
    ensure_main(&webview)?;
    let service = state.inner().clone();
    database_job(move || service.list_workspaces()).await
}

#[tauri::command]
pub async fn select_and_add_workspace<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    runtime: State<'_, WorkspaceRuntime>,
) -> Result<AddWorkspaceOutcome, WorkspaceFailure> {
    ensure_main(&webview)?;
    select_with_picker(
        state.inner().clone(),
        runtime.inner(),
        &NativePicker {
            window: webview.window(),
        },
        now_ms,
    )
    .await
}

#[tauri::command]
pub async fn remove_workspace<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    id: WorkspaceId,
) -> Result<RemoveWorkspaceResult, WorkspaceFailure> {
    ensure_main(&webview)?;
    let service = state.inner().clone();
    database_job(move || {
        service.remove_workspace(id)?;
        Ok(RemoveWorkspaceResult { removed_id: id })
    })
    .await
}

#[tauri::command]
pub async fn set_workspace_enabled<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    id: WorkspaceId,
    enabled: bool,
) -> Result<WorkspaceRecord, WorkspaceFailure> {
    ensure_main(&webview)?;
    let service = state.inner().clone();
    database_job(move || service.set_workspace_enabled(id, enabled, now_ms())).await
}

async fn check_with_runtime<A: PlatformAdapter + 'static>(
    service: Arc<ApplicationService<A>>,
    runtime: &WorkspaceRuntime,
    id: WorkspaceId,
    now: i64,
) -> Result<CheckWorkspaceResult, WorkspaceFailure> {
    let availability = match runtime
        .validate(move || service.check_workspace(id, now))
        .await
    {
        Ok(observation) => observation,
        Err(error) if error.code == WorkspaceErrorCode::ValidationTimeout => {
            AvailabilityObservation {
                state: AvailabilityState::Unknown,
                reason: Some(AvailabilityReason::Timeout),
                checked_at: Some(now),
            }
        }
        Err(error) => return Err(error),
    };
    Ok(CheckWorkspaceResult { id, availability })
}

#[tauri::command]
pub async fn check_workspace<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
    runtime: State<'_, WorkspaceRuntime>,
    id: WorkspaceId,
) -> Result<CheckWorkspaceResult, WorkspaceFailure> {
    ensure_main(&webview)?;
    check_with_runtime(state.inner().clone(), runtime.inner(), id, now_ms()).await
}

#[tauri::command]
pub async fn skip_workspace_setup<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
) -> Result<SetupWorkspaceResult, WorkspaceFailure> {
    ensure_main(&webview)?;
    let service = state.inner().clone();
    database_job(move || {
        service
            .skip_workspace_setup()
            .map(|setup_state| SetupWorkspaceResult { setup_state })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use dev_storage_core::Platform;
    use dev_storage_persistence::Database;
    use std::future::Future;
    use std::{
        path::Path,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Condvar, Mutex,
        },
        time::Duration,
    };

    #[derive(Default)]
    struct FakePicker {
        callback: Mutex<Option<PickCallback>>,
    }
    impl WorkspacePicker for FakePicker {
        fn pick(&self, callback: PickCallback) {
            *self.callback.lock().unwrap() = Some(callback);
        }
    }
    impl FakePicker {
        fn complete(&self, result: PickResult) {
            self.callback.lock().unwrap().take().unwrap()(result);
        }
    }
    #[derive(Clone, Default)]
    struct Adapter {
        validations: Arc<AtomicUsize>,
        barrier: Option<Arc<Barrier>>,
    }
    #[derive(Default)]
    struct Barrier {
        released: Mutex<bool>,
        changed: Condvar,
        completed: AtomicUsize,
    }
    impl Barrier {
        fn wait(&self) {
            drop(
                self.changed
                    .wait_while(self.released.lock().unwrap(), |done| !*done)
                    .unwrap(),
            );
            self.completed.fetch_add(1, Ordering::SeqCst);
        }
        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.changed.notify_all();
        }
    }
    struct ReleaseOnDrop(Arc<Barrier>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            self.0.release();
        }
    }
    impl PlatformAdapter for Adapter {
        fn platform(&self) -> Platform {
            Platform::MacOs
        }
        fn validate_workspace_root(
            &self,
            selected: &Path,
        ) -> Result<ValidatedWorkspace, WorkspaceFailure> {
            self.validations.fetch_add(1, Ordering::SeqCst);
            if let Some(barrier) = &self.barrier {
                barrier.wait();
            }
            Ok(ValidatedWorkspace {
                selected_path: selected.to_str().unwrap().into(),
                canonical_path: selected.to_str().unwrap().into(),
                identity: DirectoryIdentity::new("fixture"),
            })
        }
        fn check_workspace_root(
            &self,
            _: &ValidatedWorkspace,
            now: i64,
        ) -> AvailabilityObservation {
            if let Some(barrier) = &self.barrier {
                barrier.wait();
            }
            AvailabilityObservation {
                state: AvailabilityState::Present,
                reason: None,
                checked_at: Some(now),
            }
        }
    }
    fn service(adapter: Adapter) -> Arc<ApplicationService<Adapter>> {
        Arc::new(ApplicationService::new(
            adapter,
            Database::open_in_memory().unwrap(),
        ))
    }
    struct Selected;
    impl WorkspacePicker for Selected {
        fn pick(&self, cb: PickCallback) {
            cb(Ok(Some("/controlled/fake-root".into())));
        }
    }

    // Catches saving after cancel/error, failing to serialize picks, and early gate release on receiver drop.
    #[test]
    fn workspace_transport_cancel_and_busy() {
        tauri::async_runtime::block_on(async {
            let adapter = Adapter::default();
            let service = service(adapter.clone());
            let runtime = WorkspaceRuntime::default();
            let picker = FakePicker::default();
            for result in [Ok(None), Err(failure(WorkspaceErrorCode::InvalidSelection))] {
                let mut pending =
                    Box::pin(select_with_picker(service.clone(), &runtime, &picker, || 1));
                assert!(
                    std::future::poll_fn(|cx| std::task::Poll::Ready(
                        pending.as_mut().poll(cx).is_pending()
                    ))
                    .await
                );
                assert_eq!(
                    select_with_picker(service.clone(), &runtime, &picker, || 1)
                        .await
                        .unwrap_err()
                        .code,
                    WorkspaceErrorCode::OperationInProgress
                );
                let cancelled = result.is_ok();
                picker.complete(result);
                let outcome = pending.await;
                if cancelled {
                    assert_eq!(
                        serde_json::to_value(outcome.unwrap()).unwrap(),
                        serde_json::json!({"status":"cancelled"})
                    );
                } else {
                    assert_eq!(
                        outcome.unwrap_err().code,
                        WorkspaceErrorCode::InvalidSelection
                    );
                }
                assert!(runtime.try_begin_selection().is_ok());
            }
            let mut pending =
                Box::pin(select_with_picker(service.clone(), &runtime, &picker, || 1));
            assert!(
                std::future::poll_fn(|cx| std::task::Poll::Ready(
                    pending.as_mut().poll(cx).is_pending()
                ))
                .await
            );
            drop(pending);
            assert!(runtime.try_begin_selection().is_err());
            picker.complete(Ok(Some(PathBuf::from("/controlled/fake-root"))));
            assert!(runtime.try_begin_selection().is_ok());
            assert_eq!(adapter.validations.load(Ordering::SeqCst), 0);
            let listing = service.list_workspaces().unwrap();
            assert!(listing.workspaces.is_empty());
            assert_eq!(listing.setup_state, SetupState::Undecided);
        });
    }

    // Catches omitted persistence, duplicate ID/casing drift and leaked private root metadata.
    #[test]
    fn workspace_transport_contract() {
        tauri::async_runtime::block_on(async {
            let service = service(Adapter::default());
            let runtime = WorkspaceRuntime::default();
            let first = select_with_picker(service.clone(), &runtime, &Selected, || 100)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(first).unwrap(),
                serde_json::json!({"status":"added","workspace":{"id":"1","selectedPath":"/controlled/fake-root","displayName":"fake-root","enabled":true,"createdAt":100,"updatedAt":100},"availability":{"state":"present","reason":null,"checkedAt":100}})
            );
            let second = select_with_picker(service.clone(), &runtime, &Selected, || 200)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(second).unwrap(),
                serde_json::json!({"status":"duplicate","existingId":"1"})
            );
            assert_eq!(service.list_workspaces().unwrap().workspaces.len(), 1);
        });
    }

    // Catches late insertion, timeout exceptions for checks, and mutation of saved configuration.
    #[test]
    fn workspace_transport_deadline() {
        tauri::async_runtime::block_on(async {
            let barrier = Arc::new(Barrier::default());
            let _release = ReleaseOnDrop(barrier.clone());
            let service = service(Adapter {
                barrier: Some(barrier.clone()),
                ..Default::default()
            });
            let runtime = WorkspaceRuntime::new(Duration::from_millis(10));
            assert_eq!(
                select_with_picker(service.clone(), &runtime, &Selected, || 1)
                    .await
                    .unwrap_err()
                    .code,
                WorkspaceErrorCode::ValidationTimeout
            );
            assert!(service.list_workspaces().unwrap().workspaces.is_empty());
            let root = ValidatedWorkspace {
                selected_path: "/controlled/known".into(),
                canonical_path: "/controlled/known".into(),
                identity: DirectoryIdentity::new("known"),
            };
            let WorkspaceInsertOutcome::Added(saved) = service.add_workspace(root, 2).unwrap()
            else {
                panic!("expected fixture insertion")
            };
            let before = service.list_workspaces().unwrap();
            let checked = check_with_runtime(service.clone(), &runtime, saved.id, 3)
                .await
                .unwrap();
            assert_eq!(
                serde_json::to_value(checked).unwrap(),
                serde_json::json!({"id":"1","availability":{"state":"unknown","reason":"timeout","checkedAt":3}})
            );
            assert_eq!(
                select_with_picker(service.clone(), &runtime, &Selected, || 4)
                    .await
                    .unwrap_err()
                    .code,
                WorkspaceErrorCode::OperationInProgress
            );
            barrier.release();
            tokio::time::timeout(Duration::from_secs(2), async {
                while barrier.completed.load(Ordering::SeqCst) != 2 {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
            assert_eq!(service.list_workspaces().unwrap(), before);
            assert!(runtime.try_begin_selection().is_ok());
        });
    }

    // Catches applying a metadata deadline to a commit that already changed configuration.
    #[test]
    fn workspace_transport_commit_returns_actual_result() {
        tauri::async_runtime::block_on(async {
            let service = service(Adapter::default());
            let commit = database_job(move || {
                std::thread::sleep(Duration::from_millis(25));
                service.skip_workspace_setup()
            });
            assert_eq!(commit.await.unwrap(), SetupState::Skipped);
        });
    }

    // Catches app commands accidentally escaping generated production ACL, including OR window grants.
    #[test]
    fn workspace_acl_denies_other_webviews_and_remote_origins() {
        use tauri::{Manager, WebviewWindowBuilder};
        let service = Arc::new(ApplicationService::new(
            dev_storage_platform::MacOsAdapter,
            Database::open_in_memory().unwrap(),
        ));
        let calls = Arc::new(AtomicUsize::new(0));
        let entries = calls.clone();
        let handler = crate::application_handler();
        let runtime = WorkspaceRuntime::default();
        let _open_panel = runtime.try_begin_selection().unwrap();
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .manage(runtime)
            .plugin(tauri_plugin_dialog::init())
            .invoke_handler(move |invoke| {
                entries.fetch_add(1, Ordering::SeqCst);
                handler(invoke)
            })
            .build(crate::application_context())
            .unwrap();
        let main = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        assert!(invoke(
            main.as_ref(),
            "get_app_health",
            "tauri://localhost",
            serde_json::json!({})
        )
        .is_ok());
        let second = main
            .as_ref()
            .window()
            .add_child(
                tauri::webview::WebviewBuilder::new("second", Default::default()),
                tauri::LogicalPosition::new(0., 0.),
                tauri::LogicalSize::new(100., 100.),
            )
            .unwrap();
        let other = WebviewWindowBuilder::new(&app, "other", Default::default())
            .build()
            .unwrap();
        calls.store(0, Ordering::SeqCst);
        for command in [
            "get_app_health",
            "list_workspaces",
            "select_and_add_workspace",
            "remove_workspace",
            "set_workspace_enabled",
            "check_workspace",
            "skip_workspace_setup",
        ] {
            for (caller, url) in [
                (&second, "tauri://localhost"),
                (other.as_ref(), "tauri://localhost"),
                (main.as_ref(), "https://untrusted.example"),
            ] {
                let error = invoke(
                    caller,
                    command,
                    url,
                    serde_json::json!({"id":"1","enabled":false,"window":"main","webview":"main"}),
                )
                .unwrap_err();
                assert!(
                    error.as_str().unwrap().contains("not allowed"),
                    "{command}: {error}"
                );
            }
        }
        // ACL rejection precedes even the registered handler: picker and service cannot be called.
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        for command in [
            "plugin:dialog|open",
            "plugin:dialog|save",
            "plugin:fs|read_file",
            "plugin:shell|execute",
        ] {
            assert!(invoke(
                main.as_ref(),
                command,
                "tauri://localhost",
                serde_json::json!({})
            )
            .is_err());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(service.list_workspaces().unwrap().workspaces.is_empty());
        assert_eq!(
            service.list_workspaces().unwrap().setup_state,
            SetupState::Undecided
        );
        assert_eq!(app.webviews().len(), 3);
    }

    // Catches trusting the allowed webview label while ignoring its actual containing window.
    #[test]
    fn workspace_acl_checks_actual_parent_window() {
        let service = Arc::new(ApplicationService::new(
            MacOsAdapter,
            Database::open_in_memory().unwrap(),
        ));
        let runtime = WorkspaceRuntime::default();
        let _open_panel = runtime.try_begin_selection().unwrap();
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .manage(runtime)
            .invoke_handler(crate::application_handler())
            .build(crate::application_context())
            .unwrap();
        let window = tauri::WindowBuilder::new(&app, "ungranted")
            .build()
            .unwrap();
        let caller = window
            .add_child(
                tauri::webview::WebviewBuilder::new("main", Default::default()),
                tauri::LogicalPosition::new(0., 0.),
                tauri::LogicalSize::new(100., 100.),
            )
            .unwrap();
        for command in [
            "get_app_health",
            "list_workspaces",
            "select_and_add_workspace",
            "remove_workspace",
            "set_workspace_enabled",
            "check_workspace",
            "skip_workspace_setup",
        ] {
            let error = invoke(
                &caller,
                command,
                "tauri://localhost",
                serde_json::json!({"id":"1","enabled":false,"window":"main"}),
            )
            .unwrap_err();
            assert_eq!(
                error["code"], "application_unavailable",
                "{command}: {error}"
            );
        }
        assert_eq!(
            service.list_workspaces().unwrap().setup_state,
            SetupState::Undecided
        );
    }

    // Catches bad command registration, weak typed inputs, stale-ID mutation, and DTO field drift.
    #[test]
    fn workspace_transport_ipc_contract() {
        use serde_json::json;
        let service = Arc::new(ApplicationService::new(
            MacOsAdapter,
            Database::open_in_memory().unwrap(),
        ));
        let runtime = WorkspaceRuntime::default();
        let _open_panel = runtime.try_begin_selection().unwrap();
        let app = tauri::test::mock_builder()
            .manage(service.clone())
            .manage(runtime)
            .invoke_handler(crate::application_handler())
            .build(crate::application_context())
            .unwrap();
        let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let call = |command, args| invoke(main.as_ref(), command, "tauri://localhost", args);
        assert_eq!(
            call("get_app_health", json!({})).unwrap()["databaseSchemaVersion"],
            2
        );
        assert_eq!(
            call("list_workspaces", json!({})).unwrap(),
            json!({"setupState":"undecided","workspaces":[],"overlaps":[]})
        );
        assert_eq!(
            call("skip_workspace_setup", json!({})).unwrap(),
            json!({"setupState":"skipped"})
        );
        // Neither a path nor caller-supplied labels are accepted inputs to the add workflow.
        assert_eq!(
            call(
                "select_and_add_workspace",
                json!({"path":"/never/validate/from/js","window":"main","webview":"main"})
            )
            .unwrap_err(),
            json!({"code":"operation_in_progress","message":"Another workspace operation is in progress."})
        );
        assert!(service.list_workspaces().unwrap().workspaces.is_empty());
        let root = ValidatedWorkspace {
            selected_path: "/controlled/fake-root".into(),
            canonical_path: "/controlled/fake-root".into(),
            identity: DirectoryIdentity::new("fake"),
        };
        service.add_workspace(root.clone(), 100).unwrap();
        let before = service.list_workspaces().unwrap();
        for id in [
            json!(1),
            json!("0"),
            json!("01"),
            json!("-1"),
            json!("../root"),
            json!(null),
            json!("9223372036854775808"),
        ] {
            for command in [
                "remove_workspace",
                "check_workspace",
                "set_workspace_enabled",
            ] {
                assert!(call(command, json!({"id":id,"enabled":false}))
                    .unwrap_err()
                    .as_str()
                    .unwrap()
                    .contains("invalid args"));
            }
        }
        for enabled in [json!("false"), json!(0), json!(null)] {
            assert!(
                call("set_workspace_enabled", json!({"id":"1","enabled":enabled}))
                    .unwrap_err()
                    .as_str()
                    .unwrap()
                    .contains("invalid args")
            );
        }
        assert_eq!(service.list_workspaces().unwrap(), before);
        let disabled = call("set_workspace_enabled", json!({"id":"1","enabled":false})).unwrap();
        assert_eq!(disabled["enabled"], false);
        assert_eq!(disabled["id"], "1");
        assert!(disabled.get("updatedAt").is_some());
        assert_eq!(
            call("remove_workspace", json!({"id":"1"})).unwrap(),
            json!({"removedId":"1"})
        );
        for command in [
            "remove_workspace",
            "check_workspace",
            "set_workspace_enabled",
        ] {
            assert_eq!(
                call(command, json!({"id":"1","enabled":true})).unwrap_err(),
                json!({"code":"workspace_not_found","message":"This workspace is no longer configured."})
            );
        }
        let WorkspaceInsertOutcome::Added(readded) = service.add_workspace(root, 200).unwrap()
        else {
            panic!("expected insert")
        };
        assert_eq!(readded.id.get(), 2);
        assert_eq!(
            call("skip_workspace_setup", json!({})).unwrap(),
            json!({"setupState":"configured"})
        );
    }

    fn invoke(
        webview: &tauri::Webview<tauri::test::MockRuntime>,
        command: &str,
        url: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        struct Caller<'a>(&'a tauri::Webview<tauri::test::MockRuntime>);
        impl AsRef<tauri::Webview<tauri::test::MockRuntime>> for Caller<'_> {
            fn as_ref(&self) -> &tauri::Webview<tauri::test::MockRuntime> {
                self.0
            }
        }
        tauri::test::get_ipc_response(
            &Caller(webview),
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: url.parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .map(|body| body.deserialize().unwrap())
    }
}
