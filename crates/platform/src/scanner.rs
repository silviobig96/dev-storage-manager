//! Public adapter boundary re-exports the core's metadata-only cursor contracts.
pub use dev_storage_core::scanner::{ScanDirectory, ScanFilesystem, ScanIoControl, ScanIoError};

#[cfg(not(target_os = "macos"))]
pub(crate) struct UnsupportedFilesystem;

#[cfg(not(target_os = "macos"))]
impl ScanFilesystem for UnsupportedFilesystem {
    fn anchor_root(
        &self,
        _: &dev_storage_core::workspaces::StoredWorkspace,
        control: &dyn ScanIoControl,
    ) -> Result<Box<dyn ScanDirectory>, ScanIoError> {
        control.checkpoint().map_err(ScanIoError::Stopped)?;
        Err(ScanIoError::Diagnostic(
            dev_storage_core::scanner::ScanDiagnosticCode::UnsupportedFilesystem,
        ))
    }
}
