mod acl;
mod geometry;
mod scan_smoke;
mod scan_ui;
mod scanner;

use dev_storage_application::ApplicationService;
use dev_storage_persistence::Database;
use dev_storage_platform::MacOsAdapter;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Manager, Webview, Wry};

const DEADLINE: Duration = Duration::from_secs(5);
const MARKER: &str = "DSM_NATIVE_ACCEPTANCE_ONLY_V1";

#[derive(Clone, Copy, Debug)]
pub enum NativeCase {
    InheritedAcl,
    InheritedLayout,
    ScannerAcl,
    ScanSmoke,
    ScanUi,
}

#[derive(Debug)]
pub enum QaFailure {
    Assertion(String),
    Transport(String),
    BridgeMissing,
    Timeout(&'static str),
    Navigation(String),
}
pub type QaResult<T> = Result<T, QaFailure>;
pub type NativeEvidence = Value;

fn require(ok: bool, message: impl Into<String>) -> QaResult<()> {
    if ok {
        Ok(())
    } else {
        Err(QaFailure::Assertion(message.into()))
    }
}
fn transport(error: impl std::fmt::Debug) -> QaFailure {
    QaFailure::Transport(format!("{error:?}"))
}

struct Fixture {
    app: AppHandle,
    service: Arc<ApplicationService<MacOsAdapter>>,
    entries: Arc<Mutex<BTreeMap<String, usize>>>,
    runner: Option<Arc<scanner::CountedRunner>>,
    ui_runner: Option<Arc<scan_ui::UiRunner>>,
    database_path: PathBuf,
    _directory: tempfile::TempDir,
}

#[derive(Default)]
struct LoadedPages(Mutex<BTreeMap<String, String>>);

impl Fixture {
    fn count(&self, caller: &str) -> usize {
        *self.entries.lock().unwrap().get(caller).unwrap_or(&0)
    }
    fn snapshot(&self) -> QaResult<(Value, Vec<u8>)> {
        Ok((
            serde_json::to_value(self.service.list_workspaces().map_err(transport)?)
                .map_err(transport)?,
            std::fs::read(&self.database_path).map_err(transport)?,
        ))
    }
    fn main(&self) -> QaResult<Webview> {
        self.app
            .get_webview("main")
            .ok_or_else(|| QaFailure::Navigation("main webview missing".into()))
    }
}

pub fn main() {
    eprintln!("{MARKER}");
    let args: Vec<_> = std::env::args().collect();
    let case = match args
        .windows(2)
        .find(|pair| pair[0] == "--case")
        .map(|pair| pair[1].as_str())
    {
        Some("inherited-acl") => NativeCase::InheritedAcl,
        Some("inherited-layout") => NativeCase::InheritedLayout,
        Some("scanner-acl") => NativeCase::ScannerAcl,
        Some("scan-smoke") => NativeCase::ScanSmoke,
        Some("scan-ui") => NativeCase::ScanUi,
        _ => {
            eprintln!(
                "FAIL: expected --case inherited-acl|inherited-layout|scanner-acl|scan-smoke|scan-ui"
            );
            std::process::exit(2);
        }
    };
    match run_case(case) {
        Ok(evidence) => println!("PASS {evidence}"),
        Err(error) => {
            eprintln!("FAIL {case:?}: {error:?}");
            std::process::exit(1);
        }
    }
}

pub fn run_case(case: NativeCase) -> QaResult<NativeEvidence> {
    let directory = tempfile::Builder::new()
        .prefix("dsm-native-acceptance-")
        .tempdir()
        .map_err(transport)?;
    let database_path = directory.path().join("fixture.sqlite");
    let database = Database::open(&database_path).map_err(transport)?;
    let runner =
        matches!(case, NativeCase::ScannerAcl).then(|| Arc::new(scanner::CountedRunner::default()));
    let ui_runner = matches!(case, NativeCase::ScanUi).then(|| Arc::new(scan_ui::UiRunner::new()));
    let service = Arc::new(match &runner {
        Some(runner) => {
            ApplicationService::new_with_scan_runner(MacOsAdapter, database, runner.clone())
        }
        None if ui_runner.is_some() => ApplicationService::new_with_scan_runner(
            MacOsAdapter,
            database,
            ui_runner.as_ref().unwrap().clone(),
        ),
        None if matches!(case, NativeCase::ScanSmoke) => {
            crate::native_runtime::application_service(database)
        }
        None => ApplicationService::new(MacOsAdapter, database),
    });
    for number in 1..=12 {
        let root = directory
            .path()
            .join(format!("workspace-{number:02}"))
            .join("long-location-for-native-layout-verification".repeat(3));
        std::fs::create_dir_all(&root).map_err(transport)?;
        service
            .add_workspace(service.prepare_workspace(&root).map_err(transport)?, 100)
            .map_err(transport)?;
        if number == 1 && matches!(case, NativeCase::ScanSmoke | NativeCase::ScanUi) {
            std::fs::create_dir(root.join("nested")).map_err(transport)?;
            std::fs::write(root.join("sentinel"), b"unchanged").map_err(transport)?;
            std::fs::write(root.join("nested/data"), b"abcde").map_err(transport)?;
        }
        if number == 2 && matches!(case, NativeCase::ScanUi) {
            let mut nested = root.clone();
            for _ in 0..10 {
                nested = nested.join("depth");
                std::fs::create_dir(&nested).map_err(transport)?;
            }
            std::fs::write(nested.join("sentinel"), b"depth-sentinel").map_err(transport)?;
        }
    }
    let entries = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
    let counted = entries.clone();
    let handler = crate::application_handler();
    let app = tauri::Builder::default()
        .manage(LoadedPages::default())
        .on_page_load(|webview, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                webview
                    .state::<LoadedPages>()
                    .0
                    .lock()
                    .unwrap()
                    .insert(webview.label().into(), payload.url().to_string());
            }
        })
        .manage(service.clone())
        .manage(crate::workspace_runtime::WorkspaceRuntime::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(move |invoke| {
            // Scope by the actual caller AND an explicit test invocation header.
            // React's legitimate health/list/check background calls never increment it.
            if let Some(id) = invoke
                .message
                .headers()
                .get("x-dsm-native-case")
                .and_then(|v| v.to_str().ok())
            {
                let caller = invoke.message.webview_ref();
                let key = format!("{id}:{}:{}", caller.window().label(), caller.label());
                *counted.lock().unwrap().entry(key).or_default() += 1;
            }
            handler(invoke)
        })
        .build(crate::application_context())
        .map_err(transport)?;
    let exit_service = service.clone();
    let fixture = Arc::new(Fixture {
        app: app.handle().clone(),
        service,
        entries,
        runner,
        ui_runner,
        database_path,
        _directory: directory,
    });
    let result = Arc::new(Mutex::new(None));
    let collected = result.clone();
    let mut pending = Some(fixture);
    let exit_code = app.run_return(move |app, event| {
        if matches!(case, NativeCase::ScanSmoke) {
            crate::native_runtime::on_run_event(app, &event);
        }
        match event {
            tauri::RunEvent::Ready => {
                let fixture = pending.take().expect("one Ready event");
                let app = app.clone();
                let collected = collected.clone();
                tauri::async_runtime::spawn(async move {
                    let outcome = match case {
                        NativeCase::InheritedAcl => acl::run(&fixture, false).await,
                        NativeCase::ScannerAcl => acl::run(&fixture, true).await,
                        NativeCase::InheritedLayout => geometry::run(&fixture).await,
                        NativeCase::ScanSmoke => scan_smoke::run(&fixture).await,
                        NativeCase::ScanUi => scan_ui::run(&fixture).await,
                    };
                    let code = if outcome.is_ok() { 0 } else { 1 };
                    *collected.lock().unwrap() = Some(outcome);
                    app.exit(code);
                });
            }
            // Closing/replacing main is part of the wrong-parent case, not test completion.
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } if !matches!(case, NativeCase::ScanSmoke) => api.prevent_exit(),
            _ => {}
        }
    });
    let outcome = result.lock().unwrap().take().unwrap_or_else(|| {
        Err(QaFailure::Transport(format!(
            "event loop exited ({exit_code}) before collecting evidence"
        )))
    });
    let evidence = outcome?;
    if matches!(case, NativeCase::ScanSmoke) {
        require(
            matches!(exit_service.current_scan(), Err(error)
                if error.code == dev_storage_core::scanner::ScanErrorCode::ApplicationUnavailable),
            "ExitRequested must stop scan publication",
        )?;
    }
    Ok(evidence)
}

async fn eval(webview: &Webview<Wry>, script: impl Into<String>) -> QaResult<Value> {
    // Wry 0.55 queues scripts before its first navigation completes, discarding
    // their callbacks. Wait for the native Finished notification first.
    tokio::time::timeout(DEADLINE, async {
        loop {
            if webview
                .state::<LoadedPages>()
                .0
                .lock()
                .unwrap()
                .contains_key(webview.label())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("native page load"))?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = Mutex::new(Some(tx));
    webview
        .eval_with_callback(script, move |result| {
            if let Some(tx) = sender.lock().unwrap().take() {
                let _ = tx.send(result);
            }
        })
        .map_err(transport)?;
    let serialized = tokio::time::timeout(DEADLINE, rx)
        .await
        .map_err(|_| QaFailure::Timeout("eval callback"))?
        .map_err(transport)?;
    if serialized.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&serialized).map_err(transport)
}

async fn wait_for(
    webview: &Webview,
    expression: &str,
    description: &'static str,
) -> QaResult<Value> {
    tokio::time::timeout(DEADLINE, async {
        loop {
            let value = eval(webview, expression).await?;
            if !value.is_null() && value != false {
                return Ok(value);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout(description))?
}

async fn ready(webview: &Webview) -> QaResult<()> {
    wait_for(
        webview,
        "document.readyState === 'complete' && !!document.querySelector('.app-shell')",
        "frontend navigation",
    )
    .await?;
    Ok(())
}

async fn invoke(webview: &Webview, command: &str, args: Value, case: &str) -> QaResult<Value> {
    invoke_using(webview, command, args, case, "window.__TAURI_INTERNALS__").await
}

async fn invoke_using(
    webview: &Webview,
    command: &str,
    args: Value,
    case: &str,
    bridge: &str,
) -> QaResult<Value> {
    let script = format!(
        r#"(() => {{
        window.__dsmNativeReport = null;
        const bridge = {bridge};
        if (!bridge || typeof bridge.invoke !== 'function') {{ window.__dsmNativeReport = {{status:'bridge_missing'}}; return; }}
        bridge.invoke({command}, {args}, {options})
            .then(value => window.__dsmNativeReport = {{status:'resolved', value}})
            .catch(error => window.__dsmNativeReport = {{status:'rejected', error}});
    }})()"#,
        command = serde_json::to_string(command).unwrap(),
        options = json!({"headers":{"x-dsm-native-case":case}})
    );
    eval(webview, script).await?;
    let value = wait_for(
        webview,
        "window.__dsmNativeReport || null",
        "invoke promise",
    )
    .await?;
    if value["status"] == "bridge_missing" {
        return Err(QaFailure::BridgeMissing);
    }
    Ok(value)
}
