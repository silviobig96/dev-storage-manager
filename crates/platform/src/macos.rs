use dev_storage_core::Platform;

use crate::PlatformAdapter;

pub struct MacOsAdapter;

impl PlatformAdapter for MacOsAdapter {
    fn platform(&self) -> Platform {
        Platform::MacOs
    }
}
