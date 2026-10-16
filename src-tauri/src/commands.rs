use std::sync::Arc;

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
pub async fn get_app_health<R: tauri::Runtime>(
    webview: tauri::Webview<R>,
    state: State<'_, Arc<ApplicationService<MacOsAdapter>>>,
) -> Result<AppHealth, CommandError> {
    crate::workspaces::ensure_main(&webview)
        .map_err(|_| CommandError::from(ApplicationError::Unavailable))?;
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || health_payload(service.as_ref()))
        .await
        .map_err(|_| CommandError::from(ApplicationError::Unavailable))?
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
    fn command_adapter_returns_the_application_contract() {
        let service = ApplicationService::new(TestPlatform, Database::open_in_memory().unwrap());
        let payload = health_payload(&service).unwrap();
        assert_eq!(payload.database_schema_version, 2);
    }

    #[test]
    fn command_adapter_preserves_unavailable_error_contract() {
        let error =
            super::CommandError::from(dev_storage_application::ApplicationError::Unavailable);
        assert_eq!(error.code, "application_unavailable");
        assert_eq!(error.message, "application state is unavailable");
    }
}
