use super::*;
use dev_storage_platform::MacOsAdapter;
use std::time::Instant;

fn terminal<A: PlatformAdapter>(service: &ApplicationService<A>) -> ScanSnapshot {
    let state = service.scans.shared.state.lock().unwrap();
    let (state, timeout) = service
        .scans
        .shared
        .changed
        .wait_timeout_while(state, Duration::from_secs(5), |s| {
            s.retained.as_ref().unwrap().snapshot.worker_active
        })
        .unwrap();
    assert!(!timeout.timed_out(), "metadata worker did not finish");
    state.retained.as_ref().unwrap().snapshot.clone()
}

// Catches a runner that never invokes native metadata, widens scope, or persists scan results.
#[cfg(target_os = "macos")]
#[test]
fn scan_native_integration_explicit_metadata_and_restart() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("selected");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(root.join("nested")).unwrap();
    std::fs::write(root.join("sentinel"), b"unchanged").unwrap();
    std::fs::write(root.join("nested/data"), b"abcde").unwrap();
    // A sibling outside the selected root must not contribute to measurements.
    std::fs::write(fixture.path().join("outside"), b"outside selected scope").unwrap();
    let path = fixture.path().join("fixture.sqlite");
    let open = || {
        ApplicationService::new_with_scan_runner(
            MacOsAdapter,
            Database::open(&path).unwrap(),
            Arc::new(MetadataScanRunner::new(MacOsAdapter.scan_filesystem())),
        )
    };
    let service = open();
    assert!(service.current_scan().unwrap().session.is_none());
    let id = match service
        .add_workspace(service.prepare_workspace(&root).unwrap(), 1)
        .unwrap()
    {
        WorkspaceInsertOutcome::Added(row) => row.id,
        _ => panic!("fixture added twice"),
    };
    service.list_workspaces().unwrap();
    service.set_workspace_enabled(id, false, 2).unwrap();
    service.set_workspace_enabled(id, true, 3).unwrap();
    service.check_workspace(id, 4).unwrap();
    assert!(service.current_scan().unwrap().session.is_none());
    let database_before = std::fs::read(&path).unwrap();
    for (number, mode) in [(1, ScanMode::Quick), (2, ScanMode::Deep)] {
        let accepted = service
            .start_scan(ScanRequest {
                request_id: ScanRequestId::try_from(format!(
                    "{number:08x}-1111-1111-1111-111111111111"
                ))
                .unwrap(),
                expected_epoch: service.current_scan().unwrap().epoch,
                workspace_ids: vec![id],
                mode,
            })
            .unwrap();
        let result = terminal(&service);
        assert_eq!(result.session_id, accepted.session_id);
        assert_eq!(result.mode, mode);
        assert_eq!(result.state, ScanState::Completed);
        assert!(!result.worker_active);
        assert!(result.finished_at.is_some());
        assert_eq!(result.metrics.observed_logical_bytes, DecimalQuantity(14));
        assert_eq!(result.metrics.unique_files, DecimalQuantity(2));
        assert_eq!(result.metrics.unique_directories, DecimalQuantity(2));
        assert_eq!(result.entry_attempts, DecimalQuantity(5));
        assert_eq!(result.coverage, Coverage::CompleteWithinPolicy);
        assert_eq!(std::fs::read(&path).unwrap(), database_before);
        assert_eq!(std::fs::read(root.join("sentinel")).unwrap(), b"unchanged");
        assert_eq!(std::fs::read(root.join("nested/data")).unwrap(), b"abcde");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
    }
    drop(service);
    let reopened = open();
    assert_eq!(reopened.health().unwrap().database_schema_version, 2);
    assert_eq!(reopened.list_workspaces().unwrap().workspaces.len(), 1);
    assert!(reopened.current_scan().unwrap().session.is_none());
    assert_eq!(std::fs::read(path).unwrap(), database_before);
}

struct SlowFilesystem {
    blocked: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
impl ScanFilesystem for SlowFilesystem {
    fn anchor_root(
        &self,
        root: &StoredWorkspace,
        control: &dyn ScanIoControl,
    ) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        if root.id.get() == 2 {
            self.blocked.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        Ok(Box::new(ScalarDirectory(false)))
    }
}
struct ScalarDirectory(bool);
fn identity(file_id: u64) -> ObjectIdentity {
    ObjectIdentity {
        volume_uuid: [1; 16],
        file_id,
        birth_seconds: 1,
        birth_nanoseconds: 0,
    }
}
impl ScanDirectory for ScalarDirectory {
    fn identity(&self) -> ObjectIdentity {
        identity(1)
    }
    fn next_entry(
        &mut self,
        control: &dyn ScanIoControl,
    ) -> Result<Option<ScanEntry>, ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        if self.0 {
            return Ok(None);
        }
        self.0 = true;
        Ok(Some(ScanEntry {
            name: "scalar".into(),
            identity: Some(identity(2)),
            facts: ReadFacts {
                kind: EntryKind::RegularFile,
                in_read_scope: true,
                filesystem_supported: true,
                mount: MountFact::Same,
                cloud: CloudFact::Materialized,
                root_too_broad: false,
            },
            logical_bytes: Some(9_007_199_254_740_993),
            allocated_bytes: None,
        }))
    }
    fn open_child(
        &mut self,
        _: &ScanEntry,
        _: &dyn ScanIoControl,
    ) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        panic!("no child")
    }
    fn verify_link(
        &self,
        _: &ScanEntry,
        _: &ObjectIdentity,
        _: &dyn ScanIoControl,
    ) -> Result<(), ScanIoError> {
        panic!("no child")
    }
    fn finish(&mut self, control: &dyn ScanIoControl) -> Result<(), ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)
    }
}

// Catches cancellation finalizing/releasing the gate while real engine I/O remains blocked.
#[test]
fn scan_native_integration_slow_adapter_retains_gate_and_scalar_snapshot() {
    let (blocked, entered) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let service = ApplicationService::new_with_scan_runner(
        Adapter,
        Database::open_in_memory().unwrap(),
        Arc::new(MetadataScanRunner::new(Box::new(SlowFilesystem {
            blocked,
            release: Mutex::new(released),
        }))),
    );
    let fixture = tempfile::tempdir().unwrap();
    let a = add(&service, fixture.path().join("a").to_str().unwrap());
    let b = add(&service, fixture.path().join("b").to_str().unwrap());
    let accepted = service
        .start_scan(request(&service, vec![a, b], 1))
        .unwrap();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let published = service.current_scan().unwrap().session.unwrap();
    assert_eq!(
        published.metrics.observed_logical_bytes,
        DecimalQuantity(9_007_199_254_740_993)
    );
    assert_eq!(
        serde_json::to_value(&published).unwrap()["metrics"]["observedLogicalBytes"],
        "9007199254740993"
    );
    let started = Instant::now();
    let cancelling = service.cancel_scan(&accepted.session_id).unwrap();
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(cancelling.state, ScanState::Cancelling);
    assert!(cancelling.worker_active);
    assert!(cancelling.finished_at.is_none());
    assert!(service.reserve_workspace_change().is_err());
    assert_eq!(
        service
            .start_scan(request(&service, vec![a], 2))
            .unwrap_err()
            .code,
        ScanErrorCode::ScanBusy
    );
    assert_eq!(service.current_scan().unwrap().session.unwrap(), cancelling);
    release.send(()).unwrap();
    let done = terminal(&service);
    assert_eq!(done.state, ScanState::Cancelled);
    assert!(!done.worker_active);
    assert!(done.finished_at.is_some());
    assert_eq!(
        done.metrics.observed_logical_bytes,
        published.metrics.observed_logical_bytes
    );
    assert!(service.reserve_workspace_change().is_ok());
}
