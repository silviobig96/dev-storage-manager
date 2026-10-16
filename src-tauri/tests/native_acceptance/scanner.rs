// Synthetic success exists only in this opt-in test executable, never the app.
use super::*;
use dev_storage_application::ScanRunner;
use dev_storage_core::{scanner::*, workspaces::*};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Condvar,
};

#[derive(Default)]
pub(super) struct CountedRunner {
    pub calls: AtomicUsize,
    released: Mutex<bool>,
    changed: Condvar,
}
impl CountedRunner {
    pub fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
impl ScanRunner for CountedRunner {
    fn run(
        &self,
        roots: Vec<StoredWorkspace>,
        _: ScanMode,
        _: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let metrics: ScanMetrics = serde_json::from_value(json!({
            "observedLogicalBytes":"9007199254740993", "observedAllocatedBytes":null,
            "uniqueFiles":"1", "uniqueDirectories":"1", "referencesDeduplicated":"0",
            "allocationMeasuredFiles":"0", "allocationUnknownFiles":"1", "policyExcludedEntries":"0",
            "unmeasuredEntriesLowerBound":"0", "unmeasuredBytes":null,
            "logicalCoverage":"partial", "allocatedCoverage":"unavailable"
        })).unwrap();
        let mut progress = ScanProgress {
            workspaces: roots
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
                .collect(),
            metrics,
            coverage: Coverage::Partial,
            diagnostics: vec![],
            entry_attempts: DecimalQuantity(3),
        };
        publish(progress.clone());
        let (released, timed) = self
            .changed
            .wait_timeout_while(
                self.released.lock().unwrap(),
                Duration::from_secs(10),
                |v| !*v,
            )
            .unwrap();
        assert!(
            *released && !timed.timed_out(),
            "synthetic control was not released"
        );
        for root in &mut progress.workspaces {
            root.state = WorkspaceScanState::Completed;
        }
        ScanReport {
            progress,
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    }
}

pub(super) async fn positive(fixture: &Fixture, main: &Webview) -> QaResult<()> {
    let runner = fixture.runner.as_ref().expect("scanner case runner");
    struct Release<'a>(&'a CountedRunner);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.release();
        }
    }
    let _release = Release(runner);
    let current = invoke(main, "get_workspace_scan", json!({}), "scan-positive").await?;
    require(
        current["status"] == "resolved"
            && current["value"]["epoch"].is_string()
            && current["value"]["session"].is_null(),
        "initial authoritative status",
    )?;
    let request = json!({"requestId":"01234567-89ab-cdef-0123-456789abcdef", "expectedEpoch":current["value"]["epoch"], "workspaceIds":["1"], "mode":"quick"});
    let start = invoke(
        main,
        "start_workspace_scan",
        json!({"request":request}),
        "scan-positive",
    )
    .await?;
    require(
        start["status"] == "resolved",
        format!("synthetic start: {start}"),
    )?;
    let session = start["value"]["sessionId"].clone();
    require(session.is_string(), "real accepted session ID")?;
    tokio::time::timeout(DEADLINE, async {
        while fixture
            .service
            .current_scan()
            .unwrap()
            .session
            .unwrap()
            .entry_attempts
            .0
            != 3
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("synthetic publication"))?;
    let query = invoke(
        main,
        "get_workspace_scan_session",
        json!({"sessionId":session}),
        "scan-positive",
    )
    .await?;
    require(
        query["status"] == "resolved"
            && query["value"]["entryAttempts"] == "3"
            && query["value"]["metrics"]["observedLogicalBytes"] == "9007199254740993",
        "synthetic session quantities",
    )?;
    let replay = invoke(
        main,
        "start_workspace_scan",
        json!({"request":request}),
        "scan-positive",
    )
    .await?;
    require(
        replay["status"] == "resolved" && replay["value"]["sessionId"] == session,
        "native original-epoch replay",
    )?;
    let cancel = invoke(
        main,
        "cancel_workspace_scan",
        json!({"sessionId":session}),
        "scan-positive",
    )
    .await?;
    require(
        cancel["status"] == "resolved"
            && cancel["value"]["state"] == "cancelling"
            && cancel["value"]["workerActive"] == true
            && cancel["value"]["finishedAt"].is_null(),
        "native live cancellation acknowledgement",
    )?;
    require(
        runner.calls.load(Ordering::SeqCst) == 1,
        "exactly one synthetic runner",
    )?;
    runner.release();
    tokio::time::timeout(DEADLINE, async {
        while fixture
            .service
            .current_scan()
            .unwrap()
            .session
            .unwrap()
            .worker_active
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("synthetic termination"))?;
    let terminal = invoke(main, "get_workspace_scan", json!({}), "scan-positive").await?;
    require(
        terminal["status"] == "resolved"
            && terminal["value"]["session"]["state"] == "cancelled"
            && terminal["value"]["session"]["workerActive"] == false,
        "authoritative terminal cancellation",
    )?;
    require(
        fixture.count("scan-positive:main:main") == 6,
        "six tagged scan positive handler entries",
    )?;
    eprintln!("PASS scan-positive commands=4 invokes=6 synthetic_runner_calls=1 logical_bytes=9007199254740993 entry_attempts=3 cancellation=active_then_terminal");
    Ok(())
}

pub(super) async fn picker_positive(fixture: &Fixture, main: &Webview) -> QaResult<()> {
    let before = fixture.snapshot()?;
    eprintln!(
        "NATIVE_PICKER_CANCEL_REQUIRED pid={} timeout_seconds=60",
        std::process::id()
    );
    eval(main, r#"(() => {
        window.__dsmNativeReport = null;
        const bridge = window.__TAURI_INTERNALS__;
        if (!bridge || typeof bridge.invoke !== 'function') { window.__dsmNativeReport = {status:'bridge_missing'}; return; }
        bridge.invoke('select_and_add_workspace', {}, {headers:{'x-dsm-native-case':'picker-positive'}})
            .then(value => window.__dsmNativeReport = {status:'resolved',value})
            .catch(error => window.__dsmNativeReport = {status:'rejected',error});
    })()"#).await?;
    let response = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let response = eval(main, "window.__dsmNativeReport || null").await?;
            if !response.is_null() {
                return Ok(response);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| QaFailure::Timeout("native picker cancellation"))??;
    if response["status"] == "bridge_missing" {
        return Err(QaFailure::BridgeMissing);
    }
    require(
        response == json!({"status":"resolved","value":{"status":"cancelled"}}),
        format!("actual native picker did not cancel: {response}"),
    )?;
    require(
        fixture.count("picker-positive:main:main") == 1 && fixture.snapshot()? == before,
        "native cancel handler entry and unchanged DB",
    )?;
    require(
        fixture.service.reserve_workspace_change().is_ok(),
        "native cancellation released application reservation",
    )?;
    eprintln!("PASS picker-positive native_cancel=true handler_entries=1 database=identical");
    Ok(())
}
