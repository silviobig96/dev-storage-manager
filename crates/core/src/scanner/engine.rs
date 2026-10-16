//! Bounded metadata-only DFS; the service owns session finalization and DTO publication.
use super::*;
use crate::workspaces::{StoredWorkspace, WorkspaceId, WorkspaceRecord};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Values must never go backwards. Share this clock with the coordinator.
pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> Duration;
}
struct SystemClock(Instant);
impl MonotonicClock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
}

/// Create at session acceptance, before validation, with the same mode as scan.
/// A stop request never means native work or descriptor cleanup has finished.
pub struct ScanControl {
    clock: Arc<dyn MonotonicClock>,
    deadline: Duration,
    stop: Mutex<Option<StopReason>>,
}
impl ScanControl {
    pub fn new(mode: ScanMode) -> Self {
        Self::with_clock(mode, Arc::new(SystemClock(Instant::now())))
    }
    pub fn with_clock(mode: ScanMode, clock: Arc<dyn MonotonicClock>) -> Self {
        let deadline = clock.now().saturating_add(Duration::from_millis(
            ScanLimits::for_mode(mode).max_elapsed_millis(),
        ));
        Self {
            clock,
            deadline,
            stop: Mutex::new(None),
        }
    }
    /// First requested/observed stop wins. Terminal races belong to the service mutex.
    pub fn request_stop(&self, reason: StopReason) -> StopReason {
        let mut stop = self
            .stop
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *stop.get_or_insert(reason)
    }
    pub fn cancel(&self) -> StopReason {
        self.request_stop(StopReason::Cancelled)
    }
    pub fn checkpoint(&self) -> Result<(), StopReason> {
        if let Some(reason) = *self
            .stop
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
        {
            return Err(reason);
        }
        if self.clock.now() >= self.deadline {
            return Err(self.request_stop(StopReason::TimeBudget));
        }
        Ok(())
    }
}
impl ScanIoControl for ScanControl {
    fn checkpoint(&self) -> Result<(), StopReason> {
        self.checkpoint()
    }
}

/// Replace retained state; never queue each callback. No descendant paths are exported.
/// The service must bound encoded root descriptors before scanning, and serialize the
/// ENTIRE ScanSnapshot with SnapshotSizeGuard before accepting any publication.
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub workspaces: Vec<WorkspaceScanResult>,
    pub metrics: ScanMetrics,
    pub coverage: Coverage,
    pub diagnostics: Vec<ScanDiagnostic>,
    /// Issued next_entry calls, including EOF/failure; not objects or native syscalls.
    pub entry_attempts: DecimalQuantity,
}
/// Returned after all cursors drop. The service supplies revision/time/workerActive
/// and decides terminal session state under its finalization mutex.
#[derive(Debug, Clone)]
pub struct ScanReport {
    pub progress: ScanProgress,
    pub state: ScanState,
    pub stop_reason: Option<StopReason>,
    pub failure: Option<ScanFailure>,
}
pub fn scan(
    roots: Vec<StoredWorkspace>,
    mode: ScanMode,
    fs: &dyn ScanFilesystem,
    control: &ScanControl,
    publish: &mut dyn FnMut(ScanProgress),
) -> ScanReport {
    scan_with_limits(roots, ScanLimits::for_mode(mode), fs, control, publish)
}
// Private entry point. Only module tests can construct smaller limits.
pub(super) fn scan_with_limits(
    mut roots: Vec<StoredWorkspace>,
    limits: ScanLimits,
    fs: &dyn ScanFilesystem,
    control: &ScanControl,
    publish: &mut dyn FnMut(ScanProgress),
) -> ScanReport {
    roots.sort_by_key(|root| root.id.get());
    let invalid = if roots.is_empty() {
        Some(ScanErrorCode::ScopeEmpty)
    } else if roots.len() > limits.max_requested_roots() {
        Some(ScanErrorCode::ScopeTooLarge)
    } else if roots.windows(2).any(|pair| pair[0].id == pair[1].id) {
        Some(ScanErrorCode::InvalidRequest)
    } else if roots.iter().any(|root| !root.enabled) {
        Some(ScanErrorCode::WorkspaceDisabled)
    } else {
        None
    };
    if let Some(code) = invalid {
        return failed_report(ScanFailure { code });
    }
    let mut engine = match Engine::new(roots, limits) {
        Ok(engine) => engine,
        Err(failure) => return failed_report(failure),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| engine.run(fs, control, publish)));
    let failure = match outcome {
        Ok(Ok(())) => None,
        Ok(Err(reason)) => {
            engine.stop(reason, control);
            None
        }
        Err(_) => Some(ScanFailure {
            code: ScanErrorCode::ApplicationUnavailable,
        }),
    };
    // run() has dropped all cursors, also on unwind. Pending identities remain
    // partial after panic, without a follow-up filesystem call.
    if engine.stop_reason.is_some() || failure.is_some() {
        for (index, root) in engine.roots.iter().enumerate() {
            if matches!(
                engine.states[index],
                WorkspaceScanState::Pending
                    | WorkspaceScanState::Validating
                    | WorkspaceScanState::Running
            ) || (index == engine.current_root && engine.measured[index])
            {
                let _ = engine.accounting.mark_incomplete(root.id, 0);
                engine.states[index] = if engine.stop_reason == Some(StopReason::Cancelled) {
                    WorkspaceScanState::Cancelled
                } else {
                    WorkspaceScanState::Partial
                };
            }
        }
    }
    let progress = engine.progress();
    let state = if failure.is_some() {
        ScanState::Error
    } else if engine.stop_reason == Some(StopReason::Cancelled) {
        ScanState::Cancelled
    } else if engine.stop_reason.is_some() {
        ScanState::Partial
    } else if !engine.measured.iter().any(|value| *value) {
        ScanState::Error
    } else if progress.coverage == Coverage::CompleteWithinPolicy {
        ScanState::Completed
    } else {
        ScanState::Partial
    };
    ScanReport {
        progress,
        state,
        stop_reason: engine.stop_reason,
        failure,
    }
}
fn failed_report(failure: ScanFailure) -> ScanReport {
    let metrics = ScanMetrics {
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
    };
    ScanReport {
        progress: ScanProgress {
            workspaces: Vec::new(),
            metrics,
            coverage: Coverage::Unavailable,
            diagnostics: Vec::new(),
            entry_attempts: DecimalQuantity(0),
        },
        state: ScanState::Error,
        stop_reason: None,
        failure: Some(failure),
    }
}

struct Frame {
    cursor: Box<dyn ScanDirectory>,
    identity: ObjectIdentity,
    link: Option<ScanEntry>,
    partial: bool,
}
struct Engine {
    roots: Vec<StoredWorkspace>,
    states: Vec<WorkspaceScanState>,
    measured: Vec<bool>,
    accounting: ScanAccounting,
    diagnostics: ScanDiagnostics,
    limits: ScanLimits,
    attempts: u64,
    stop_reason: Option<StopReason>,
    last_publish: Duration,
    current_root: usize,
}
impl Engine {
    fn new(roots: Vec<StoredWorkspace>, limits: ScanLimits) -> Result<Self, ScanFailure> {
        Ok(Self {
            states: vec![WorkspaceScanState::Pending; roots.len()],
            measured: vec![false; roots.len()],
            roots,
            accounting: ScanAccounting::new(limits)?,
            diagnostics: ScanDiagnostics::new(limits)?,
            limits,
            attempts: 0,
            stop_reason: None,
            last_publish: Duration::ZERO,
            current_root: 0,
        })
    }
    fn progress(&self) -> ScanProgress {
        let aggregate = self.accounting.snapshot();
        let mut metrics = aggregate.metrics;
        let coverage = if !self.measured.iter().any(|value| *value) {
            metrics.logical_coverage = Coverage::Unavailable;
            metrics.allocated_coverage = Coverage::Unavailable;
            metrics.observed_allocated_bytes = None;
            metrics.unmeasured_bytes = None;
            Coverage::Unavailable
        } else if self.states.iter().any(|state| {
            matches!(
                state,
                WorkspaceScanState::Pending
                    | WorkspaceScanState::Validating
                    | WorkspaceScanState::Unavailable
            )
        }) {
            metrics.logical_coverage = Coverage::Partial;
            metrics.unmeasured_bytes = None;
            if metrics.allocated_coverage == Coverage::CompleteWithinPolicy {
                metrics.allocated_coverage = Coverage::Partial;
            }
            Coverage::Partial
        } else {
            aggregate.coverage
        };
        let workspaces = self
            .roots
            .iter()
            .enumerate()
            .map(|(index, root)| {
                let metadata = WorkspaceRecord::from(root);
                let observed = self.measured[index]
                    .then(|| self.accounting.workspace_snapshot(root.id))
                    .flatten();
                WorkspaceScanResult {
                    workspace_id: root.id,
                    display_name: metadata.display_name,
                    selected_path: metadata.selected_path,
                    state: self.states[index],
                    metrics: observed.as_ref().map(|value| value.metrics.clone()),
                    coverage: observed
                        .as_ref()
                        .map_or(Coverage::Unavailable, |value| value.coverage),
                    covered_by_workspace_ids: observed
                        .map_or_else(Vec::new, |value| value.covered_by_workspace_ids),
                }
            })
            .collect();
        ScanProgress {
            workspaces,
            metrics,
            coverage,
            diagnostics: self.diagnostics.snapshot(),
            entry_attempts: DecimalQuantity(self.attempts),
        }
    }
    fn publish(
        &mut self,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
        force: bool,
    ) {
        let now = control.clock.now();
        if force
            || now.saturating_sub(self.last_publish)
                >= Duration::from_millis(self.limits.query_interval_millis())
        {
            self.last_publish = now;
            publish(self.progress());
        }
    }
    fn stop(&mut self, reason: StopReason, control: &ScanControl) {
        self.stop_reason = Some(self.diagnostics.record_stop(control.request_stop(reason)));
    }
    fn diagnostic(
        &mut self,
        owner: WorkspaceId,
        code: ScanDiagnosticCode,
    ) -> Result<(), StopReason> {
        self.diagnostics
            .record(Some(owner), code, DiagnosticSeverity::Warning)
    }
    fn omission(
        &mut self,
        owner: WorkspaceId,
        stack: &mut [Frame],
        count: u64,
    ) -> Result<(), StopReason> {
        for frame in stack {
            frame.partial = true;
        }
        self.accounting.mark_incomplete(owner, count)
    }
    fn run(
        &mut self,
        fs: &dyn ScanFilesystem,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> Result<(), StopReason> {
        for index in 0..self.roots.len() {
            self.current_root = index;
            control.checkpoint()?;
            let owner = self.roots[index].id;
            self.accounting.register_workspace(owner)?;
            self.states[index] = WorkspaceScanState::Validating;
            self.publish(control, publish, true);
            // Native acquisition needs at most 3 simultaneous descriptors: selected,
            // canonical, temporary home. No previous root cursor remains live.
            if self.limits.max_descriptors() < 3 {
                return Err(StopReason::DescriptorLimit);
            }
            control.checkpoint()?;
            let result = fs.anchor_root(&self.roots[index], control);
            control.checkpoint()?;
            let cursor = match result {
                Ok(cursor) => cursor,
                Err(ScanIoError::Stopped(reason)) => return Err(reason),
                Err(ScanIoError::Diagnostic(code)) => {
                    self.states[index] = WorkspaceScanState::Unavailable;
                    self.accounting.mark_incomplete(owner, 0)?;
                    self.diagnostic(owner, code)?;
                    self.publish(control, publish, true);
                    continue;
                }
            };
            let identity = cursor.identity();
            // An anchored cursor promises a supported, authorized directory object.
            let metadata = ScanEntry {
                name: Default::default(),
                identity: Some(identity),
                facts: ReadFacts {
                    kind: EntryKind::Directory,
                    in_read_scope: true,
                    filesystem_supported: true,
                    mount: MountFact::Same,
                    cloud: CloudFact::Materialized,
                    root_too_broad: false,
                },
                logical_bytes: None,
                allocated_bytes: None,
            };
            control.checkpoint()?;
            let observation = self.accounting.observe(owner, &metadata)?;
            self.measured[index] = true;
            self.states[index] = WorkspaceScanState::Running;
            if observation == ObservationOutcome::Directory {
                self.walk(owner, cursor, control, publish)?;
            } else {
                drop(cursor);
            }
            let coverage = self
                .accounting
                .workspace_snapshot(owner)
                .expect("registered root")
                .coverage;
            self.states[index] = if coverage == Coverage::CompleteWithinPolicy {
                WorkspaceScanState::Completed
            } else {
                WorkspaceScanState::Partial
            };
            self.publish(control, publish, true);
        }
        control.checkpoint()
    }
    fn walk(
        &mut self,
        owner: WorkspaceId,
        cursor: Box<dyn ScanDirectory>,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> Result<(), StopReason> {
        let mut stack = Vec::with_capacity(self.limits.max_child_depth() + 1);
        stack.push(Frame {
            identity: cursor.identity(),
            cursor,
            link: None,
            partial: false,
        });
        let outcome = self.walk_stack(owner, &mut stack, control, publish);
        if outcome.is_err() {
            self.omission(owner, &mut stack, 0)?;
            for frame in &stack {
                self.accounting
                    .finish_directory(frame.identity, Coverage::Partial);
            }
        }
        // Unconditional RAII close; no finish syscalls after a stop.
        drop(stack);
        outcome
    }
    fn walk_stack(
        &mut self,
        owner: WorkspaceId,
        stack: &mut Vec<Frame>,
        control: &ScanControl,
        publish: &mut dyn FnMut(ScanProgress),
    ) -> Result<(), StopReason> {
        'walk: while !stack.is_empty() {
            control.checkpoint()?;
            let top = stack.len() - 1;
            // Retaining an immediate parent does not prove that parent still
            // belongs to its own ancestor. Revalidate the bounded active chain.
            for child in 1..stack.len() {
                let result = stack[child - 1].cursor.verify_link(
                    stack[child].link.as_ref().expect("child link"),
                    &stack[child].identity,
                    control,
                );
                control.checkpoint()?;
                match result {
                    Ok(()) => {}
                    Err(ScanIoError::Stopped(reason)) => return Err(reason),
                    Err(ScanIoError::Diagnostic(code)) => {
                        self.omission(owner, stack, 0)?;
                        self.diagnostic(owner, code)?;
                        while stack.len() > child {
                            self.close_frame(stack);
                        }
                        continue 'walk;
                    }
                }
            }
            if self.attempts >= self.limits.max_entry_attempts() {
                return Err(StopReason::EntryLimit);
            }
            self.attempts = self
                .attempts
                .checked_add(1)
                .ok_or(StopReason::QuantityOverflow)?;
            let result = stack[top].cursor.next_entry(control);
            control.checkpoint()?;
            let entry = match result {
                Err(ScanIoError::Stopped(reason)) => return Err(reason),
                Err(ScanIoError::Diagnostic(code)) => {
                    self.omission(owner, stack, 0)?;
                    self.diagnostic(owner, code)?;
                    self.close_frame(stack);
                    continue;
                }
                Ok(None) => {
                    control.checkpoint()?;
                    let result = stack[top].cursor.finish(control);
                    control.checkpoint()?;
                    match result {
                        Ok(()) => {}
                        Err(ScanIoError::Stopped(reason)) => return Err(reason),
                        Err(ScanIoError::Diagnostic(code)) => {
                            self.omission(owner, stack, 0)?;
                            self.diagnostic(owner, code)?;
                        }
                    }
                    self.close_frame(stack);
                    continue;
                }
                Ok(Some(entry)) => entry,
            };
            control.checkpoint()?;
            let decision = ScanReadPolicy::evaluate(&entry.facts);
            let observation = match self.accounting.observe(owner, &entry) {
                Ok(observation) => observation,
                Err(reason) => {
                    self.omission(owner, stack, 1)?;
                    return Err(reason);
                }
            };
            if entry.name.to_str().is_none() {
                self.diagnostic(owner, ScanDiagnosticCode::UnrepresentableName)?;
            }
            match observation {
                ObservationOutcome::Directory => {
                    let identity = entry.identity.expect("accounted identity");
                    if stack.len() > self.limits.max_child_depth() {
                        self.accounting
                            .finish_directory(identity, Coverage::Partial);
                        self.omission(owner, stack, 1)?;
                        self.diagnostic(owner, ScanDiagnosticCode::DepthLimit)?;
                    } else {
                        if stack.len() >= self.limits.max_descriptors() {
                            self.accounting
                                .finish_directory(identity, Coverage::Partial);
                            self.omission(owner, stack, 1)?;
                            return Err(StopReason::DescriptorLimit);
                        }
                        control.checkpoint()?;
                        let result = stack[top].cursor.open_child(&entry, control);
                        control.checkpoint()?;
                        match result {
                            Ok(cursor) => {
                                if cursor.identity() != identity {
                                    self.accounting
                                        .finish_directory(identity, Coverage::Partial);
                                    self.omission(owner, stack, 1)?;
                                    self.diagnostic(owner, ScanDiagnosticCode::ChangedDuringScan)?;
                                } else {
                                    stack.push(Frame {
                                        cursor,
                                        identity,
                                        link: Some(entry),
                                        partial: false,
                                    });
                                }
                            }
                            Err(ScanIoError::Stopped(reason)) => return Err(reason),
                            Err(ScanIoError::Diagnostic(code)) => {
                                self.accounting
                                    .finish_directory(identity, Coverage::Partial);
                                self.omission(owner, stack, 1)?;
                                self.diagnostic(owner, code)?;
                            }
                        }
                    }
                }
                ObservationOutcome::Unmeasured => {
                    let fixed_exclusion = matches!(
                        decision,
                        ReadDecision::Skip(
                            ReadSkipReason::SymlinkExcluded
                                | ReadSkipReason::SpecialExcluded
                                | ReadSkipReason::MountExcluded
                        )
                    );
                    if !fixed_exclusion {
                        for frame in stack.iter_mut() {
                            frame.partial = true;
                        }
                    }
                    let code = match decision {
                        ReadDecision::Skip(reason) => reason.into(),
                        _ => ScanDiagnosticCode::UnsupportedFilesystem,
                    };
                    self.diagnostic(owner, code)?;
                }
                ObservationOutcome::Duplicate {
                    directory_coverage: Some(coverage),
                    ..
                } if coverage != Coverage::CompleteWithinPolicy => {
                    for frame in stack.iter_mut() {
                        frame.partial = true;
                    }
                }
                ObservationOutcome::File if entry.allocated_bytes.is_none() => {
                    self.diagnostic(owner, ScanDiagnosticCode::AllocationUnknown)?;
                }
                _ => {}
            }
            self.publish(control, publish, false);
        }
        Ok(())
    }
    fn close_frame(&mut self, stack: &mut Vec<Frame>) {
        let frame = stack.pop().expect("active frame");
        self.accounting.finish_directory(
            frame.identity,
            if frame.partial {
                Coverage::Partial
            } else {
                Coverage::CompleteWithinPolicy
            },
        );
        if frame.partial {
            for ancestor in stack {
                ancestor.partial = true;
            }
        }
    }
}
