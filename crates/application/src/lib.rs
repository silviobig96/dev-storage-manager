use dev_storage_core::AppHealth;
use dev_storage_persistence::{Database, PersistenceError};
use dev_storage_platform::PlatformAdapter;
use thiserror::Error;

pub struct ApplicationService<A> {
    adapter: A,
    database: Database,
}

impl<A: PlatformAdapter> ApplicationService<A> {
    pub fn new(adapter: A, database: Database) -> Self {
        Self { adapter, database }
    }

    pub fn health(&self) -> Result<AppHealth, ApplicationError> {
        Ok(AppHealth {
            app_name: "dev-storage-manager".to_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            platform: self.adapter.platform(),
            database_schema_version: self.database.schema_version()?,
        })
    }
}

#[derive(Debug, Error)]
pub enum ApplicationError {
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
        fn platform(&self) -> Platform {
            Platform::MacOs
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
        assert_eq!(health.database_schema_version, 1);
    }
}
