//! Native rendering of real measurements, then explicitly synthetic held work.
use super::*;
use dev_storage_application::{MetadataScanRunner, ScanRunner};
use dev_storage_core::{scanner::*, workspaces::StoredWorkspace};
use dev_storage_platform::PlatformAdapter;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Condvar,
};

pub(super) struct UiRunner {
    real: MetadataScanRunner,
    calls: AtomicUsize,
    released: Mutex<bool>,
    changed: Condvar,
}
impl UiRunner {
    pub fn new() -> Self {
        Self {
            real: MetadataScanRunner::new(MacOsAdapter.scan_filesystem()),
            calls: AtomicUsize::new(0),
            released: Mutex::new(false),
            changed: Condvar::new(),
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
impl ScanRunner for UiRunner {
    fn run(
        &self,
        roots: Vec<StoredWorkspace>,
        mode: ScanMode,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let mut report = self.real.run(
            roots,
            if call == 2 { ScanMode::Deep } else { mode },
            control,
            publish,
        );
        if call != 2 {
            return report;
        }
        // This third invocation is a controlled state exercise, NOT OS blocked I/O.
        report.progress.metrics.observed_logical_bytes = DecimalQuantity(9_007_199_254_740_993);
        report.progress.metrics.observed_allocated_bytes = None;
        report.progress.metrics.allocated_coverage = Coverage::Unavailable;
        report.progress.metrics.logical_coverage = Coverage::Partial;
        report.progress.coverage = Coverage::Partial;
        control.request_stop(StopReason::TimeBudget);
        publish(report.progress.clone());
        let _ = self
            .changed
            .wait_timeout_while(
                self.released.lock().unwrap(),
                Duration::from_secs(30),
                |v| !*v,
            )
            .unwrap();
        report.state = ScanState::Partial;
        report.stop_reason = Some(StopReason::TimeBudget);
        report
    }
}

async fn click(main: &Webview, text: &str) -> QaResult<()> {
    let literal = serde_json::to_string(text).unwrap();
    require(eval(main, format!("(() => {{ const b=[...document.querySelectorAll('button')].find(b=>b.textContent==={literal}); if(!b || b.disabled)return false; b.click(); return true; }})()")).await? == true, format!("enabled {text} control"))
}

async fn terminal(fixture: &Fixture, mode: &str) -> QaResult<Value> {
    tokio::time::timeout(DEADLINE, async {
        loop {
            let status = serde_json::to_value(fixture.service.current_scan().map_err(transport)?)
                .map_err(transport)?;
            if status["session"].is_object()
                && status["session"]["workerActive"] == false
                && status["session"]["mode"] == mode
            {
                return Ok(status["session"].clone());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("UI scan terminal"))?
}

async fn recover_page(main: &Webview) -> QaResult<()> {
    let old = eval(main, "performance.timeOrigin").await?;
    eval(main, "location.reload()").await?;
    wait_for(main,&format!("performance.timeOrigin > {old} && document.readyState==='complete' && !!document.querySelector('.app-shell')"),"fresh native mount recovery").await?;
    click(main, "Workspaces").await?;
    wait_for(main,"document.querySelectorAll('.workspace-row').length===12 && !document.querySelector('.scan-summary')?.textContent.includes('Checking scan status')","recovered workspace UI").await?;
    Ok(())
}

pub(super) async fn run(fixture: &Fixture) -> QaResult<Value> {
    let main = fixture.main()?;
    fixture
        .app
        .set_activation_policy(tauri::ActivationPolicy::Regular)
        .map_err(transport)?;
    fixture.app.show().map_err(transport)?;
    main.window().show().map_err(transport)?;
    main.window().set_focus().map_err(transport)?;
    ready(&main).await?;
    let snapshot_rendering = std::env::args().any(|arg| arg == "--snapshot-rendering");
    eprintln!("FOREGROUND READY pid={}", std::process::id());
    if !snapshot_rendering {
        tokio::time::timeout(Duration::from_secs(60), async {
            while eval(&main, "document.visibilityState==='visible'").await? != true {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok::<(), QaFailure>(())
        })
        .await
        .map_err(|_| QaFailure::Timeout("foreground native frontend"))??;
    }
    click(&main, "Workspaces").await?;
    wait_for(&main, "document.querySelectorAll('.workspace-row').length === 12 && [...document.querySelectorAll('button')].some(b=>b.textContent==='Quick scan'&&!b.disabled)", "scan controls").await?;
    let before = fixture.snapshot()?;
    let mut real = Vec::new();
    for (button, mode, expected) in [
        ("Quick scan", "quick", "partial"),
        ("Deep scan", "deep", "completed"),
    ] {
        let started = std::time::Instant::now();
        click(&main, button).await?;
        let done = terminal(fixture, mode).await?;
        if snapshot_rendering {
            recover_page(&main).await?;
        }
        require(
            done["mode"] == mode && done["state"] == expected,
            format!("real UI state {done}"),
        )?;
        let message = if mode == "quick" {
            "Partial measurement"
        } else {
            "Scan completed within the declared scope."
        };
        let rendered = wait_for(
            &main,
            &format!(
                "document.querySelector('.scan-summary')?.textContent.includes({})",
                serde_json::to_string(message).unwrap()
            ),
            "rendered real summary",
        )
        .await;
        if rendered.is_err() {
            eprintln!(
                "RENDER_DIAGNOSTIC mode={mode} state={} summary={}",
                done["state"],
                eval(
                    &main,
                    "document.querySelector('.scan-summary')?.textContent"
                )
                .await?
            );
        }
        rendered?;
        require(
            done["metrics"]["observedLogicalBytes"] == if mode == "quick" { "14" } else { "28" },
            "hand-counted real logical bytes",
        )?;
        require(
            if mode == "quick" {
                done["stopReason"].is_null()
                    && done["diagnostics"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|d| d["code"] == "depth_limit")
            } else {
                done["stopReason"].is_null()
            },
            format!(
                "real stop reason {} diagnostics {}",
                done["stopReason"], done["diagnostics"]
            ),
        )?;
        real.push(json!({"mode":mode,"durationMillis":started.elapsed().as_millis(),"state":done["state"],"stopReason":done["stopReason"],"workerActive":done["workerActive"],"entryAttempts":done["entryAttempts"],"metrics":done["metrics"]}));
    }
    let geometry = geometry::populated_matrix(&main).await?;
    click(&main, "Workspaces").await?;
    // Real WK dialog/focus, activated through DOM. This does not claim keyboard input.
    eval(&main, "(() => { const b=document.querySelector('.workspace-actions button:last-child'); b.focus(); window.__dsmInvoker=b; b.click(); })()").await?;
    wait_for(
        &main,
        "document.querySelector('dialog')?.open && document.activeElement?.textContent==='Cancel'",
        "dialog Cancel focus",
    )
    .await?;
    require(eval(&main, "document.querySelector('dialog').textContent.includes('Files on disk will not be deleted.')").await? == true, "dialog safety copy")?;
    click(&main, "Cancel").await?;
    wait_for(
        &main,
        "!document.querySelector('dialog') && document.activeElement===window.__dsmInvoker",
        "dialog invoker restoration",
    )
    .await?;
    let runner = fixture.ui_runner.as_ref().unwrap();
    struct Release<'a>(&'a UiRunner);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.release();
        }
    }
    let _release = Release(runner);
    click(&main, "Quick scan").await?;
    if snapshot_rendering {
        tokio::time::timeout(DEADLINE, async {
            loop {
                let status =
                    serde_json::to_value(fixture.service.current_scan().map_err(transport)?)
                        .map_err(transport)?;
                if status["session"]["state"] == "stopping" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Ok::<(), QaFailure>(())
        })
        .await
        .map_err(|_| QaFailure::Timeout("controlled stopping"))??;
        recover_page(&main).await?;
    }
    wait_for(&main, "document.querySelector('.scan-summary')?.textContent.includes('Waiting for a filesystem operation to finish.')", "controlled stopping presentation").await?;
    require(eval(&main, "(() => { const s=document.querySelector('.scan-summary'); return s.textContent.includes('9,007,199,254,740,993 bytes') && s.textContent.includes('Not available') && [...document.querySelectorAll('.scan-controls button')].filter(b=>b.textContent.endsWith(' scan')&&b.textContent!=='Cancel scan').every(b=>b.disabled) && [...document.querySelectorAll('.workspace-enabled input')].every(b=>b.disabled); })()").await? == true, "stopping large quantity/null/write lock")?;
    let stopping = serde_json::to_value(fixture.service.current_scan().map_err(transport)?)
        .map_err(transport)?;
    require(
        stopping["session"]["state"] == "stopping" && stopping["session"]["workerActive"] == true,
        "controlled live worker",
    )?;
    runner.release();
    let stopped = terminal(fixture, "quick").await?;
    if snapshot_rendering {
        recover_page(&main).await?;
    }
    require(
        stopped["state"] == "partial"
            && stopped["workerActive"] == false
            && stopped["stopReason"] == "time_budget",
        "controlled terminal partial",
    )?;
    wait_for(&main, "[...document.querySelectorAll('button')].some(b=>b.textContent==='Deep scan'&&!b.disabled)", "gate released controls").await?;
    require(fixture.snapshot()? == before, "UI scan DB unchanged")?;
    let roots = fixture
        .service
        .list_workspaces()
        .map_err(transport)?
        .workspaces;
    require(
        std::fs::read(std::path::Path::new(&roots[0].selected_path).join("sentinel"))
            .map_err(transport)?
            == b"unchanged",
        "real fixture sentinel unchanged",
    )?;
    let mut deep = PathBuf::from(&roots[1].selected_path);
    for _ in 0..10 {
        deep = deep.join("depth");
    }
    require(
        std::fs::read(deep.join("sentinel")).map_err(transport)? == b"depth-sentinel",
        "deep sentinel unchanged",
    )?;
    let evidence = json!({"case":"scan-ui","snapshotRenderingOnly":snapshot_rendering,"visibility":eval(&main,"document.visibilityState").await?,"activation":"DOM on actual Wry; not native keyboard/pointer","realMetadata":real,"geometry":geometry,"controlledSynthetic":{"state":stopping["session"]["state"],"workerActive":true,"logicalBytes":"9007199254740993","allocatedBytes":null,"stopReason":"time_budget","terminal":stopped["state"],"terminalWorkerActive":false},"dialog":"DOM activated native dialog: Cancel focus and invoker restoration passed","database":"identical","sentinels":"unchanged","humanVisualQa":"AWAITING HUMAN REVIEW"});
    eprintln!("AUTOMATIC_SCAN_UI {evidence}");
    if std::env::args().any(|arg| arg == "--hold") {
        eprintln!("INTERACTIVE READY pid={} Phase03 controlled fixture; terminal partial synthetic summary; next Quick/Deep use real metadata; no personal DB. Ten minute review window.", std::process::id());
        tokio::time::sleep(Duration::from_secs(600)).await;
    }
    Ok(evidence)
}
