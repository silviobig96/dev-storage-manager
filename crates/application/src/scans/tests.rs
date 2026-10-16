use super::*;
use dev_storage_core::{workspaces::*, Platform};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Mutex,
    },
    time::Duration,
};

mod scan_native_integration;

fn wait_scan(service: &ApplicationService<Adapter>) -> ScanSnapshot {
    let state = service.scans.shared.state.lock().unwrap();
    let (state, timed) = service
        .scans
        .shared
        .changed
        .wait_timeout_while(state, Duration::from_secs(5), |s| {
            s.retained
                .as_ref()
                .is_some_and(|r| r.snapshot.worker_active)
        })
        .unwrap();
    assert!(!timed.timed_out(), "worker did not terminate");
    state.retained.as_ref().unwrap().snapshot.clone()
}

#[derive(Default)]
struct Adapter;
impl PlatformAdapter for Adapter {
    fn platform(&self) -> Platform {
        Platform::MacOs
    }
    fn scan_filesystem(&self) -> Box<dyn ScanFilesystem> {
        panic!("unwired platform")
    }
    fn validate_workspace_root(&self, _: &Path) -> Result<ValidatedWorkspace, WorkspaceFailure> {
        panic!("no picker")
    }
    fn check_workspace_root(&self, _: &ValidatedWorkspace, now: i64) -> AvailabilityObservation {
        AvailabilityObservation {
            state: AvailabilityState::Present,
            reason: None,
            checked_at: Some(now),
        }
    }
}
fn root(path: &str) -> ValidatedWorkspace {
    ValidatedWorkspace {
        selected_path: path.into(),
        canonical_path: path.into(),
        identity: DirectoryIdentity::new(path),
    }
}
fn add(service: &ApplicationService<Adapter>, path: &str) -> WorkspaceId {
    match service.add_workspace(root(path), 1).unwrap() {
        WorkspaceInsertOutcome::Added(row) => row.id,
        _ => panic!("duplicate"),
    }
}
fn request(
    service: &ApplicationService<Adapter>,
    ids: Vec<WorkspaceId>,
    number: u32,
) -> ScanRequest {
    ScanRequest {
        request_id: ScanRequestId::try_from(format!("{number:08x}-1111-1111-1111-111111111111"))
            .unwrap(),
        expected_epoch: service.current_scan().unwrap().epoch,
        workspace_ids: ids,
        mode: ScanMode::Quick,
    }
}
#[derive(Default)]
struct NeverRun(AtomicUsize);
impl ScanRunner for NeverRun {
    fn run(
        &self,
        _: Vec<StoredWorkspace>,
        _: ScanMode,
        _: &ScanControl,
        _: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("rejected scope must not start work")
    }
}

fn metrics() -> ScanMetrics {
    ScanMetrics {
        observed_logical_bytes: DecimalQuantity(3),
        observed_allocated_bytes: Some(DecimalQuantity(8)),
        unique_files: DecimalQuantity(1),
        unique_directories: DecimalQuantity(1),
        references_deduplicated: DecimalQuantity(0),
        allocation_measured_files: DecimalQuantity(1),
        allocation_unknown_files: DecimalQuantity(0),
        policy_excluded_entries: DecimalQuantity(0),
        unmeasured_entries_lower_bound: DecimalQuantity(0),
        unmeasured_bytes: Some(DecimalQuantity(0)),
        logical_coverage: Coverage::CompleteWithinPolicy,
        allocated_coverage: Coverage::CompleteWithinPolicy,
    }
}
fn progress(roots: &[StoredWorkspace]) -> ScanProgress {
    ScanProgress {
        workspaces: roots
            .iter()
            .map(|r| WorkspaceScanResult {
                workspace_id: r.id,
                display_name: WorkspaceRecord::from(r).display_name,
                selected_path: r.root.selected_path.clone(),
                state: WorkspaceScanState::Completed,
                metrics: Some(metrics()),
                coverage: Coverage::CompleteWithinPolicy,
                covered_by_workspace_ids: vec![],
            })
            .collect(),
        metrics: metrics(),
        coverage: Coverage::CompleteWithinPolicy,
        diagnostics: vec![],
        entry_attempts: DecimalQuantity(2),
    }
}
struct BlockedRunner {
    entered: mpsc::Sender<Vec<WorkspaceId>>,
    release: Mutex<mpsc::Receiver<()>>,
    publish_running: bool,
    calls: AtomicUsize,
}
impl ScanRunner for BlockedRunner {
    fn run(
        &self,
        roots: Vec<StoredWorkspace>,
        _: ScanMode,
        _: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.publish_running {
            let mut update = progress(&roots);
            update.workspaces[0].state = WorkspaceScanState::Running;
            publish(update);
        }
        self.entered
            .send(roots.iter().map(|r| r.id).collect())
            .unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        ScanReport {
            progress: progress(&roots),
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    }
}
fn blocked(
    publish_running: bool,
) -> (
    ApplicationService<Adapter>,
    Arc<BlockedRunner>,
    mpsc::Receiver<Vec<WorkspaceId>>,
    mpsc::Sender<()>,
) {
    let (entered, observed) = mpsc::channel();
    let (release, receive) = mpsc::channel();
    let runner = Arc::new(BlockedRunner {
        entered,
        release: Mutex::new(receive),
        publish_running,
        calls: AtomicUsize::new(0),
    });
    let service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        runner.clone(),
    );
    (service, runner, observed, release)
}

// Catches reentrant add, foreign guards and writes during native work or accepted cancellation.
#[test]
fn scan_activity_serializes_picker_writes_and_actual_stop() {
    let (service, _, entered, release) = blocked(true);
    let a = add(&service, "/fixture/a");
    let picker = service.reserve_workspace_change().unwrap();
    let req = request(&service, vec![a], 1);
    assert_eq!(
        service.start_scan(req.clone()).unwrap_err().code,
        ScanErrorCode::WorkspaceOperationInProgress
    );
    let b = match service
        .commit_workspace_add(picker, root("/fixture/b"), 1)
        .unwrap()
    {
        WorkspaceInsertOutcome::Added(row) => row.id,
        _ => panic!("duplicate"),
    };
    let accepted = service.start_scan(req).unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    for cancel in [false, true] {
        if cancel {
            assert_eq!(
                service.cancel_scan(&accepted.session_id).unwrap().state,
                ScanState::Cancelling
            );
        }
        assert_eq!(
            service.reserve_workspace_change().err().unwrap().code,
            WorkspaceErrorCode::ScanInProgress
        );
        assert_eq!(
            service
                .add_workspace(root("/fixture/c"), 1)
                .unwrap_err()
                .code,
            WorkspaceErrorCode::ScanInProgress
        );
        assert_eq!(
            service.remove_workspace(a).unwrap_err().code,
            WorkspaceErrorCode::ScanInProgress
        );
        assert_eq!(
            service.set_workspace_enabled(a, false, 2).unwrap_err().code,
            WorkspaceErrorCode::ScanInProgress
        );
        assert_eq!(
            service.skip_workspace_setup().unwrap_err().code,
            WorkspaceErrorCode::ScanInProgress
        );
        assert_eq!(service.health().unwrap().database_schema_version, 2);
        assert_eq!(service.list_workspaces().unwrap().workspaces.len(), 2);
        assert_eq!(service.check_workspace(a, 3).unwrap().checked_at, Some(3));
    }
    let cancelling = service.current_scan().unwrap().session.unwrap();
    assert!(cancelling.worker_active);
    assert!(cancelling.finished_at.is_none());
    release.send(()).unwrap();
    let terminal = wait_scan(&service);
    assert_eq!(terminal.state, ScanState::Cancelled);
    assert!(!terminal.worker_active);
    assert!(terminal.finished_at.is_some());
    assert_eq!(terminal.coverage, Coverage::Partial);
    assert_eq!(terminal.entry_attempts, DecimalQuantity(2));
    assert!(terminal
        .diagnostics
        .iter()
        .any(|d| d.code == ScanDiagnosticCode::Cancelled));
    service.remove_workspace(b).unwrap();
    let foreign = ApplicationService::new(Adapter, Database::open_in_memory().unwrap());
    let foreign_guard = foreign.reserve_workspace_change().unwrap();
    assert_eq!(
        service
            .commit_workspace_add(foreign_guard, root("/foreign"), 1)
            .unwrap_err()
            .code,
        WorkspaceErrorCode::ApplicationUnavailable
    );
    assert_eq!(service.list_workspaces().unwrap().workspaces.len(), 1);
}

// Catches normalizing away accepted payload, replay after epoch/gate and replay restarting work.
#[test]
fn scan_idempotent_start_replays_original_payload_before_busy_and_epoch() {
    let (service, runner, entered, release) = blocked(false);
    let a = add(&service, "/fixture/a");
    let b = add(&service, "/fixture/b");
    let req = request(&service, vec![b, a], 1);
    let accepted = service.start_scan(req.clone()).unwrap();
    assert_eq!(accepted.state, ScanState::Validating);
    assert_ne!(accepted.epoch, req.expected_epoch);
    assert_eq!(
        entered.recv_timeout(Duration::from_secs(5)).unwrap(),
        vec![a, b]
    );
    assert_eq!(
        service.start_scan(req.clone()).unwrap().session_id,
        accepted.session_id
    );
    let mut changed = req.clone();
    changed.workspace_ids.reverse();
    assert_eq!(
        service.start_scan(changed).unwrap_err().code,
        ScanErrorCode::RequestConflict
    );
    assert_eq!(
        service
            .start_scan(request(&service, vec![a], 2))
            .unwrap_err()
            .code,
        ScanErrorCode::ScanBusy
    );
    service.cancel_scan(&accepted.session_id).unwrap();
    assert_eq!(
        service.start_scan(req.clone()).unwrap().state,
        ScanState::Cancelling
    );
    release.send(()).unwrap();
    wait_scan(&service);
    assert_eq!(
        service.start_scan(req.clone()).unwrap().state,
        ScanState::Cancelled
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
    let replacement = service.start_scan(request(&service, vec![a], 2)).unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        service.start_scan(req.clone()).unwrap_err().code,
        ScanErrorCode::StaleEpoch
    );
    assert_eq!(
        service.scan_session(&accepted.session_id).unwrap_err().code,
        ScanErrorCode::SessionNotFound
    );
    release.send(()).unwrap();
    let complete = wait_scan(&service);
    assert_eq!(complete.state, ScanState::Completed);
    assert_eq!(
        service.cancel_scan(&replacement.session_id).unwrap(),
        complete
    );
    let restarted = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        runner.clone(),
    );
    assert!(restarted.current_scan().unwrap().session.is_none());
    assert_ne!(restarted.current_scan().unwrap().epoch, complete.epoch);
    assert_eq!(
        restarted.start_scan(req).unwrap_err().code,
        ScanErrorCode::StaleEpoch
    );
    assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
}

// Catches scope broadening, accepting disabled/duplicates, and shipped fake success.
#[test]
fn scan_scope_resolves_ids_before_runner() {
    let runner = Arc::new(NeverRun::default());
    let service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        runner.clone(),
    );
    let a = add(&service, "/fixture/a");
    let b = add(&service, "/fixture/b");
    service.set_workspace_enabled(b, false, 2).unwrap();
    for (ids, code) in [
        (vec![], ScanErrorCode::ScopeEmpty),
        (vec![a, a], ScanErrorCode::InvalidRequest),
        (vec![a; 33], ScanErrorCode::ScopeTooLarge),
        (
            vec![WorkspaceId::try_from(999).unwrap()],
            ScanErrorCode::WorkspaceNotFound,
        ),
        (vec![a, b], ScanErrorCode::WorkspaceDisabled),
    ] {
        assert_eq!(
            service
                .start_scan(request(&service, ids, 1))
                .unwrap_err()
                .code,
            code
        );
    }
    let huge = add(&service, &format!("/{}", "x".repeat(32768)));
    assert_eq!(
        service
            .start_scan(request(&service, vec![huge], 1))
            .unwrap_err()
            .code,
        ScanErrorCode::ScopeTooLarge
    );
    assert_eq!(runner.0.load(Ordering::SeqCst), 0);
    assert!(service.current_scan().unwrap().session.is_none());
    let unwired = ApplicationService::new(Adapter, Database::open_in_memory().unwrap());
    let id = add(&unwired, "/fixture/unwired");
    assert_eq!(
        unwired
            .start_scan(request(&unwired, vec![id], 1))
            .unwrap_err()
            .code,
        ScanErrorCode::ApplicationUnavailable
    );
}

struct Script<F>(F);
impl<F> ScanRunner for Script<F>
where
    F: Fn(Vec<StoredWorkspace>, &ScanControl, &mut dyn FnMut(ScanProgress)) -> ScanReport
        + Send
        + Sync,
{
    fn run(
        &self,
        roots: Vec<StoredWorkspace>,
        _: ScanMode,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport {
        (self.0)(roots, control, publish)
    }
}
fn scripted<F>(action: F) -> ApplicationService<Adapter>
where
    F: Fn(Vec<StoredWorkspace>, &ScanControl, &mut dyn FnMut(ScanProgress)) -> ScanReport
        + Send
        + Sync
        + 'static,
{
    ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        Arc::new(Script(action)),
    )
}

// The 23-byte literal tuple overhead plus escaped prefix (12) and padding (32733)
// equals 32768. Catches raw-string length estimates and an off-by-one root cap.
#[test]
fn scan_scope_encoded_root_limit_counts_quotes_and_controls() {
    for extra in [0, 1] {
        let (service, runner, entered, release) = blocked(false);
        let root = ValidatedWorkspace {
            selected_path: "/r".into(),
            canonical_path: format!("\"\\\n\u{0001}{}", "x".repeat(32733 + extra)),
            identity: DirectoryIdentity::new("i"),
        };
        let id = match service.add_workspace(root, 1).unwrap() {
            WorkspaceInsertOutcome::Added(row) => row.id,
            _ => panic!("duplicate"),
        };
        let result = service.start_scan(request(&service, vec![id], 1));
        if extra == 0 {
            assert!(
                result.is_ok(),
                "exact encoded descriptor cap must fit: {result:?}"
            );
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            release.send(()).unwrap();
            wait_scan(&service);
            assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
        } else {
            assert_eq!(result.unwrap_err().code, ScanErrorCode::ScopeTooLarge);
            assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
            assert!(service.reserve_workspace_change().is_ok());
        }
    }
}

// Catches consuming the epoch/request or leaking a lease when no worker was created.
#[test]
fn scan_spawn_failure_and_persistence_failure_start_nothing() {
    let runner = Arc::new(NeverRun::default());
    let mut service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        runner.clone(),
    );
    let id = add(&service, "/fixture/a");
    let before = service.current_scan().unwrap();
    service.scans.fail_spawn = true;
    assert_eq!(
        service
            .start_scan(request(&service, vec![id], 1))
            .unwrap_err()
            .code,
        ScanErrorCode::ResourceUnavailable
    );
    assert_eq!(service.current_scan().unwrap(), before);
    assert!(service.reserve_workspace_change().is_ok());
    service.scans.fail_spawn = false;
    service
        .database
        .lock()
        .unwrap()
        .set_metadata("workspace_setup_state_v1", "invalid-fixture")
        .unwrap();
    assert_eq!(
        service
            .start_scan(request(&service, vec![id], 2))
            .unwrap_err()
            .code,
        ScanErrorCode::PersistenceUnavailable
    );
    assert_eq!(service.current_scan().unwrap(), before);
    assert_eq!(runner.0.load(Ordering::SeqCst), 0);
}

#[derive(Default)]
struct Clock(std::sync::atomic::AtomicU64);
impl MonotonicClock for Clock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
}

// Catches status pretending blocked work stopped, replay rejection in stopping,
// and replacing a previously observed budget stop with a later cancellation.
#[test]
fn scan_lifecycle_deadline_includes_blocked_validation_and_retains_owner() {
    for running in [false, true] {
        let (mut service, _, entered, release) = blocked(running);
        let clock = Arc::new(Clock::default());
        service.scans.clock = Some(clock.clone());
        let id = add(&service, "/fixture/a");
        let req = request(&service, vec![id], 1);
        let accepted = service.start_scan(req.clone()).unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            service.scan_session(&accepted.session_id).unwrap().state,
            if running {
                ScanState::Running
            } else {
                ScanState::Validating
            }
        );
        clock.0.store(5000, Ordering::SeqCst);
        let stopping = service.current_scan().unwrap().session.unwrap();
        assert_eq!(stopping.state, ScanState::Stopping);
        assert_eq!(stopping.stop_reason, Some(StopReason::TimeBudget));
        assert!(stopping.worker_active);
        assert!(stopping.finished_at.is_none());
        assert_eq!(service.start_scan(req).unwrap(), stopping);
        assert_eq!(
            service.cancel_scan(&accepted.session_id).unwrap().state,
            ScanState::Stopping
        );
        assert_eq!(
            service.remove_workspace(id).unwrap_err().code,
            WorkspaceErrorCode::ScanInProgress
        );
        release.send(()).unwrap();
        let terminal = wait_scan(&service);
        assert_eq!(terminal.state, ScanState::Partial);
        assert_eq!(terminal.stop_reason, Some(StopReason::TimeBudget));
        assert_eq!(terminal.metrics.observed_logical_bytes, DecimalQuantity(3));
        assert_eq!(terminal.metrics.logical_coverage, Coverage::Partial);
        assert!(service.reserve_workspace_change().is_ok());
    }
}

// Catches publication of a raw oversized report or losing the last accepted prefix.
#[test]
fn scan_lifecycle_encoded_snapshot_limit_retains_approved_prefix() {
    let service = scripted(|roots, _, publish| {
        let approved = progress(&roots);
        publish(approved.clone());
        let mut oversized = approved;
        oversized.workspaces[0].selected_path = "\u{0001}\"".repeat(20000);
        oversized.metrics.observed_logical_bytes = DecimalQuantity(999);
        oversized.entry_attempts = DecimalQuantity(999);
        publish(oversized.clone());
        // A later small update must not replace the frozen size-approved prefix.
        oversized.workspaces[0].selected_path = "/fixture/a".into();
        publish(oversized.clone());
        ScanReport {
            progress: oversized,
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    });
    let id = add(&service, "/fixture/a");
    service.start_scan(request(&service, vec![id], 1)).unwrap();
    let terminal = wait_scan(&service);
    assert_eq!(terminal.state, ScanState::Partial);
    assert_eq!(terminal.stop_reason, Some(StopReason::ResultLimit));
    assert_eq!(terminal.metrics.observed_logical_bytes, DecimalQuantity(3));
    assert_eq!(terminal.entry_attempts, DecimalQuantity(2));
    assert_eq!(terminal.workspaces[0].selected_path, "/fixture/a");
    assert!(serde_json::to_vec(&terminal).unwrap().len() <= 131072);
    assert!(terminal
        .diagnostics
        .iter()
        .any(|d| d.code == ScanDiagnosticCode::ResultLimit));
}

// Catches zero-success on no valid root, lost partial reports and leaked panic text.
#[test]
fn scan_lifecycle_unavailable_partial_and_panic_are_terminal() {
    for case in [0, 1, 2] {
        let service = scripted(move |roots, _, publish| {
            let mut update = progress(&roots);
            if case == 0 {
                update.workspaces[0].metrics = None;
                update.workspaces[0].state = WorkspaceScanState::Unavailable;
                update.workspaces[0].coverage = Coverage::Unavailable;
                update.coverage = Coverage::Unavailable;
                update.metrics = empty_metrics();
                update.diagnostics.push(ScanDiagnostic {
                    workspace_id: Some(roots[0].id),
                    code: ScanDiagnosticCode::RootDenied,
                    severity: DiagnosticSeverity::Error,
                    count: DecimalQuantity(1),
                });
            } else if case == 1 {
                update.coverage = Coverage::Partial;
                update.metrics.logical_coverage = Coverage::Partial;
                update.workspaces[0].state = WorkspaceScanState::Partial;
                update.workspaces[0].coverage = Coverage::Partial;
            }
            publish(update.clone());
            if case == 2 {
                panic!("private fixture panic payload");
            }
            ScanReport {
                progress: update,
                state: if case == 0 {
                    ScanState::Error
                } else {
                    ScanState::Partial
                },
                stop_reason: None,
                failure: None,
            }
        });
        let id = add(&service, "/fixture/a");
        service.start_scan(request(&service, vec![id], 1)).unwrap();
        let terminal = wait_scan(&service);
        assert_eq!(
            terminal.state,
            if case == 1 {
                ScanState::Partial
            } else {
                ScanState::Error
            }
        );
        assert!(!terminal.worker_active);
        assert!(terminal.finished_at.is_some());
        if case == 0 {
            assert!(terminal.workspaces[0].metrics.is_none());
            assert_eq!(terminal.coverage, Coverage::Unavailable);
        }
        if case == 2 {
            assert_eq!(
                terminal.failure.as_ref().unwrap().code,
                ScanErrorCode::ApplicationUnavailable
            );
            assert_eq!(terminal.coverage, Coverage::Partial);
        }
        assert!(!serde_json::to_string(&terminal)
            .unwrap()
            .contains("private fixture"));
        assert!(service.reserve_workspace_change().is_ok());
    }
}

// Catches shutdown waiting for I/O or permitting post-shutdown publication.
#[test]
fn scan_lifecycle_shutdown_stops_publication_without_waiting_for_io() {
    let (entered_tx, entered) = mpsc::channel();
    let (release, receive) = mpsc::channel();
    let receive = Mutex::new(receive);
    let service = scripted(move |roots, control, publish| {
        entered_tx.send(()).unwrap();
        receive
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert_eq!(control.checkpoint(), Err(StopReason::Cancelled));
        publish(progress(&roots));
        ScanReport {
            progress: progress(&roots),
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    });
    let id = add(&service, "/fixture/a");
    service.start_scan(request(&service, vec![id], 1)).unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let shared = service.scans.shared.clone();
    let (before, worker) = {
        let mut state = shared.state.lock().unwrap();
        (
            state.retained.as_ref().unwrap().snapshot.clone(),
            state.worker.take().unwrap(),
        )
    };
    let (dropped_tx, dropped) = mpsc::channel();
    thread::spawn(move || {
        drop(service);
        dropped_tx.send(()).unwrap();
    });
    let did_drop = dropped.recv_timeout(Duration::from_secs(2));
    release.send(()).unwrap();
    assert!(did_drop.is_ok(), "shutdown must not wait for the worker");
    let (joined_tx, joined) = mpsc::channel();
    thread::spawn(move || {
        worker.join().unwrap();
        joined_tx.send(()).unwrap();
    });
    joined.recv_timeout(Duration::from_secs(5)).unwrap();
    let state = shared.state.lock().unwrap();
    assert!(state.shutdown);
    assert_eq!(state.retained.as_ref().unwrap().snapshot, before);
}

// Catches fallback epochs, generation wrap and breaking health on scan clock failure.
#[test]
fn scan_lifecycle_initialization_and_generation_fail_closed() {
    let service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        Arc::new(NeverRun::default()),
    );
    let id = add(&service, "/fixture/a");
    let req = request(&service, vec![id], 1);
    service.scans.shared.state.lock().unwrap().startup = None;
    assert_eq!(
        service.current_scan().unwrap_err().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert_eq!(
        service.start_scan(req).unwrap_err().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert_eq!(service.health().unwrap().database_schema_version, 2);
    assert_eq!(service.list_workspaces().unwrap().workspaces.len(), 1);
    let service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        Arc::new(NeverRun::default()),
    );
    let id = add(&service, "/fixture/a");
    service.scans.shared.state.lock().unwrap().generation = u64::MAX;
    assert_eq!(
        service
            .start_scan(request(&service, vec![id], 1))
            .unwrap_err()
            .code,
        ScanErrorCode::ResourceUnavailable
    );
    assert!(service.current_scan().unwrap().session.is_none());
    assert!(service.reserve_workspace_change().is_ok());
}

// Catches oversized terminal-only reports, unbounded covered-by/diagnostic/root vectors,
// and propagating reports after a structural limit instead of retaining the prefix.
#[test]
fn scan_lifecycle_terminal_and_vector_limits_preserve_prefix() {
    for case in 0..4 {
        let service = scripted(move |roots, _, publish| {
            let approved = progress(&roots);
            publish(approved.clone());
            let mut invalid = approved;
            invalid.metrics.observed_logical_bytes = DecimalQuantity(777);
            match case {
                0 => invalid.workspaces[0].display_name = "\u{0001}".repeat(30000),
                1 => invalid.workspaces[0].covered_by_workspace_ids = vec![roots[0].id; 33],
                2 => {
                    invalid.diagnostics = vec![
                        ScanDiagnostic {
                            workspace_id: None,
                            code: ScanDiagnosticCode::EntryDenied,
                            severity: DiagnosticSeverity::Warning,
                            count: DecimalQuantity(1)
                        };
                        129
                    ]
                }
                _ => invalid.workspaces = vec![invalid.workspaces[0].clone(); 33],
            }
            ScanReport {
                progress: invalid,
                state: ScanState::Completed,
                stop_reason: None,
                failure: None,
            }
        });
        let id = add(&service, "/fixture/a");
        service.start_scan(request(&service, vec![id], 1)).unwrap();
        let terminal = wait_scan(&service);
        assert_eq!(terminal.state, ScanState::Partial);
        assert_eq!(terminal.stop_reason, Some(StopReason::ResultLimit));
        assert_eq!(terminal.metrics.observed_logical_bytes, DecimalQuantity(3));
        assert_eq!(terminal.workspaces.len(), 1);
        assert_eq!(terminal.workspaces[0].display_name, "a");
        assert!(terminal.workspaces[0].covered_by_workspace_ids.is_empty());
        assert!(terminal.diagnostics.len() <= 128);
        assert!(serde_json::to_vec(&terminal).unwrap().len() <= 131072);
    }
}

// Catches broadening the scan cap into a configuration limit or dropping requested rows.
#[test]
fn scan_scope_accepts_32_explicit_roots_in_numeric_order() {
    let (service, _, entered, release) = blocked(false);
    let mut ids = Vec::new();
    for index in 0..33 {
        ids.push(add(&service, &format!("/fixture/{index}")));
    }
    let selected: Vec<_> = ids[0..32].iter().rev().copied().collect();
    let req = request(&service, selected, 1);
    service.start_scan(req).unwrap();
    assert_eq!(
        entered.recv_timeout(Duration::from_secs(5)).unwrap(),
        ids[0..32]
    );
    release.send(()).unwrap();
    assert_eq!(wait_scan(&service).workspaces.len(), 32);
    assert_eq!(service.list_workspaces().unwrap().workspaces.len(), 33);
}

// Catches accidental SQLite/coordinator lock ownership across the runner's native wait.
#[test]
fn scan_activity_reads_respond_while_worker_blocks() {
    let (service, _, entered, release) = blocked(false);
    let id = add(&service, "/fixture/a");
    let req = request(&service, vec![id], 1);
    drop(service.start_scan(req.clone()).unwrap());
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let service = Arc::new(service);
    let reader = service.clone();
    let (sent, response) = mpsc::channel();
    thread::spawn(move || {
        sent.send((
            reader.health().unwrap().database_schema_version,
            reader.list_workspaces().unwrap().workspaces.len(),
            reader.check_workspace(id, 42).unwrap().checked_at,
            reader.start_scan(req).unwrap().worker_active,
        ))
        .unwrap();
    });
    let observed = response.recv_timeout(Duration::from_secs(2));
    release.send(()).unwrap();
    assert_eq!(observed.unwrap(), (2, 1, Some(42), true));
    wait_scan(&service);
}

// Catches reporting a worker panic as terminal before its owned resources unwind.
#[test]
fn scan_lifecycle_panic_drops_owned_resources_before_terminal() {
    struct Cleanup(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let cleaned = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let owned = cleaned.clone();
    let service = scripted(move |_, _, _| {
        let _resource = Cleanup(owned.clone());
        panic!("controlled worker panic")
    });
    let id = add(&service, "/fixture/a");
    service.start_scan(request(&service, vec![id], 1)).unwrap();
    let terminal = wait_scan(&service);
    assert!(cleaned.load(Ordering::SeqCst));
    assert_eq!(terminal.state, ScanState::Error);
    assert_eq!(
        terminal.failure.unwrap().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert!(service.reserve_workspace_change().is_ok());
}

// Engine allocation failure intentionally has no per-root report. It must preserve
// the accepted unavailable rows without inventing a result-limit stop.
#[test]
fn scan_lifecycle_runner_initialization_failure_preserves_typed_error() {
    let service = scripted(|_, _, _| ScanReport {
        progress: ScanProgress {
            workspaces: vec![],
            metrics: empty_metrics(),
            coverage: Coverage::Unavailable,
            diagnostics: vec![],
            entry_attempts: DecimalQuantity(0),
        },
        state: ScanState::Error,
        stop_reason: None,
        failure: Some(failure(ScanErrorCode::ResourceUnavailable)),
    });
    let id = add(&service, "/fixture/a");
    service.start_scan(request(&service, vec![id], 1)).unwrap();
    let terminal = wait_scan(&service);
    assert_eq!(terminal.state, ScanState::Error);
    assert_eq!(
        terminal.failure.unwrap().code,
        ScanErrorCode::ResourceUnavailable
    );
    assert_eq!(terminal.stop_reason, None);
    assert_eq!(terminal.workspaces.len(), 1);
    assert!(terminal.workspaces[0].metrics.is_none());
    assert_eq!(terminal.coverage, Coverage::Unavailable);
}

// Catches an ExitRequested hook needing to wait for I/O, releasing the lease early,
// or permitting a post-shutdown poll/replay to mutate and publish the snapshot.
#[test]
fn scan_lifecycle_explicit_shutdown_is_idempotent_and_retains_live_lease() {
    let (service, _, entered, release) = blocked(true);
    let id = add(&service, "/fixture/a");
    let req = request(&service, vec![id], 1);
    let accepted = service.start_scan(req.clone()).unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let service = Arc::new(service);
    let shared = service.scans.shared.clone();
    let before = shared
        .state
        .lock()
        .unwrap()
        .retained
        .as_ref()
        .unwrap()
        .snapshot
        .clone();
    let worker = shared.state.lock().unwrap().worker.take().unwrap();
    let shutdown = service.clone();
    let (sent, response) = mpsc::channel();
    thread::spawn(move || {
        shutdown.shutdown_scans();
        shutdown.shutdown_scans();
        sent.send(()).unwrap();
    });
    let returned = response.recv_timeout(Duration::from_secs(2));
    if returned.is_err() {
        let _ = release.send(());
    }
    returned.unwrap();
    assert_eq!(
        service.reserve_workspace_change().err().unwrap().code,
        WorkspaceErrorCode::ScanInProgress
    );
    assert_eq!(
        service.current_scan().unwrap_err().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert_eq!(
        service.start_scan(req).unwrap_err().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert_eq!(
        service.cancel_scan(&accepted.session_id).unwrap_err().code,
        ScanErrorCode::ApplicationUnavailable
    );
    assert_eq!(service.health().unwrap().database_schema_version, 2);
    release.send(()).unwrap();
    let (sent, joined) = mpsc::channel();
    thread::spawn(move || {
        worker.join().unwrap();
        sent.send(()).unwrap();
    });
    joined.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(service.reserve_workspace_change().is_ok());
    assert_eq!(
        shared
            .state
            .lock()
            .unwrap()
            .retained
            .as_ref()
            .unwrap()
            .snapshot,
        before
    );
}

// Build a controlled 32-row envelope. Padding is sized with the independent full
// JSON serializer, never the production size/reservation helpers under test.
fn multi_root_envelope(
    mut base: ScanSnapshot,
    roots: &[StoredWorkspace],
    bytes: usize,
    revision: u64,
) -> ScanSnapshot {
    let mut update = progress(roots);
    update.coverage = Coverage::Partial;
    update.metrics.logical_coverage = Coverage::Partial;
    update.metrics.allocated_coverage = Coverage::Partial;
    update.metrics.unmeasured_bytes = None;
    for row in &mut update.workspaces {
        row.state = WorkspaceScanState::Running;
        row.coverage = Coverage::Partial;
        row.metrics = Some(update.metrics.clone());
    }
    base.workspaces = update.workspaces;
    base.metrics = update.metrics;
    base.coverage = update.coverage;
    base.entry_attempts = update.entry_attempts;
    base.state = ScanState::Running;
    base.revision = DecimalQuantity(revision);
    let padding = bytes
        .checked_sub(serde_json::to_vec(&base).unwrap().len())
        .unwrap();
    base.workspaces[0]
        .selected_path
        .push_str(&"x".repeat(padding));
    assert_eq!(serde_json::to_vec(&base).unwrap().len(), bytes);
    base
}

fn envelope_progress(snapshot: ScanSnapshot) -> ScanProgress {
    ScanProgress {
        workspaces: snapshot.workspaces,
        metrics: snapshot.metrics,
        coverage: snapshot.coverage,
        diagnostics: snapshot.diagnostics,
        entry_attempts: snapshot.entry_attempts,
    }
}

// Catches the old QuantityOverflow-only reserve admitting an envelope whose
// immediate cancel acknowledgement exceeds the immutable ordinary-snapshot cap.
#[test]
fn scan_cancellation_reserve_rejects_unsafe_multi_root_publication() {
    let (seed_tx, seed_rx) = mpsc::channel::<ScanSnapshot>();
    let seed_rx = Mutex::new(seed_rx);
    let (observed_tx, observed) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    let service = scripted(move |roots, _, publish| {
        let base = seed_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        publish(progress(&roots));
        let unsafe_snapshot = multi_root_envelope(base, &roots, 126_941, 3);
        let mut old_reserve = unsafe_snapshot.clone();
        old_reserve.revision = DecimalQuantity(u64::MAX);
        old_reserve.state = ScanState::Stopping;
        old_reserve.stop_reason = Some(StopReason::QuantityOverflow);
        for row in &mut old_reserve.workspaces {
            row.state = WorkspaceScanState::Partial;
        }
        assert_eq!(serde_json::to_vec(&old_reserve).unwrap().len(), 126_976);
        let mut cancellation = unsafe_snapshot.clone();
        cancellation.revision = DecimalQuantity(4);
        cancellation.state = ScanState::Cancelling;
        cancellation.stop_reason = Some(StopReason::Cancelled);
        for row in &mut cancellation.workspaces {
            row.state = WorkspaceScanState::Cancelled;
        }
        assert_eq!(serde_json::to_vec(&cancellation).unwrap().len(), 127_015);
        println!(
            "unsafe bytes: running=126941 old_max_revision_reserve=126976 cancellation=127015"
        );
        publish(envelope_progress(unsafe_snapshot.clone()));
        observed_tx.send(()).unwrap();
        held.lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        ScanReport {
            progress: envelope_progress(unsafe_snapshot),
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    });
    let ids = (0..32)
        .map(|index| add(&service, &format!("/fixture/{index}")))
        .collect();
    let accepted = service.start_scan(request(&service, ids, 1)).unwrap();
    seed_tx.send(accepted.clone()).unwrap();
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    let stopped = service.current_scan().unwrap().session.unwrap();
    assert_eq!(stopped.state, ScanState::Stopping);
    assert_eq!(stopped.stop_reason, Some(StopReason::ResultLimit));
    assert_eq!(stopped.workspaces[0].selected_path, "/fixture/0");
    assert_eq!(stopped.metrics.observed_logical_bytes, DecimalQuantity(3));
    assert_eq!(stopped.entry_attempts, DecimalQuantity(2));
    assert!(stopped.worker_active);
    assert_eq!(service.cancel_scan(&accepted.session_id).unwrap(), stopped);
    release.send(()).unwrap();
    let terminal = wait_scan(&service);
    assert_eq!(terminal.state, ScanState::Partial);
    assert_eq!(terminal.stop_reason, Some(StopReason::ResultLimit));
    assert_eq!(terminal.workspaces[0].selected_path, "/fixture/0");
    assert_eq!(terminal.metrics.observed_logical_bytes, DecimalQuantity(3));
}

// Catches an off-by-one reserve, any unsupported active-stop/terminal shape, and
// delayed cancellation acknowledgement or early lease release at the exact cap.
#[test]
fn scan_cancellation_reserve_exact_safe_envelope_acknowledges_while_blocked() {
    let (seed_tx, seed_rx) = mpsc::channel::<ScanSnapshot>();
    let seed_rx = Mutex::new(seed_rx);
    let (observed_tx, observed) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let held = Mutex::new(held);
    let service = scripted(move |roots, _, publish| {
        let base = seed_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        // u64::MAX has 20 decimal digits, 19 more than this one-digit revision.
        let safe = multi_root_envelope(base, &roots, 126_883, 2);
        assert!(live_fits(&safe));
        let mut over = safe.clone();
        over.workspaces[0].selected_path.push('x');
        assert!(
            !live_fits(&over),
            "one byte beyond the cancellation reserve must reject"
        );
        for reason in [
            StopReason::TimeBudget,
            StopReason::EntryLimit,
            StopReason::IdentityLimit,
            StopReason::DescriptorLimit,
            StopReason::DiagnosticLimit,
            StopReason::ResultLimit,
            StopReason::QuantityOverflow,
            StopReason::Cancelled,
        ] {
            let mut stopped = safe.clone();
            stopped.revision = DecimalQuantity(u64::MAX);
            stopped.stop_reason = Some(reason);
            stopped.state = if reason == StopReason::Cancelled {
                ScanState::Cancelling
            } else {
                ScanState::Stopping
            };
            for row in &mut stopped.workspaces {
                row.state = if reason == StopReason::Cancelled {
                    WorkspaceScanState::Cancelled
                } else {
                    WorkspaceScanState::Partial
                };
            }
            let active_bytes = serde_json::to_vec(&stopped).unwrap().len();
            assert!(active_bytes <= 126_976, "{reason:?}: {active_bytes}");
            if reason == StopReason::Cancelled {
                assert_eq!(active_bytes, 126_976);
            }
            stopped.state = if reason == StopReason::Cancelled {
                ScanState::Cancelled
            } else {
                ScanState::Partial
            };
            stopped.worker_active = false;
            stopped.finished_at = Some(DecimalQuantity(u64::MAX));
            stopped.failure = Some(failure(ScanErrorCode::ApplicationUnavailable));
            stopped.diagnostics.push(ScanDiagnostic {
                workspace_id: None,
                code: reason.into(),
                severity: DiagnosticSeverity::Warning,
                count: DecimalQuantity(1),
            });
            let terminal_bytes = serde_json::to_vec(&stopped).unwrap().len();
            assert!(terminal_bytes <= 131_072, "{reason:?}: {terminal_bytes}");
            println!("safe stop {reason:?}: max_revision_active={active_bytes} max_time_terminal={terminal_bytes}");
        }
        publish(envelope_progress(safe.clone()));
        observed_tx.send(()).unwrap();
        held.lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        ScanReport {
            progress: envelope_progress(safe),
            state: ScanState::Completed,
            stop_reason: None,
            failure: None,
        }
    });
    let ids: Vec<_> = (0..32)
        .map(|index| add(&service, &format!("/fixture/{index}")))
        .collect();
    let first_id = ids[0];
    let accepted = service.start_scan(request(&service, ids, 1)).unwrap();
    seed_tx.send(accepted.clone()).unwrap();
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    let running = service.current_scan().unwrap().session.unwrap();
    assert_eq!(running.state, ScanState::Running);
    assert_eq!(serde_json::to_vec(&running).unwrap().len(), 126_883);
    let cancelling = service.cancel_scan(&accepted.session_id).unwrap();
    assert_eq!(cancelling.state, ScanState::Cancelling);
    assert_eq!(cancelling.stop_reason, Some(StopReason::Cancelled));
    assert!(cancelling
        .workspaces
        .iter()
        .all(|row| row.state == WorkspaceScanState::Cancelled));
    assert_eq!(
        cancelling.workspaces[0].selected_path,
        running.workspaces[0].selected_path
    );
    assert_eq!(
        cancelling.metrics.observed_logical_bytes,
        DecimalQuantity(3)
    );
    assert_eq!(cancelling.entry_attempts, DecimalQuantity(2));
    assert!(cancelling.worker_active);
    assert!(cancelling.finished_at.is_none());
    assert_eq!(serde_json::to_vec(&cancelling).unwrap().len(), 126_957);
    assert_eq!(
        service.remove_workspace(first_id).unwrap_err().code,
        WorkspaceErrorCode::ScanInProgress
    );
    release.send(()).unwrap();
    let terminal = wait_scan(&service);
    assert_eq!(terminal.state, ScanState::Cancelled);
    assert!(!terminal.worker_active);
    assert_eq!(
        terminal.workspaces[0].selected_path,
        running.workspaces[0].selected_path
    );
    assert_eq!(terminal.entry_attempts, DecimalQuantity(2));
    assert!(serde_json::to_vec(&terminal).unwrap().len() <= 131_072);
    assert!(service.reserve_workspace_change().is_ok());
}
