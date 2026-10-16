use dev_storage_core::AppHealth;
use dev_storage_persistence::{Database, PersistenceError};
use dev_storage_platform::PlatformAdapter;
use std::sync::Mutex;
use thiserror::Error;

mod activity;
mod scans;
mod workspaces;
pub use activity::{ActivityGate, WorkspaceChangeGuard};
pub use scans::{MetadataScanRunner, ScanRunner};

pub struct ApplicationService<A> {
    adapter: A,
    database: Mutex<Database>,
    activity: ActivityGate,
    scans: scans::ScanCoordinator,
}

impl<A: PlatformAdapter> ApplicationService<A> {
    pub fn new(adapter: A, database: Database) -> Self {
        Self {
            adapter,
            database: Mutex::new(database),
            activity: ActivityGate::default(),
            scans: scans::ScanCoordinator::new(None),
        }
    }

    pub fn health(&self) -> Result<AppHealth, ApplicationError> {
        Ok(AppHealth {
            app_name: "dev-storage-manager".to_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            platform: self.adapter.platform(),
            database_schema_version: self
                .database
                .lock()
                .map_err(|_| ApplicationError::Unavailable)?
                .schema_version()?,
        })
    }
}

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("application state is unavailable")]
    Unavailable,
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

#[cfg(test)]
mod tests {
    use super::ApplicationService;
    use dev_storage_core::Platform;
    use dev_storage_persistence::Database;
    use dev_storage_platform::PlatformAdapter;

    struct TestPlatform;

    impl PlatformAdapter for TestPlatform {
        fn scan_filesystem(&self) -> Box<dyn dev_storage_core::scanner::ScanFilesystem> {
            panic!("health-only fixture must not request a scan filesystem")
        }
        fn platform(&self) -> Platform {
            Platform::MacOs
        }

        fn validate_workspace_root(
            &self,
            _: &std::path::Path,
        ) -> Result<
            dev_storage_core::workspaces::ValidatedWorkspace,
            dev_storage_core::workspaces::WorkspaceFailure,
        > {
            panic!("health-only fixture must not validate workspaces")
        }

        fn check_workspace_root(
            &self,
            _: &dev_storage_core::workspaces::ValidatedWorkspace,
            _: i64,
        ) -> dev_storage_core::workspaces::AvailabilityObservation {
            panic!("health-only fixture must not check workspaces")
        }
    }

    #[test]
    fn health_combines_build_platform_and_schema_information() {
        let database = Database::open_in_memory().unwrap();
        let service = ApplicationService::new(TestPlatform, database);
        let health = service.health().unwrap();

        assert_eq!(health.app_name, "dev-storage-manager");
        assert_eq!(health.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(health.platform, Platform::MacOs);
        assert_eq!(health.database_schema_version, 2);
    }
}
