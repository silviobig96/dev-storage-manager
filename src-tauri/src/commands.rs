use std::sync::Mutex;

use dev_storage_application::{ApplicationError, ApplicationService};
use dev_storage_core::AppHealth;
use dev_storage_platform::{MacOsAdapter, PlatformAdapter};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl From<ApplicationError> for CommandError {
    fn from(error: ApplicationError) -> Self {
        Self {
            code: "application_unavailable",
            message: error.to_string(),
        }
    }
}

fn health_payload<A: PlatformAdapter>(
    service: &ApplicationService<A>,
) -> Result<AppHealth, CommandError> {
    service.health().map_err(CommandError::from)
}

#[tauri::command]
pub fn get_app_health(
    state: State<'_, Mutex<ApplicationService<MacOsAdapter>>>,
) -> Result<AppHealth, CommandError> {
    let service = state.lock().map_err(|_| CommandError {
        code: "application_unavailable",
        message: "application state is unavailable".to_owned(),
    })?;

    health_payload(&service)
}

#[cfg(test)]
mod tests {
    use super::health_payload;
    use dev_storage_application::ApplicationService;
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
    fn command_adapter_returns_the_application_contract() {
        let service = ApplicationService::new(TestPlatform, Database::open_in_memory().unwrap());
        let payload = health_payload(&service).unwrap();
        assert_eq!(payload.database_schema_version, 1);
    }
}
