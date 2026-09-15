mod macos;

use dev_storage_core::Platform;
pub use macos::MacOsAdapter;

pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> Platform;
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
