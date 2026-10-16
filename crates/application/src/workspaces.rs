use std::{path::Path, sync::MutexGuard};

use dev_storage_core::workspaces::{
    known_overlaps, AvailabilityObservation, SetupState, ValidatedWorkspace, WorkspaceErrorCode,
    WorkspaceFailure, WorkspaceId, WorkspaceInsertOutcome, WorkspaceListing, WorkspaceRecord,
};
use dev_storage_persistence::Database;
use dev_storage_platform::PlatformAdapter;

use crate::ApplicationService;

fn unavailable() -> WorkspaceFailure {
    WorkspaceFailure {
        code: WorkspaceErrorCode::ApplicationUnavailable,
        existing_id: None,
    }
}

impl<A: PlatformAdapter> ApplicationService<A> {
    pub fn reserve_workspace_change(
        &self,
    ) -> Result<crate::WorkspaceChangeGuard, WorkspaceFailure> {
        self.activity.try_workspace_change()
    }
    pub fn commit_workspace_add(
        &self,
        guard: crate::WorkspaceChangeGuard,
        root: ValidatedWorkspace,
        now: i64,
    ) -> Result<WorkspaceInsertOutcome, WorkspaceFailure> {
        if !self.activity.owns(&guard) {
            return Err(unavailable());
        }
        self.workspace_database()?.insert_workspace(&root, now)
    }
    fn workspace_database(&self) -> Result<MutexGuard<'_, Database>, WorkspaceFailure> {
        self.database.lock().map_err(|_| unavailable())
    }

    pub fn list_workspaces(&self) -> Result<WorkspaceListing, WorkspaceFailure> {
        let configuration = self.workspace_database()?.workspace_configuration()?;
        Ok(WorkspaceListing {
            setup_state: configuration.setup_state,
            workspaces: configuration
                .records
                .iter()
                .map(WorkspaceRecord::from)
                .collect(),
            overlaps: known_overlaps(&configuration.records),
        })
    }

    /// Validate root metadata without holding the database lock. Only Rust can supply this input.
    pub fn prepare_workspace(
        &self,
        selected: &Path,
    ) -> Result<ValidatedWorkspace, WorkspaceFailure> {
        self.adapter.validate_workspace_root(selected)
    }

    /// Commit a successfully prepared root; validation workers never commit configuration.
    pub fn add_workspace(
        &self,
        root: ValidatedWorkspace,
        now: i64,
    ) -> Result<WorkspaceInsertOutcome, WorkspaceFailure> {
        self.commit_workspace_add(self.reserve_workspace_change()?, root, now)
    }

    pub fn remove_workspace(&self, id: WorkspaceId) -> Result<(), WorkspaceFailure> {
        let _guard = self.reserve_workspace_change()?;
        self.workspace_database()?.remove_workspace(id)
    }

    pub fn set_workspace_enabled(
        &self,
        id: WorkspaceId,
        enabled: bool,
        now: i64,
    ) -> Result<WorkspaceRecord, WorkspaceFailure> {
        let _guard = self.reserve_workspace_change()?;
        self.workspace_database()?
            .set_workspace_enabled(id, enabled, now)
    }

    pub fn check_workspace(
        &self,
        id: WorkspaceId,
        now: i64,
    ) -> Result<AvailabilityObservation, WorkspaceFailure> {
        let configuration = self.workspace_database()?.workspace_configuration()?;
        let stored = configuration
            .records
            .into_iter()
            .find(|record| record.id == id)
            .ok_or(WorkspaceFailure {
                code: WorkspaceErrorCode::WorkspaceNotFound,
                existing_id: None,
            })?;
        // The owned snapshot outlives the short DB lock; a slow OS call cannot block health/list.
        Ok(self.adapter.check_workspace_root(&stored.root, now))
    }

    pub fn skip_workspace_setup(&self) -> Result<SetupState, WorkspaceFailure> {
        let _guard = self.reserve_workspace_change()?;
        self.workspace_database()?.skip_workspace_setup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dev_storage_core::{
        workspaces::{AvailabilityReason, AvailabilityState, DirectoryIdentity},
        Platform,
    };
    use dev_storage_persistence::Database;
    use std::{
        fs,
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc, Arc, Condvar, Mutex,
        },
        thread,
        time::Duration,
    };

    #[derive(Clone, Default)]
    struct TestAdapter {
        validations: Arc<AtomicUsize>,
        checks: Arc<AtomicUsize>,
        failure: Option<WorkspaceErrorCode>,
        checked: Arc<Mutex<Option<(ValidatedWorkspace, i64)>>>,
        blocking: Option<(mpsc::Sender<()>, Arc<MetadataBarrier>)>,
    }

    // A one-way test barrier can release even if a broken check never reaches metadata.
    #[derive(Default)]
    struct MetadataBarrier {
        released: Mutex<bool>,
        changed: Condvar,
    }

    impl MetadataBarrier {
        fn wait(&self) {
            let _released = self
                .changed
                .wait_while(self.released.lock().unwrap(), |released| !*released)
                .unwrap();
        }

        fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.changed.notify_all();
        }
    }

    fn root(path: &str) -> ValidatedWorkspace {
        ValidatedWorkspace {
            selected_path: path.into(),
            canonical_path: format!("/resolved{path}"),
            identity: DirectoryIdentity::new(format!("fixture:{path}")),
        }
    }

    impl PlatformAdapter for TestAdapter {
        fn scan_filesystem(&self) -> Box<dyn dev_storage_core::scanner::ScanFilesystem> {
            panic!("workspace-only fixture must not request a scan filesystem")
        }
        fn platform(&self) -> Platform {
            Platform::MacOs
        }
        fn validate_workspace_root(
            &self,
            selected: &Path,
        ) -> Result<ValidatedWorkspace, WorkspaceFailure> {
            self.validations.fetch_add(1, Ordering::SeqCst);
            match self.failure {
                Some(code) => Err(WorkspaceFailure {
                    code,
                    existing_id: None,
                }),
                None => Ok(root(selected.to_str().unwrap())),
            }
        }
        fn check_workspace_root(
            &self,
            root: &ValidatedWorkspace,
            now: i64,
        ) -> AvailabilityObservation {
            self.checks.fetch_add(1, Ordering::SeqCst);
            *self.checked.lock().unwrap() = Some((root.clone(), now));
            if let Some((entered, release)) = &self.blocking {
                entered.send(()).unwrap();
                release.wait();
            }
            AvailabilityObservation {
                state: AvailabilityState::Unknown,
                reason: Some(AvailabilityReason::IdentityChanged),
                checked_at: Some(now),
            }
        }
    }

    fn added(outcome: WorkspaceInsertOutcome) -> WorkspaceRecord {
        match outcome {
            WorkspaceInsertOutcome::Added(record) => record,
            other => panic!("expected added, got {other:?}"),
        }
    }

    // Catches extra platform work, missing persistence, bad ID/root forwarding, and lost overlap records.
    #[test]
    fn workspace_use_cases() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.db");
        let adapter = TestAdapter::default();
        let service = ApplicationService::new(adapter.clone(), Database::open(&path).unwrap());
        let empty = service.list_workspaces().unwrap();
        assert_eq!(empty.setup_state, SetupState::Undecided);
        assert!(empty.workspaces.is_empty());
        assert_eq!(service.skip_workspace_setup().unwrap(), SetupState::Skipped);
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);

        let validated = service
            .prepare_workspace(Path::new("/fixture/root"))
            .unwrap();
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 1);
        assert!(service.list_workspaces().unwrap().workspaces.is_empty());
        let parent = added(service.add_workspace(validated.clone(), 100).unwrap());
        let child = added(
            service
                .add_workspace(root("/fixture/root/nested"), 200)
                .unwrap(),
        );
        let listing = service.list_workspaces().unwrap();
        assert_eq!(listing.setup_state, SetupState::Configured);
        assert_eq!(listing.workspaces, vec![parent.clone(), child.clone()]);
        assert_eq!(listing.overlaps.len(), 1);
        assert_eq!(listing.overlaps[0].parent_id, parent.id);
        assert_eq!(listing.overlaps[0].child_id, child.id);
        let disabled = service
            .set_workspace_enabled(parent.id, false, 300)
            .unwrap();
        assert!(!disabled.enabled);
        assert_eq!(disabled.updated_at, 300);
        assert_eq!(
            service.skip_workspace_setup().unwrap(),
            SetupState::Configured
        );
        service.remove_workspace(child.id).unwrap();
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);

        let before = service.list_workspaces().unwrap();
        let observation = service.check_workspace(parent.id, 400).unwrap();
        assert_eq!(observation.state, AvailabilityState::Unknown);
        assert_eq!(
            observation.reason,
            Some(AvailabilityReason::IdentityChanged)
        );
        assert_eq!(observation.checked_at, Some(400));
        assert_eq!(*adapter.checked.lock().unwrap(), Some((validated, 400)));
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 1);
        assert_eq!(service.list_workspaces().unwrap(), before);
        let absent = WorkspaceId::try_from(999).unwrap();
        assert_eq!(
            service.check_workspace(absent, 500).unwrap_err().code,
            WorkspaceErrorCode::WorkspaceNotFound
        );
        assert_eq!(
            service.remove_workspace(absent).unwrap_err().code,
            WorkspaceErrorCode::WorkspaceNotFound
        );
        assert_eq!(
            service
                .set_workspace_enabled(absent, false, 500)
                .unwrap_err()
                .code,
            WorkspaceErrorCode::WorkspaceNotFound
        );
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 1);
        drop(service);
        let reopened = Database::open(&path)
            .unwrap()
            .workspace_configuration()
            .unwrap();
        assert_eq!(reopened.records.len(), 1);
        assert_eq!(reopened.records[0].id, parent.id);
        assert!(!reopened.records[0].enabled);
    }

    // Catches any save or setup mutation on failed prepare, duplicate, or changed path identity.
    #[test]
    fn workspace_validation_failure_never_saves() {
        for code in [
            WorkspaceErrorCode::InvalidSelection,
            WorkspaceErrorCode::UnsupportedPathEncoding,
            WorkspaceErrorCode::NotDirectory,
            WorkspaceErrorCode::FolderMissing,
            WorkspaceErrorCode::AccessDenied,
            WorkspaceErrorCode::IdentityUnavailable,
            WorkspaceErrorCode::LocationChanged,
            WorkspaceErrorCode::ValidationTimeout,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("application.db");
            let adapter = TestAdapter {
                failure: Some(code),
                ..Default::default()
            };
            let service = ApplicationService::new(adapter.clone(), Database::open(&path).unwrap());
            assert_eq!(
                service
                    .prepare_workspace(Path::new("/fixture/rejected"))
                    .unwrap_err(),
                WorkspaceFailure {
                    code,
                    existing_id: None
                }
            );
            assert_eq!(adapter.validations.load(Ordering::SeqCst), 1);
            assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);
            let stored = Database::open(&path)
                .unwrap()
                .workspace_configuration()
                .unwrap();
            assert_eq!(stored.setup_state, SetupState::Undecided);
            assert!(stored.records.is_empty());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.db");
        let adapter = TestAdapter::default();
        let service = ApplicationService::new(adapter.clone(), Database::open(&path).unwrap());
        let first = root("/fixture/registered");
        let record = added(service.add_workspace(first.clone(), 100).unwrap());
        service
            .set_workspace_enabled(record.id, false, 150)
            .unwrap();
        let before = service.list_workspaces().unwrap();
        assert_eq!(
            service.add_workspace(first.clone(), 200).unwrap(),
            WorkspaceInsertOutcome::Duplicate(record.id)
        );
        let replacement = ValidatedWorkspace {
            identity: DirectoryIdentity::new("replacement"),
            ..first
        };
        let error = service.add_workspace(replacement, 300).unwrap_err();
        assert_eq!(error.code, WorkspaceErrorCode::PathAlreadyConfigured);
        assert_eq!(error.existing_id, Some(record.id));
        assert_eq!(service.list_workspaces().unwrap(), before);
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);
    }

    // Catches leaking poisoned state or persistence errors, and calling metadata on DB failure.
    #[test]
    fn workspace_errors_are_sanitized_and_do_not_start_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let mut database = Database::open(&directory.path().join("application.db")).unwrap();
        database
            .set_metadata("workspace_setup_state_v1", "invalid-sentinel")
            .unwrap();
        let adapter = TestAdapter::default();
        let service = ApplicationService::new(adapter.clone(), database);
        let id = WorkspaceId::try_from(1).unwrap();
        assert_eq!(
            service.list_workspaces().unwrap_err().code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(
            service.skip_workspace_setup().unwrap_err().code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(
            service
                .add_workspace(root("/fixture/root"), 100)
                .unwrap_err()
                .code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(
            service.remove_workspace(id).unwrap_err().code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(
            service
                .set_workspace_enabled(id, false, 100)
                .unwrap_err()
                .code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(
            service.check_workspace(id, 100).unwrap_err().code,
            WorkspaceErrorCode::PersistenceUnavailable
        );
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = service.database.lock().unwrap();
            panic!("controlled poison fixture");
        }));
        assert!(poisoned.is_err());
        assert_eq!(
            service.list_workspaces().unwrap_err(),
            WorkspaceFailure {
                code: WorkspaceErrorCode::ApplicationUnavailable,
                existing_id: None
            }
        );
        assert_eq!(
            service.health().unwrap_err().to_string(),
            "application state is unavailable"
        );
    }

    // Catches accidental user-folder mutation and requiring live/enabled roots to remove configuration.
    #[test]
    fn workspace_remove_preserves_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("application.db");
        let workspace = directory.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        let sentinel = workspace.join("sentinel.txt");
        fs::write(&sentinel, b"keep me").unwrap();
        let adapter = TestAdapter::default();
        let service = ApplicationService::new(adapter.clone(), Database::open(&path).unwrap());
        let record = added(
            service
                .add_workspace(root(workspace.to_str().unwrap()), 100)
                .unwrap(),
        );
        service
            .set_workspace_enabled(record.id, false, 200)
            .unwrap();
        service.remove_workspace(record.id).unwrap();
        assert!(workspace.is_dir());
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep me");
        let missing = directory.path().join("never-created");
        let missing_record = added(
            service
                .add_workspace(root(missing.to_str().unwrap()), 300)
                .unwrap(),
        );
        service.remove_workspace(missing_record.id).unwrap();
        assert!(!missing.exists());
        assert_eq!(adapter.validations.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.checks.load(Ordering::SeqCst), 0);
        drop(service);
        let reopened = ApplicationService::new(adapter, Database::open(&path).unwrap())
            .list_workspaces()
            .unwrap();
        assert!(reopened.workspaces.is_empty());
        assert_eq!(reopened.setup_state, SetupState::Configured);
    }

    // Catches holding the database mutex across blocking OS metadata work.
    #[test]
    fn workspace_slow_check_does_not_lock_database() {
        let directory = tempfile::tempdir().unwrap();
        let mut database = Database::open(&directory.path().join("application.db")).unwrap();
        let record = added(
            database
                .insert_workspace(&root("/fixture/slow"), 100)
                .unwrap(),
        );
        let (entered_tx, entered_rx) = mpsc::channel();
        let release = Arc::new(MetadataBarrier::default());
        let adapter = TestAdapter {
            blocking: Some((entered_tx, release.clone())),
            ..Default::default()
        };
        let service = Arc::new(ApplicationService::new(adapter, database));
        let check_service = service.clone();
        let check = thread::spawn(move || check_service.check_workspace(record.id, 200));
        let entered = entered_rx.recv_timeout(Duration::from_secs(2));
        if entered.is_err() {
            release.release();
            check.join().unwrap().unwrap();
            panic!("check never reached platform metadata");
        }
        let (progress_tx, progress_rx) = mpsc::channel();
        let progress_service = service.clone();
        let progress = thread::spawn(move || {
            progress_tx
                .send((
                    progress_service.health(),
                    progress_service.list_workspaces(),
                ))
                .unwrap();
        });
        let result = progress_rx.recv_timeout(Duration::from_secs(2));
        // Always release the metadata worker, including when a regression blocks database reads.
        release.release();
        assert_eq!(
            check.join().unwrap().unwrap().state,
            AvailabilityState::Unknown
        );
        progress.join().unwrap();
        let (health, listing) = result.expect("health/list blocked by platform metadata");
        assert_eq!(health.unwrap().database_schema_version, 2);
        assert_eq!(listing.unwrap().workspaces, vec![record]);
    }
}
