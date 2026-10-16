use crate::ApplicationService;
use dev_storage_core::{
    scanner::*,
    workspaces::{StoredWorkspace, WorkspaceRecord},
};
use dev_storage_persistence::Database;
use dev_storage_platform::PlatformAdapter;
use std::{
    io::{self, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

fn failure(code: ScanErrorCode) -> ScanFailure {
    ScanFailure { code }
}
fn unix_millis() -> Result<DecimalQuantity, ScanFailure> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .map(DecimalQuantity)
        .ok_or_else(|| failure(ScanErrorCode::ApplicationUnavailable))
}

struct Session {
    request: ScanRequest,
    snapshot: ScanSnapshot,
    control: Arc<ScanControl>,
    publication_limited: bool,
}
struct State {
    startup: Option<u128>,
    generation: u64,
    retained: Option<Session>,
    worker: Option<JoinHandle<()>>,
    shutdown: bool,
}
impl State {
    fn epoch(&self) -> Result<ScanEpoch, ScanFailure> {
        self.startup
            .map(|time| ScanEpoch::new(std::process::id(), time, self.generation))
            .ok_or_else(|| failure(ScanErrorCode::ApplicationUnavailable))
    }
}
struct Shared {
    state: Mutex<State>,
    #[cfg(test)]
    changed: std::sync::Condvar,
}
pub(crate) struct ScanCoordinator {
    shared: Arc<Shared>,
    runner: Option<Arc<dyn ScanRunner>>,
    #[cfg(test)]
    fail_spawn: bool,
    #[cfg(test)]
    clock: Option<Arc<dyn MonotonicClock>>,
}
impl ScanCoordinator {
    pub(crate) fn new(runner: Option<Arc<dyn ScanRunner>>) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    startup: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_nanos()),
                    generation: 0,
                    retained: None,
                    worker: None,
                    shutdown: false,
                }),
                #[cfg(test)]
                changed: std::sync::Condvar::new(),
            }),
            runner,
            #[cfg(test)]
            fail_spawn: false,
            #[cfg(test)]
            clock: None,
        }
    }
    fn control(&self, mode: ScanMode) -> Arc<ScanControl> {
        #[cfg(test)]
        if let Some(clock) = &self.clock {
            return Arc::new(ScanControl::with_clock(mode, clock.clone()));
        }
        Arc::new(ScanControl::new(mode))
    }
    fn spawn(&self, job: impl FnOnce() + Send + 'static) -> io::Result<JoinHandle<()>> {
        #[cfg(test)]
        if self.fail_spawn {
            return Err(io::Error::other("injected spawn failure"));
        }
        thread::Builder::new()
            .name("workspace-scan".into())
            .spawn(job)
    }
    fn shutdown(&self) {
        let mut state = self.shared.state.lock().unwrap_or_else(|p| p.into_inner());
        state.shutdown = true;
        if let Some(session) = &state.retained {
            session.control.cancel();
        }
        // Never join a possibly blocked syscall on the caller/UI thread.
        state.worker.take();
    }
}
impl Drop for ScanCoordinator {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Counts actual JSON bytes, including escaping, without allocating an encoded buffer.
struct BoundedBytes {
    count: usize,
    limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.count = self
            .count
            .checked_add(bytes.len())
            .filter(|n| *n <= self.limit)
            .ok_or_else(|| io::Error::other("scan encoding limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn check_scope(roots: &[StoredWorkspace], limits: ScanLimits) -> Result<(), ScanFailure> {
    let mut writer = BoundedBytes {
        count: 0,
        limit: limits.max_root_descriptor_bytes(),
    };
    // Internal descriptor format: [id, displayName, selectedPath, canonicalPath, identity].
    // Precheck raw lengths before cloning display names. JSON cannot encode fewer bytes.
    if roots.iter().any(|r| {
        r.root.selected_path.len() > writer.limit
            || r.root.canonical_path.len() > writer.limit
            || r.root.identity.as_str().len() > writer.limit
    }) {
        return Err(failure(ScanErrorCode::ScopeTooLarge));
    }
    let descriptors: Vec<_> = roots
        .iter()
        .map(|r| {
            (
                r.id,
                WorkspaceRecord::from(r).display_name,
                &r.root.selected_path,
                &r.root.canonical_path,
                r.root.identity.as_str(),
            )
        })
        .collect();
    serde_json::to_writer(&mut writer, &descriptors)
        .map_err(|_| failure(ScanErrorCode::ScopeTooLarge))
}
fn encoded_size(snapshot: &ScanSnapshot) -> Result<usize, StopReason> {
    let mut writer = BoundedBytes {
        count: 0,
        limit: ScanLimits::for_mode(snapshot.mode).max_snapshot_bytes(),
    };
    serde_json::to_writer(&mut writer, snapshot).map_err(|_| StopReason::ResultLimit)?;
    Ok(writer.count)
}
fn fits(snapshot: &ScanSnapshot, terminal: bool) -> bool {
    let Ok(bytes) = encoded_size(snapshot) else {
        return false;
    };
    let guard = SnapshotSizeGuard::new(ScanLimits::for_mode(snapshot.mode));
    if terminal {
        guard.check_terminal(bytes).is_ok()
    } else {
        guard.check_snapshot(bytes).is_ok()
    }
}
fn empty_metrics() -> ScanMetrics {
    ScanMetrics {
        observed_logical_bytes: DecimalQuantity(0),
        observed_allocated_bytes: None,
        unique_files: DecimalQuantity(0),
        unique_directories: DecimalQuantity(0),
        references_deduplicated: DecimalQuantity(0),
        allocation_measured_files: DecimalQuantity(0),
        allocation_unknown_files: DecimalQuantity(0),
        policy_excluded_entries: DecimalQuantity(0),
        unmeasured_entries_lower_bound: DecimalQuantity(0),
        unmeasured_bytes: None,
        logical_coverage: Coverage::Unavailable,
        allocated_coverage: Coverage::Unavailable,
    }
}
fn initial(
    request: &ScanRequest,
    roots: &[StoredWorkspace],
    epoch: ScanEpoch,
    generation: u64,
) -> Result<ScanSnapshot, ScanFailure> {
    Ok(ScanSnapshot {
        session_id: ScanSessionId::new(&epoch, generation)
            .map_err(|_| failure(ScanErrorCode::ResourceUnavailable))?,
        request_id: request.request_id.clone(),
        epoch,
        mode: request.mode,
        state: ScanState::Validating,
        revision: DecimalQuantity(1),
        started_at: unix_millis()?,
        finished_at: None,
        worker_active: true,
        stop_reason: None,
        failure: None,
        entry_attempts: DecimalQuantity(0),
        workspaces: roots
            .iter()
            .map(|r| WorkspaceScanResult {
                workspace_id: r.id,
                display_name: WorkspaceRecord::from(r).display_name,
                selected_path: r.root.selected_path.clone(),
                state: WorkspaceScanState::Pending,
                metrics: None,
                coverage: Coverage::Unavailable,
                covered_by_workspace_ids: vec![],
            })
            .collect(),
        metrics: empty_metrics(),
        coverage: Coverage::Unavailable,
        protection: measurement_protection(),
        diagnostics: vec![],
    })
}
fn partial_metrics(metrics: &mut ScanMetrics) {
    if metrics.logical_coverage != Coverage::Unavailable {
        metrics.logical_coverage = Coverage::Partial;
    }
    if metrics.allocated_coverage != Coverage::Unavailable {
        metrics.allocated_coverage = Coverage::Partial;
    }
    metrics.unmeasured_bytes = None;
}
fn incomplete(snapshot: &mut ScanSnapshot, cancelled: bool) {
    if snapshot.coverage != Coverage::Unavailable {
        snapshot.coverage = Coverage::Partial;
    }
    partial_metrics(&mut snapshot.metrics);
    for row in &mut snapshot.workspaces {
        if row.state == WorkspaceScanState::Unavailable {
            continue;
        }
        row.state = if cancelled {
            WorkspaceScanState::Cancelled
        } else {
            WorkspaceScanState::Partial
        };
        if row.coverage != Coverage::Unavailable {
            row.coverage = Coverage::Partial;
        }
        if let Some(metrics) = &mut row.metrics {
            partial_metrics(metrics);
        }
    }
}
fn stopped(snapshot: &mut ScanSnapshot, reason: StopReason) {
    snapshot.stop_reason = Some(reason);
    snapshot.state = if reason == StopReason::Cancelled {
        ScanState::Cancelling
    } else {
        ScanState::Stopping
    };
    incomplete(snapshot, reason == StopReason::Cancelled);
}
fn terminal_diagnostic(snapshot: &mut ScanSnapshot, reason: StopReason) {
    let code = ScanDiagnosticCode::from(reason);
    if snapshot.diagnostics.len() < ScanLimits::for_mode(snapshot.mode).max_diagnostic_groups()
        && !snapshot
            .diagnostics
            .iter()
            .any(|d| d.workspace_id.is_none() && d.code == code)
    {
        snapshot.diagnostics.push(ScanDiagnostic {
            workspace_id: None,
            code,
            severity: DiagnosticSeverity::Warning,
            count: DecimalQuantity(1),
        });
    }
    // If all groups are already occupied, the mandatory stopReason remains
    // authoritative. Never truncate an accepted group to make room.
}
fn next_revision(snapshot: &mut ScanSnapshot) -> bool {
    match snapshot.revision.0.checked_add(1) {
        Some(value) => {
            snapshot.revision = DecimalQuantity(value);
            true
        }
        None => false,
    }
}
fn refresh_stop(session: &mut Session) {
    if session.snapshot.worker_active {
        if let Err(reason) = session.control.checkpoint() {
            if session.snapshot.stop_reason != Some(reason) {
                let mut candidate = session.snapshot.clone();
                stopped(&mut candidate, reason);
                next_revision(&mut candidate);
                if fits(&candidate, false) {
                    session.snapshot = candidate;
                }
            }
        }
    }
}
// Reserve actual encoded space for any active stop transition as well as final fields.
fn live_fits(snapshot: &ScanSnapshot) -> bool {
    if !fits(snapshot, false) {
        return false;
    }
    // QuantityOverflow is the longest budget-stop reason. Cancellation has a
    // shorter reason, but its per-workspace "cancelled" state is longer than
    // "partial", so neither envelope bounds the other for every root count.
    for reason in [StopReason::QuantityOverflow, StopReason::Cancelled] {
        let mut reserve = snapshot.clone();
        reserve.revision = DecimalQuantity(u64::MAX);
        stopped(&mut reserve, reason);
        if !fits(&reserve, false) {
            return false;
        }
        reserve.state = ScanState::Cancelled;
        reserve.finished_at = Some(DecimalQuantity(u64::MAX));
        reserve.worker_active = false;
        reserve.failure = Some(failure(ScanErrorCode::ApplicationUnavailable));
        terminal_diagnostic(&mut reserve, reason);
        if !fits(&reserve, true) {
            return false;
        }
    }
    true
}
fn replace_progress(candidate: &mut ScanSnapshot, progress: ScanProgress) -> bool {
    let limits = ScanLimits::for_mode(candidate.mode);
    if progress.workspaces.len() != candidate.workspaces.len()
        || progress
            .workspaces
            .iter()
            .zip(&candidate.workspaces)
            .any(|(a, b)| a.workspace_id != b.workspace_id)
        || progress
            .workspaces
            .iter()
            .any(|row| row.covered_by_workspace_ids.len() > limits.max_requested_roots())
        || progress.diagnostics.len() > limits.max_diagnostic_groups()
    {
        return false;
    }
    candidate.workspaces = progress.workspaces;
    candidate.metrics = progress.metrics;
    candidate.coverage = progress.coverage;
    candidate.diagnostics = progress.diagnostics;
    candidate.entry_attempts = progress.entry_attempts;
    true
}

fn publish(shared: &Shared, id: &ScanSessionId, progress: ScanProgress) {
    let mut state = shared.state.lock().unwrap_or_else(|p| p.into_inner());
    if state.shutdown {
        return;
    }
    let Some(session) = state
        .retained
        .as_mut()
        .filter(|s| s.snapshot.session_id == *id && s.snapshot.worker_active)
    else {
        return;
    };
    if session.publication_limited {
        return;
    }
    let mut candidate = session.snapshot.clone();
    let valid = replace_progress(&mut candidate, progress);
    if candidate.workspaces.iter().any(|r| r.metrics.is_some()) {
        candidate.state = ScanState::Running;
    }
    if let Err(reason) = session.control.checkpoint() {
        stopped(&mut candidate, reason);
    }
    if !next_revision(&mut candidate) {
        session.control.request_stop(StopReason::QuantityOverflow);
    } else if valid && live_fits(&candidate) {
        session.snapshot = candidate;
    } else {
        session.publication_limited = true;
        session.control.request_stop(StopReason::ResultLimit);
    }
    refresh_stop(session);
}

fn finish(
    shared: &Shared,
    id: &ScanSessionId,
    report: Result<ScanReport, Box<dyn std::any::Any + Send>>,
    reservation: crate::activity::ScanReservation,
) {
    let mut state = shared.state.lock().unwrap_or_else(|p| p.into_inner());
    if !state.shutdown {
        if let Some(session) = state
            .retained
            .as_mut()
            .filter(|s| s.snapshot.session_id == *id)
        {
            let mut candidate = session.snapshot.clone();
            let (report_state, report_failure) = match report {
                Ok(report) => {
                    if let Some(reason) = report.stop_reason {
                        session.control.request_stop(reason);
                    }
                    let initialization_failed =
                        report.failure.is_some() && report.progress.workspaces.is_empty();
                    if !session.publication_limited
                        && !initialization_failed
                        && (!replace_progress(&mut candidate, report.progress)
                            || !live_fits(&candidate))
                    {
                        session.publication_limited = true;
                        session.control.request_stop(StopReason::ResultLimit);
                        candidate = session.snapshot.clone();
                    }
                    (report.state, report.failure)
                }
                Err(_) => (
                    ScanState::Error,
                    Some(failure(ScanErrorCode::ApplicationUnavailable)),
                ),
            };
            let reason = session.control.checkpoint().err();
            if let Some(reason) = reason {
                stopped(&mut candidate, reason);
                terminal_diagnostic(&mut candidate, reason);
            }
            if report_failure.is_some() {
                incomplete(&mut candidate, reason == Some(StopReason::Cancelled));
            }
            candidate.state = if reason == Some(StopReason::Cancelled) {
                ScanState::Cancelled
            } else if report_failure.is_some() {
                ScanState::Error
            } else if reason.is_some() {
                ScanState::Partial
            } else {
                report_state
            };
            candidate.failure = report_failure;
            candidate.worker_active = false;
            candidate.finished_at = unix_millis().ok();
            if candidate.finished_at.is_none() {
                candidate.state = ScanState::Error;
                candidate.failure = Some(failure(ScanErrorCode::ApplicationUnavailable));
            }
            next_revision(&mut candidate);
            if !fits(&candidate, true) {
                // The raw terminal report is not publishable. Preserve the last
                // size-approved prefix and independently encode the fallback.
                let reason = session.control.request_stop(StopReason::ResultLimit);
                let mut fallback = session.snapshot.clone();
                stopped(&mut fallback, reason);
                terminal_diagnostic(&mut fallback, reason);
                fallback.state = if reason == StopReason::Cancelled {
                    ScanState::Cancelled
                } else {
                    ScanState::Partial
                };
                fallback.worker_active = false;
                fallback.finished_at = candidate.finished_at;
                next_revision(&mut fallback);
                candidate = fallback;
            }
            // Every retained prefix passed live_fits with actual terminal reserve.
            assert!(
                fits(&candidate, true),
                "reserved terminal snapshot must fit"
            );
            session.snapshot = candidate;
        }
    }
    // The runner has returned/unwound and all its cursors are gone. Linearize the
    // terminal snapshot and lease release together against cancel/start.
    drop(reservation);
    #[cfg(test)]
    shared.changed.notify_all();
}

/// Rust-only injection; no frontend can choose a runner or construct filesystem scope.
pub trait ScanRunner: Send + Sync {
    fn run(
        &self,
        roots: Vec<StoredWorkspace>,
        mode: ScanMode,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> ScanReport;
}

impl<A: PlatformAdapter> ApplicationService<A> {
    /// Rust lifecycle hook for application exit, never an IPC command. The owned
    /// worker keeps its reservation until actual cleanup; this does not join it.
    pub fn shutdown_scans(&self) {
        self.scans.shutdown();
    }
    pub fn new_with_scan_runner(
        adapter: A,
        database: Database,
        runner: Arc<dyn ScanRunner>,
    ) -> Self {
        Self {
            adapter,
            database: Mutex::new(database),
            activity: crate::ActivityGate::default(),
            scans: ScanCoordinator::new(Some(runner)),
        }
    }
    pub fn start_scan(&self, request: ScanRequest) -> Result<ScanSnapshot, ScanFailure> {
        let mut state = self
            .scans
            .shared
            .state
            .lock()
            .map_err(|_| failure(ScanErrorCode::ApplicationUnavailable))?;
        if state.shutdown {
            return Err(failure(ScanErrorCode::ApplicationUnavailable));
        }
        if let Some(session) = state
            .retained
            .as_mut()
            .filter(|s| s.request.request_id == request.request_id)
        {
            if session.request != request {
                return Err(failure(ScanErrorCode::RequestConflict));
            }
            refresh_stop(session);
            return Ok(session.snapshot.clone());
        }
        if request.expected_epoch != state.epoch()? {
            return Err(failure(ScanErrorCode::StaleEpoch));
        }
        let runner = self
            .scans
            .runner
            .clone()
            .ok_or_else(|| failure(ScanErrorCode::ApplicationUnavailable))?;
        let reservation = self.activity.try_scan()?;
        let limits = ScanLimits::for_mode(request.mode);
        if request.workspace_ids.is_empty() {
            return Err(failure(ScanErrorCode::ScopeEmpty));
        }
        if request.workspace_ids.len() > limits.max_requested_roots() {
            return Err(failure(ScanErrorCode::ScopeTooLarge));
        }
        let mut ids = request.workspace_ids.clone();
        ids.sort_by_key(|id| id.get());
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(failure(ScanErrorCode::InvalidRequest));
        }
        let control = self.scans.control(request.mode);
        let roots = self
            .database
            .lock()
            .map_err(|_| failure(ScanErrorCode::ApplicationUnavailable))?
            .scan_roots_by_ids(&ids)
            .map_err(|_| failure(ScanErrorCode::PersistenceUnavailable))?;
        if roots.len() != ids.len() {
            return Err(failure(ScanErrorCode::WorkspaceNotFound));
        }
        if roots.iter().any(|root| !root.enabled) {
            return Err(failure(ScanErrorCode::WorkspaceDisabled));
        }
        check_scope(&roots, limits)?;
        let generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| failure(ScanErrorCode::ResourceUnavailable))?;
        let epoch = ScanEpoch::new(
            std::process::id(),
            state
                .startup
                .ok_or_else(|| failure(ScanErrorCode::ApplicationUnavailable))?,
            generation,
        );
        let snapshot = initial(&request, &roots, epoch, generation)?;
        if !live_fits(&snapshot) {
            return Err(failure(ScanErrorCode::ScopeTooLarge));
        }
        let shared = self.scans.shared.clone();
        let worker_control = control.clone();
        let id = snapshot.session_id.clone();
        let mode = request.mode;
        let worker = self
            .scans
            .spawn(move || {
                let report = catch_unwind(AssertUnwindSafe(|| {
                    runner.run(roots, mode, &worker_control, &mut |progress| {
                        publish(&shared, &id, progress)
                    })
                }));
                finish(&shared, &id, report, reservation);
            })
            .map_err(|_| failure(ScanErrorCode::ResourceUnavailable))?;
        state.generation = generation;
        state.retained = Some(Session {
            request,
            snapshot: snapshot.clone(),
            control,
            publication_limited: false,
        });
        state.worker = Some(worker);
        Ok(snapshot)
    }
    pub fn current_scan(&self) -> Result<ScanStatus, ScanFailure> {
        let mut state = self
            .scans
            .shared
            .state
            .lock()
            .map_err(|_| failure(ScanErrorCode::ApplicationUnavailable))?;
        if state.shutdown {
            return Err(failure(ScanErrorCode::ApplicationUnavailable));
        }
        if let Some(session) = &mut state.retained {
            refresh_stop(session);
        }
        Ok(ScanStatus {
            epoch: state.epoch()?,
            session: state.retained.as_ref().map(|s| s.snapshot.clone()),
        })
    }
    pub fn scan_session(&self, id: &ScanSessionId) -> Result<ScanSnapshot, ScanFailure> {
        self.current_scan()?
            .session
            .filter(|s| s.session_id == *id)
            .ok_or_else(|| failure(ScanErrorCode::SessionNotFound))
    }
    pub fn cancel_scan(&self, id: &ScanSessionId) -> Result<ScanSnapshot, ScanFailure> {
        let mut state = self
            .scans
            .shared
            .state
            .lock()
            .map_err(|_| failure(ScanErrorCode::ApplicationUnavailable))?;
        if state.shutdown {
            return Err(failure(ScanErrorCode::ApplicationUnavailable));
        }
        let session = state
            .retained
            .as_mut()
            .filter(|s| s.snapshot.session_id == *id)
            .ok_or_else(|| failure(ScanErrorCode::SessionNotFound))?;
        if session.snapshot.worker_active {
            session.control.cancel();
            refresh_stop(session);
        }
        Ok(session.snapshot.clone())
    }
}

#[cfg(test)]
mod tests;
