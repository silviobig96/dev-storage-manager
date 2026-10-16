use dev_storage_core::workspaces::{AvailabilityObservation, ValidatedWorkspace, WorkspaceFailure};
use dev_storage_core::Platform;
use std::path::Path;

use crate::PlatformAdapter;

#[cfg(target_os = "macos")]
mod workspaces;

#[cfg(target_os = "macos")]
mod scanner;

#[cfg(target_os = "macos")]
mod identity;

pub struct MacOsAdapter;

impl PlatformAdapter for MacOsAdapter {
    fn scan_filesystem(&self) -> Box<dyn crate::scanner::ScanFilesystem> {
        #[cfg(target_os = "macos")]
        {
            Box::new(scanner::NativeFilesystem::default())
        }
        #[cfg(not(target_os = "macos"))]
        {
            Box::new(crate::scanner::UnsupportedFilesystem)
        }
    }
    fn platform(&self) -> Platform {
        Platform::MacOs
    }

    fn validate_workspace_root(
        &self,
        selected: &Path,
    ) -> Result<ValidatedWorkspace, WorkspaceFailure> {
        #[cfg(target_os = "macos")]
        {
            workspaces::validate(selected)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = selected;
            Err(WorkspaceFailure {
                code: dev_storage_core::workspaces::WorkspaceErrorCode::IdentityUnavailable,
                existing_id: None,
            })
        }
    }

    fn check_workspace_root(&self, root: &ValidatedWorkspace, now: i64) -> AvailabilityObservation {
        #[cfg(target_os = "macos")]
        {
            workspaces::check(root, now)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = root;
            AvailabilityObservation {
                state: dev_storage_core::workspaces::AvailabilityState::Unknown,
                reason: Some(dev_storage_core::workspaces::AvailabilityReason::UnsupportedIdentity),
                checked_at: Some(now),
            }
        }
    }
}
