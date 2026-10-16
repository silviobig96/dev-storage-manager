use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};
use tauri::{
    LogicalPosition, LogicalSize, WebviewBuilder, WebviewUrl, WebviewWindowBuilder, WindowBuilder,
};

const COMMANDS: [&str; 7] = [
    "get_app_health",
    "list_workspaces",
    "select_and_add_workspace",
    "remove_workspace",
    "set_workspace_enabled",
    "check_workspace",
    "skip_workspace_setup",
];

// Regression caught: broadening a capability to a child/window/remote caller,
// or removing the actual-parent guard, would reach a service/picker or mutate DB.
pub(super) async fn run(fixture: &Fixture) -> QaResult<Value> {
    let main = fixture.main()?;
    ready(&main).await?;
    for command in ["get_app_health", "list_workspaces"] {
        let response = invoke(&main, command, json!({}), "allowed").await?;
        require(
            response["status"] == "resolved",
            format!("allowed main {command}: {response}"),
        )?;
        if command == "get_app_health" {
            require(
                response["value"]["databaseSchemaVersion"] == 2,
                "health contract",
            )?;
        } else {
            require(
                response["value"]["workspaces"].as_array().map(Vec::len) == Some(12),
                "seeded list contract",
            )?;
        }
    }
    require(
        fixture.count("allowed:main:main") == 2,
        "positive control must reach real handler",
    )?;
    for (command, args) in [
        ("set_workspace_enabled", json!({"id":"12","enabled":false})),
        ("set_workspace_enabled", json!({"id":"12","enabled":true})),
        ("check_workspace", json!({"id":"12"})),
        ("skip_workspace_setup", json!({})),
        ("remove_workspace", json!({"id":"12"})),
    ] {
        let response = invoke(&main, command, args, "mutation-control").await?;
        require(
            response["status"] == "resolved",
            format!("fixture mutation {command}: {response}"),
        )?;
    }
    require(
        fixture
            .service
            .list_workspaces()
            .map_err(transport)?
            .workspaces
            .len()
            == 11,
        "real fixture mutation positive control",
    )?;
    // Invalid transport and a never-settled promise must fail, never look denied.
    require(
        matches!(
            invoke_using(&main, "get_app_health", json!({}), "invalid", "undefined").await,
            Err(QaFailure::BridgeMissing)
        ),
        "missing bridge must be a test failure",
    )?;
    eval(&main, "window.__dsmNativeReport = null").await?;
    require(
        matches!(
            wait_for(
                &main,
                "window.__dsmNativeReport",
                "deliberately unsettled promise"
            )
            .await,
            Err(QaFailure::Timeout(_))
        ),
        "unsettled promise must be a test failure",
    )?;

    let child = main
        .window()
        .add_child(
            WebviewBuilder::new("ungranted-child", WebviewUrl::default()),
            LogicalPosition::new(0., 0.),
            LogicalSize::new(400., 300.),
        )
        .map_err(transport)?;
    ready(&child).await?;
    denied(fixture, &child, "child", false).await?;
    child.close().map_err(transport)?;
    let other = WebviewWindowBuilder::new(&fixture.app, "other", WebviewUrl::default())
        .build()
        .map_err(transport)?;
    ready(other.as_ref()).await?;
    denied(fixture, other.as_ref(), "other", false).await?;
    other.close().map_err(transport)?;

    {
        let server = OriginServer::start()?;
        main.state::<LoadedPages>().0.lock().unwrap().remove("main");
        main.navigate(server.url.parse().map_err(transport)?)
            .map_err(transport)?;
        wait_for(
            &main,
            "window.__dsmOriginReady === 'DSM_NATIVE_ACCEPTANCE_ONLY_V1'",
            "loopback origin navigation",
        )
        .await?;
        require(
            main.url().map_err(transport)?.as_str() == server.url,
            "actual main must navigate to external loopback origin",
        )?;
        denied(fixture, &main, "external", false).await?;
    } // Stop and join server before the next isolated caller case.
    main.window().close().map_err(transport)?;
    main.state::<LoadedPages>().0.lock().unwrap().remove("main");
    tokio::time::timeout(DEADLINE, async {
        while fixture.app.get_webview("main").is_some() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("closing main before wrong-parent case"))?;
    let wrong_window = WindowBuilder::new(&fixture.app, "wrong-parent")
        .build()
        .map_err(transport)?;
    let wrong = wrong_window
        .add_child(
            WebviewBuilder::new("main", WebviewUrl::default()),
            LogicalPosition::new(0., 0.),
            LogicalSize::new(600., 400.),
        )
        .map_err(transport)?;
    ready(&wrong).await?;
    denied(fixture, &wrong, "wrong-parent", true).await?;
    wrong_window.close().map_err(transport)?;
    Ok(
        json!({"case":"inherited-acl", "engine":"actual Wry", "allowedHealthAndList":2, "fixtureMutations":5,
        "deniedBeforeHandler":{"child":7,"otherWindow":7,"externalMain":7}, "wrongParentGuard":7,
        "missingBridgeIsFailure":true,"unsettledPromiseIsFailure":true,"unchangedFixtureDatabase":true,"scanner":"not implemented"}),
    )
}

async fn denied(fixture: &Fixture, caller: &Webview, case: &str, guard: bool) -> QaResult<()> {
    let before = fixture.snapshot()?;
    let key = format!("{case}:{}:{}", caller.window().label(), caller.label());
    let initial = fixture.count(&key);
    for command in COMMANDS {
        let response = invoke(caller, command, json!({"id":"1","enabled":false}), case).await?;
        require(
            response["status"] == "rejected",
            format!("{case} {command} did not reject: {response}"),
        )?;
        if guard {
            require(
                response["error"]["code"] == "application_unavailable",
                format!("{case} {command} did not hit actual-parent guard: {response}"),
            )?;
        } else {
            require(
                response["error"]
                    .as_str()
                    .is_some_and(|error| error.contains("not allowed")),
                format!("{case} {command} was not ACL rejection: {response}"),
            )?;
        }
        require(
            fixture.snapshot()? == before,
            format!("{case} {command} changed fixture database"),
        )?;
    }
    require(
        fixture.count(&key) - initial == if guard { 7 } else { 0 },
        format!("{case}: unexpected handler entries"),
    )?;
    eprintln!(
        "PASS caller={case} commands=7 handler_entries={} database=identical",
        fixture.count(&key) - initial
    );
    Ok(())
}

struct OriginServer {
    url: String,
    stopped: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl OriginServer {
    fn start() -> QaResult<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(transport)?;
        listener.set_nonblocking(true).map_err(transport)?;
        let url = format!("http://{}/", listener.local_addr().map_err(transport)?);
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let worker = thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
                        let mut request = [0; 4096];
                        let _ = stream.read(&mut request);
                        let body = include_str!("origin.html");
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            url,
            stopped,
            worker: Some(worker),
        })
    }
}
impl Drop for OriginServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
