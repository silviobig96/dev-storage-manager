mod macos;
pub mod scanner;

use dev_storage_core::workspaces::{AvailabilityObservation, ValidatedWorkspace, WorkspaceFailure};
use dev_storage_core::Platform;
pub use macos::MacOsAdapter;
use std::path::Path;

pub trait PlatformAdapter: Send + Sync {
    fn scan_filesystem(&self) -> Box<dyn scanner::ScanFilesystem>;
    fn platform(&self) -> Platform;
    fn validate_workspace_root(
        &self,
        selected: &Path,
    ) -> Result<ValidatedWorkspace, WorkspaceFailure>;
    fn check_workspace_root(&self, root: &ValidatedWorkspace, now: i64) -> AvailabilityObservation;
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("platform capability is unavailable: {0}")]
    CapabilityUnavailable(&'static str),
}

#[cfg(test)]
mod tests {
    use super::{MacOsAdapter, PlatformAdapter};
    use dev_storage_core::Platform;

    #[test]
    fn macos_adapter_reports_only_its_platform_identity() {
        assert_eq!(MacOsAdapter.platform(), Platform::MacOs);
    }
}
