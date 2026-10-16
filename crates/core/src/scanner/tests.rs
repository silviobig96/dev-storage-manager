//! Synthetic metadata cursors. No fixture performs filesystem I/O.
use super::*;
use crate::workspaces::{DirectoryIdentity, StoredWorkspace, ValidatedWorkspace, WorkspaceId};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(super) fn identity(id: u64) -> ObjectIdentity {
    ObjectIdentity {
        volume_uuid: [1; 16],
        file_id: id,
        birth_seconds: 0,
        birth_nanoseconds: 0,
    }
}

pub(super) fn entry(id: u64, kind: EntryKind, bytes: u64) -> ScanEntry {
    ScanEntry {
        name: format!("entry-{id}").into(),
        identity: Some(identity(id)),
        facts: ReadFacts {
            kind,
            in_read_scope: true,
            filesystem_supported: true,
            mount: MountFact::Same,
            cloud: CloudFact::Materialized,
            root_too_broad: false,
        },
        logical_bytes: (kind == EntryKind::RegularFile).then_some(bytes),
        allocated_bytes: (kind == EntryKind::RegularFile).then_some(bytes),
    }
}

pub(super) fn root(id: i64, directory: u64) -> StoredWorkspace {
    StoredWorkspace {
        id: WorkspaceId::try_from(id).unwrap(),
        root: ValidatedWorkspace {
            selected_path: format!("/synthetic/{directory}"),
            canonical_path: format!("/synthetic/{directory}"),
            identity: DirectoryIdentity::new(directory.to_string()),
        },
        enabled: true,
        created_at: 0,
        updated_at: 0,
    }
}

#[derive(Default)]
pub(super) struct Counters {
    pub live: usize,
    pub peak: usize,
    pub events: Vec<String>,
}

#[derive(Default, Clone)]
pub(super) struct Directory {
    pub entries: Vec<Result<ScanEntry, ScanIoError>>,
    pub finish_error: Option<ScanIoError>,
    pub link_error: Option<ScanIoError>,
    pub panic_next: bool,
    pub on_next: Option<Arc<dyn Fn(usize) + Send + Sync>>,
}

#[derive(Default, Clone)]
pub(super) struct Filesystem {
    pub directories: BTreeMap<u64, Directory>,
    pub counters: Arc<Mutex<Counters>>,
}

impl Filesystem {
    pub fn directory(&mut self, id: u64, entries: Vec<ScanEntry>) {
        self.directories.insert(
            id,
            Directory {
                entries: entries.into_iter().map(Ok).collect(),
                ..Directory::default()
            },
        );
    }
    fn open(&self, id: u64) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        let data = self
            .directories
            .get(&id)
            .ok_or(ScanIoError::Diagnostic(ScanDiagnosticCode::RootMissing))?
            .clone();
        let mut counters = self.counters.lock().unwrap();
        counters.live += 1;
        counters.peak = counters.peak.max(counters.live);
        counters.events.push(format!("open:{id}"));
        Ok(Box::new(Cursor {
            fs: self.clone(),
            id,
            data,
            position: 0,
        }))
    }
}

impl ScanFilesystem for Filesystem {
    fn anchor_root(
        &self,
        root: &StoredWorkspace,
        control: &dyn ScanIoControl,
    ) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        // Native root acquisition temporarily retains selected + canonical + home.
        {
            let mut counters = self.counters.lock().unwrap();
            assert_eq!(
                counters.live, 0,
                "previous root must close before next anchoring"
            );
            counters.peak = counters.peak.max(counters.live + 3);
            counters.events.push(format!("anchor:{}", root.id.get()));
        }
        self.open(root.root.identity.as_str().parse().unwrap())
    }
}

struct Cursor {
    fs: Filesystem,
    id: u64,
    data: Directory,
    position: usize,
}
impl Drop for Cursor {
    fn drop(&mut self) {
        let mut counters = self.fs.counters.lock().unwrap();
        counters.live -= 1;
        counters.events.push(format!("close:{}", self.id));
    }
}
impl ScanDirectory for Cursor {
    fn identity(&self) -> ObjectIdentity {
        identity(self.id)
    }
    fn next_entry(
        &mut self,
        control: &dyn ScanIoControl,
    ) -> Result<Option<ScanEntry>, ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        self.fs
            .counters
            .lock()
            .unwrap()
            .events
            .push(format!("next:{}", self.id));
        if let Some(hook) = &self.data.on_next {
            hook(self.position);
        }
        assert!(!self.data.panic_next, "private synthetic panic content");
        let result = self.data.entries.get(self.position).cloned().transpose();
        self.position += 1;
        result
    }
    fn open_child(
        &mut self,
        entry: &ScanEntry,
        control: &dyn ScanIoControl,
    ) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        self.fs.open(entry.identity.unwrap().file_id)
    }
    fn verify_link(
        &self,
        entry: &ScanEntry,
        child: &ObjectIdentity,
        control: &dyn ScanIoControl,
    ) -> Result<(), ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        assert_eq!(entry.identity, Some(*child));
        self.fs
            .counters
            .lock()
            .unwrap()
            .events
            .push(format!("verify:{}:{}", self.id, child.file_id));
        self.data.link_error.map_or(Ok(()), Err)
    }
    fn finish(&mut self, control: &dyn ScanIoControl) -> Result<(), ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        self.fs
            .counters
            .lock()
            .unwrap()
            .events
            .push(format!("finish:{}", self.id));
        self.data.finish_error.map_or(Ok(()), Err)
    }
}

// Detects fabricated empty/unavailable totals, omitted ordinary names, and root FD accumulation.
#[test]
fn scan_empty_and_multiple_roots() {
    let mut fs = Filesystem::default();
    fs.directory(1, vec![]);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Completed);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(0)
    );
    assert_eq!(report.progress.coverage, Coverage::CompleteWithinPolicy);
    let mut git = entry(10, EntryKind::RegularFile, 3);
    git.name = ".git".into();
    let mut modules = entry(20, EntryKind::RegularFile, 5);
    modules.name = "node_modules".into();
    fs.directory(1, vec![git]);
    fs.directory(2, vec![modules]);
    let report = scan(
        vec![root(2, 2), root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Completed);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(8)
    );
    assert_eq!(report.progress.metrics.unique_files, DecimalQuantity(2));
    assert_eq!(report.progress.workspaces[0].workspace_id.get(), 1);
    assert_eq!(fs.counters.lock().unwrap().live, 0);
    assert_eq!(fs.counters.lock().unwrap().peak, 3);
}

// Detects request-order ownership and enumeration-order dependent complete totals.
#[test]
fn scan_nesting_and_order() {
    for reverse in [false, true] {
        let mut fs = Filesystem::default();
        let mut entries = vec![
            entry(2, EntryKind::Directory, 0),
            entry(10, EntryKind::RegularFile, 3),
        ];
        if reverse {
            entries.reverse();
        }
        fs.directory(1, entries);
        fs.directory(
            2,
            vec![
                entry(10, EntryKind::RegularFile, 3),
                entry(20, EntryKind::RegularFile, 5),
            ],
        );
        let report = scan(
            vec![root(9, 2), root(1, 1)],
            ScanMode::Quick,
            &fs,
            &ScanControl::new(ScanMode::Quick),
            &mut |_| {},
        );
        assert_eq!(report.state, ScanState::Completed);
        assert_eq!(
            report.progress.metrics.observed_logical_bytes,
            DecimalQuantity(8)
        );
        assert_eq!(
            report.progress.workspaces[0]
                .metrics
                .as_ref()
                .unwrap()
                .observed_logical_bytes,
            DecimalQuantity(8)
        );
        assert_eq!(
            report.progress.workspaces[1]
                .metrics
                .as_ref()
                .unwrap()
                .observed_logical_bytes,
            DecimalQuantity(0)
        );
        assert_eq!(
            report.progress.workspaces[1].covered_by_workspace_ids,
            vec![root(1, 1).id]
        );
        let events = &fs.counters.lock().unwrap().events;
        for (index, event) in events.iter().enumerate() {
            if event == "next:2" {
                assert_eq!(events[index - 1], "verify:1:2");
            }
        }
    }
}

fn limited(
    fs: &Filesystem,
    roots: Vec<StoredWorkspace>,
    depth: usize,
    attempts: u64,
    records: u64,
    fds: usize,
) -> ScanReport {
    super::engine::scan_with_limits(
        roots,
        ScanLimits::for_test(depth, attempts, records, fds),
        fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    )
}

// Detects lost ancestor partial coverage, subtree retries and session stops on depth alone.
#[test]
fn scan_limits_depth_propagates_to_later_duplicates() {
    let mut fs = Filesystem::default();
    fs.directory(
        1,
        vec![
            entry(2, EntryKind::Directory, 0),
            entry(10, EntryKind::RegularFile, 3),
        ],
    );
    fs.directory(2, vec![entry(3, EntryKind::Directory, 0)]);
    fs.directory(3, vec![entry(20, EntryKind::RegularFile, 5)]);
    let report = limited(
        &fs,
        vec![root(1, 1), root(2, 2), root(3, 3)],
        1,
        100,
        100,
        36,
    );
    assert_eq!(report.state, ScanState::Partial);
    assert_eq!(report.stop_reason, None);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(3)
    );
    assert_eq!(
        report.progress.metrics.unmeasured_entries_lower_bound,
        DecimalQuantity(1)
    );
    assert_eq!(report.progress.metrics.unmeasured_bytes, None);
    assert!(report
        .progress
        .workspaces
        .iter()
        .all(|row| row.coverage == Coverage::Partial));
    assert!(report
        .progress
        .diagnostics
        .iter()
        .any(|d| d.code == ScanDiagnosticCode::DepthLimit));
    assert!(!fs
        .counters
        .lock()
        .unwrap()
        .events
        .iter()
        .any(|e| e == "next:3"));
}

// Detects off-by-one work admission, lost prefixes and exceeding live descriptor caps.
#[test]
fn scan_limits_and_partial() {
    for (attempts, records, fds, reason, bytes) in [
        (2, 100, 36, StopReason::EntryLimit, 8),
        (100, 2, 36, StopReason::IdentityLimit, 3),
        (100, 100, 2, StopReason::DescriptorLimit, 0),
    ] {
        let mut fs = Filesystem::default();
        fs.directory(
            1,
            vec![
                entry(10, EntryKind::RegularFile, 3),
                entry(20, EntryKind::RegularFile, 5),
                entry(30, EntryKind::RegularFile, 7),
            ],
        );
        let report = limited(&fs, vec![root(1, 1)], 8, attempts, records, fds);
        assert_eq!(report.stop_reason, Some(reason));
        assert_eq!(
            report.progress.metrics.observed_logical_bytes,
            DecimalQuantity(bytes)
        );
        assert_ne!(report.progress.coverage, Coverage::CompleteWithinPolicy);
        assert_eq!(report.progress.metrics.unmeasured_bytes, None);
        if reason == StopReason::IdentityLimit {
            assert_eq!(
                report.progress.metrics.unmeasured_entries_lower_bound,
                DecimalQuantity(1)
            );
        }
        assert!(fs.counters.lock().unwrap().peak <= fds);
        assert_eq!(fs.counters.lock().unwrap().live, 0);
    }
    let mut fs = Filesystem::default();
    for id in 1..=5 {
        fs.directory(id, vec![entry(id + 1, EntryKind::Directory, 0)]);
    }
    let report = limited(&fs, vec![root(1, 1)], 8, 100, 100, 3);
    assert_eq!(report.stop_reason, Some(StopReason::DescriptorLimit));
    assert_eq!(
        report.progress.metrics.unmeasured_entries_lower_bound,
        DecimalQuantity(1)
    );
    assert_eq!(fs.counters.lock().unwrap().peak, 3);
    assert_eq!(fs.counters.lock().unwrap().live, 0);
}

// Detects treating policy exclusions or unknown allocation as lost logical traversal.
#[test]
fn scan_policy_and_allocation_coverage() {
    let mut fs = Filesystem::default();
    let mut unknown = entry(10, EntryKind::RegularFile, 9);
    unknown.allocated_bytes = None;
    fs.directory(
        1,
        vec![
            entry(2, EntryKind::Symlink, 50),
            entry(3, EntryKind::Special, 60),
            unknown,
        ],
    );
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(9)
    );
    assert_eq!(
        report.progress.metrics.logical_coverage,
        Coverage::CompleteWithinPolicy
    );
    assert_eq!(
        report.progress.metrics.allocated_coverage,
        Coverage::Unavailable
    );
    assert_eq!(
        report.progress.metrics.policy_excluded_entries,
        DecimalQuantity(2)
    );
    assert_eq!(report.state, ScanState::Partial);
    let mut cloud = entry(4, EntryKind::Directory, 0);
    cloud.facts.cloud = CloudFact::Dataless;
    fs.directory(1, vec![cloud]);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.progress.metrics.logical_coverage, Coverage::Partial);
    assert_eq!(report.progress.metrics.unmeasured_bytes, None);
    assert_eq!(fs.counters.lock().unwrap().live, 0);
}

// Detects errors becoming zero-byte success, missing RAII on panic, and retrying failed branches.
#[test]
fn scan_cancel_and_errors() {
    let mut fs = Filesystem::default();
    fs.directory(1, vec![entry(10, EntryKind::RegularFile, 3)]);
    let control = ScanControl::new(ScanMode::Quick);
    control.cancel();
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &control,
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Cancelled);
    assert_eq!(report.progress.workspaces[0].metrics, None);
    assert!(fs.counters.lock().unwrap().events.is_empty());
    let report = scan(
        vec![root(1, 99), root(2, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Partial);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(3)
    );
    assert_eq!(report.progress.workspaces[0].metrics, None);
    let report = scan(
        vec![root(1, 99)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Error);
    assert_eq!(report.progress.coverage, Coverage::Unavailable);
    for kind in 0..3 {
        let dir = fs.directories.get_mut(&1).unwrap();
        dir.finish_error = (kind == 0).then_some(ScanIoError::Diagnostic(
            ScanDiagnosticCode::ChangedDuringScan,
        ));
        dir.entries = vec![Ok(entry(10, EntryKind::RegularFile, 3))];
        if kind == 1 {
            dir.entries.push(Err(ScanIoError::Diagnostic(
                ScanDiagnosticCode::EntryDenied,
            )));
        }
        dir.panic_next = kind == 2;
        let report = scan(
            vec![root(1, 1)],
            ScanMode::Quick,
            &fs,
            &ScanControl::new(ScanMode::Quick),
            &mut |_| {},
        );
        assert_ne!(report.progress.coverage, Coverage::CompleteWithinPolicy);
        assert_eq!(
            report.progress.metrics.unmeasured_entries_lower_bound,
            DecimalQuantity(0)
        );
        assert_eq!(fs.counters.lock().unwrap().live, 0);
        if kind == 2 {
            assert_eq!(report.state, ScanState::Error);
            assert_eq!(
                report.failure.unwrap().code,
                ScanErrorCode::ApplicationUnavailable
            );
            assert!(!format!("{:?}", report.progress).contains("private synthetic"));
        }
    }
}

#[derive(Default)]
struct Clock(std::sync::atomic::AtomicU64);
impl MonotonicClock for Clock {
    fn now(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.0.load(std::sync::atomic::Ordering::SeqCst))
    }
}

// Detects accepting empty/disabled/duplicate scopes or treating an unrequested child as an exclusion.
#[test]
fn scan_scope_validation_and_unrequested_descendants() {
    let mut fs = Filesystem::default();
    let mut disabled = root(2, 2);
    disabled.enabled = false;
    for (roots, code) in [
        (vec![], ScanErrorCode::ScopeEmpty),
        (vec![root(1, 1), root(1, 1)], ScanErrorCode::InvalidRequest),
        (vec![disabled], ScanErrorCode::WorkspaceDisabled),
        (
            (1..=33).map(|id| root(id, id as u64)).collect(),
            ScanErrorCode::ScopeTooLarge,
        ),
    ] {
        let report = scan(
            roots,
            ScanMode::Quick,
            &fs,
            &ScanControl::new(ScanMode::Quick),
            &mut |_| {},
        );
        assert_eq!(report.state, ScanState::Error);
        assert_eq!(report.failure.unwrap().code, code);
        assert_eq!(report.progress.coverage, Coverage::Unavailable);
    }
    assert!(fs.counters.lock().unwrap().events.is_empty());
    fs.directory(1, vec![entry(2, EntryKind::Directory, 0)]);
    fs.directory(2, vec![entry(20, EntryKind::RegularFile, 5)]);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Completed);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(5)
    );
}

// Detects continuing a grandchild after an ancestor was moved from its retained parent.
#[test]
fn scan_parent_chain_verified_before_descendant_continuation() {
    let mut fs = Filesystem::default();
    fs.directory(
        1,
        vec![
            entry(2, EntryKind::Directory, 0),
            entry(40, EntryKind::RegularFile, 7),
        ],
    );
    fs.directory(2, vec![entry(3, EntryKind::Directory, 0)]);
    fs.directory(3, vec![entry(30, EntryKind::RegularFile, 9)]);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Completed);
    let counters = fs.counters.lock().unwrap();
    let first_grandchild = counters
        .events
        .iter()
        .position(|event| event == "next:3")
        .unwrap();
    assert_eq!(
        &counters.events[first_grandchild - 2..first_grandchild],
        &["verify:1:2", "verify:2:3"]
    );
}

// Detects early terminal reporting/descriptor release during a blocked call and lost prefix counts.
#[test]
fn scan_cancel_blocked_call_waits_for_cleanup() {
    use std::sync::{mpsc, Barrier};
    let mut fs = Filesystem::default();
    fs.directory(
        1,
        vec![
            entry(10, EntryKind::RegularFile, 3),
            entry(20, EntryKind::RegularFile, 5),
        ],
    );
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let (entered_worker, release_worker) = (entered.clone(), release.clone());
    fs.directories.get_mut(&1).unwrap().on_next = Some(Arc::new(move |position| {
        if position == 1 {
            entered_worker.wait();
            release_worker.wait();
        }
    }));
    let control = Arc::new(ScanControl::new(ScanMode::Quick));
    let (tx, rx) = mpsc::channel();
    let (worker_fs, worker_control) = (fs.clone(), control.clone());
    let worker = std::thread::spawn(move || {
        tx.send(scan(
            vec![root(1, 1)],
            ScanMode::Quick,
            &worker_fs,
            &worker_control,
            &mut |_| {},
        ))
        .unwrap();
    });
    entered.wait();
    assert_eq!(control.cancel(), StopReason::Cancelled);
    assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(fs.counters.lock().unwrap().live, 1);
    release.wait();
    let report = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
    worker.join().unwrap();
    assert_eq!(report.state, ScanState::Cancelled);
    assert_eq!(report.progress.entry_attempts, DecimalQuantity(2));
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(3)
    );
    assert_eq!(fs.counters.lock().unwrap().live, 0);
    assert_eq!(
        fs.counters
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|event| event.starts_with("next:"))
            .count(),
        2
    );
}

// Detects resetting the deadline after validation or missing a post-I/O checkpoint.
#[test]
fn scan_deadline_and_progress_are_cooperative_and_bounded() {
    let mut fs = Filesystem::default();
    fs.directory(
        1,
        (10..1010)
            .map(|id| entry(id, EntryKind::RegularFile, 1))
            .collect(),
    );
    let clock = Arc::new(Clock::default());
    let control = ScanControl::with_clock(ScanMode::Quick, clock.clone());
    let mut publications = 0;
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &control,
        &mut |_| publications += 1,
    );
    assert_eq!(report.progress.entry_attempts, DecimalQuantity(1001));
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(1000)
    );
    assert!(
        publications <= 3,
        "stationary clock must not publish each entry"
    );
    let control = ScanControl::with_clock(ScanMode::Quick, clock.clone());
    let clock_hook = clock.clone();
    fs.directories.get_mut(&1).unwrap().on_next = Some(Arc::new(move |position| {
        if position == 1 {
            clock_hook
                .0
                .store(5000, std::sync::atomic::Ordering::SeqCst);
        }
    }));
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &control,
        &mut |_| {},
    );
    assert_eq!(report.stop_reason, Some(StopReason::TimeBudget));
    assert_eq!(report.progress.entry_attempts, DecimalQuantity(2));
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(1)
    );
    assert_eq!(control.cancel(), StopReason::TimeBudget);
    assert_eq!(
        ScanIoControl::checkpoint(&control),
        Err(StopReason::TimeBudget)
    );
    assert_eq!(fs.counters.lock().unwrap().live, 0);
}

// Detects abandoning ancestor siblings or treating failed child validation as complete.
#[test]
fn scan_child_errors_retain_siblings_and_later_partial_owners() {
    for code in [
        ScanDiagnosticCode::EntryDenied,
        ScanDiagnosticCode::EntryMissing,
        ScanDiagnosticCode::ChangedDuringScan,
    ] {
        let mut fs = Filesystem::default();
        fs.directory(
            1,
            vec![
                entry(2, EntryKind::Directory, 0),
                entry(10, EntryKind::RegularFile, 3),
            ],
        );
        fs.directory(2, vec![]);
        fs.directories.get_mut(&1).unwrap().link_error = Some(ScanIoError::Diagnostic(code));
        let report = scan(
            vec![root(1, 1), root(2, 2)],
            ScanMode::Quick,
            &fs,
            &ScanControl::new(ScanMode::Quick),
            &mut |_| {},
        );
        assert_eq!(report.state, ScanState::Partial);
        assert_eq!(
            report.progress.metrics.observed_logical_bytes,
            DecimalQuantity(3)
        );
        assert!(report
            .progress
            .workspaces
            .iter()
            .all(|row| row.coverage == Coverage::Partial));
        assert!(report
            .progress
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
        assert!(!fs
            .counters
            .lock()
            .unwrap()
            .events
            .iter()
            .any(|event| event == "next:2"));
    }
}

// Detects a complete-looking cancelled result when cancel wins after the final root callback.
#[test]
fn scan_cancel_at_final_publication_keeps_observations_partial() {
    let mut fs = Filesystem::default();
    fs.directory(1, vec![entry(10, EntryKind::RegularFile, 3)]);
    let control = ScanControl::new(ScanMode::Quick);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &control,
        &mut |progress| {
            if progress.workspaces[0].state == WorkspaceScanState::Completed {
                control.cancel();
            }
        },
    );
    assert_eq!(report.state, ScanState::Cancelled);
    assert_eq!(report.progress.coverage, Coverage::Partial);
    assert_eq!(report.progress.metrics.logical_coverage, Coverage::Partial);
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(3)
    );
    assert_eq!(
        report.progress.workspaces[0].state,
        WorkspaceScanState::Cancelled
    );
}

// Detects per-root descriptor accumulation and frontier/list-sized growth in the engine.
#[test]
fn scan_many_roots_and_deep_stack_stay_bounded() {
    let mut fs = Filesystem::default();
    let mut roots = Vec::new();
    for id in 1..=32 {
        fs.directory(id, vec![]);
        roots.push(root(id as i64, id));
    }
    let report = scan(
        roots,
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Completed);
    assert_eq!(report.progress.entry_attempts, DecimalQuantity(32));
    assert_eq!(fs.counters.lock().unwrap().peak, 3);
    let mut fs = Filesystem::default();
    for id in 1..=34 {
        fs.directory(id, vec![entry(id + 1, EntryKind::Directory, 0)]);
    }
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Deep,
        &fs,
        &ScanControl::new(ScanMode::Deep),
        &mut |_| {},
    );
    assert_eq!(report.state, ScanState::Partial);
    assert_eq!(report.stop_reason, None);
    assert_eq!(fs.counters.lock().unwrap().peak, 33);
    assert_eq!(fs.counters.lock().unwrap().live, 0);
    assert_eq!(
        report.progress.metrics.unmeasured_entries_lower_bound,
        DecimalQuantity(1)
    );
}

// Detects loss of representable prefixes, ignored bounded diagnostics or missing external stop handling.
#[test]
fn scan_quantity_diagnostic_and_publication_stops() {
    let mut fs = Filesystem::default();
    fs.directory(
        1,
        vec![
            entry(10, EntryKind::RegularFile, u64::MAX),
            entry(20, EntryKind::RegularFile, 1),
        ],
    );
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.stop_reason, Some(StopReason::QuantityOverflow));
    assert_eq!(
        report.progress.metrics.observed_logical_bytes,
        DecimalQuantity(u64::MAX)
    );
    let control = ScanControl::new(ScanMode::Quick);
    let report = scan(
        vec![root(1, 1)],
        ScanMode::Quick,
        &fs,
        &control,
        &mut |_| {
            control.request_stop(StopReason::ResultLimit);
        },
    );
    assert_eq!(report.stop_reason, Some(StopReason::ResultLimit));
    assert_eq!(report.progress.entry_attempts, DecimalQuantity(0));
    let mut roots = Vec::new();
    for id in 1..=32 {
        let mut mount = entry(100 + id, EntryKind::Directory, 0);
        mount.facts.mount = MountFact::Boundary;
        let mut cloud = entry(200 + id, EntryKind::Directory, 0);
        cloud.facts.cloud = CloudFact::Dataless;
        let mut unknown = entry(300 + id, EntryKind::RegularFile, 1);
        unknown.allocated_bytes = None;
        fs.directory(
            id,
            vec![
                entry(400 + id, EntryKind::Symlink, 0),
                entry(500 + id, EntryKind::Special, 0),
                mount,
                cloud,
                unknown,
            ],
        );
        roots.push(root(id as i64, id));
    }
    let report = scan(
        roots,
        ScanMode::Quick,
        &fs,
        &ScanControl::new(ScanMode::Quick),
        &mut |_| {},
    );
    assert_eq!(report.stop_reason, Some(StopReason::DiagnosticLimit));
    assert_eq!(report.progress.diagnostics.len(), 128);
    assert_eq!(
        report.progress.diagnostics.last().unwrap().code,
        ScanDiagnosticCode::DiagnosticLimit
    );
    assert_eq!(fs.counters.lock().unwrap().live, 0);
}
