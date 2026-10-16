use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use dev_storage_core::{
    scanner::{ScanErrorCode, ScanFailure},
    workspaces::{WorkspaceErrorCode, WorkspaceFailure},
};

#[derive(Default)]
pub struct ActivityGate(Arc<AtomicU8>);
pub struct WorkspaceChangeGuard(Arc<AtomicU8>);
pub(crate) struct ScanReservation(Arc<AtomicU8>);

impl ActivityGate {
    pub fn try_workspace_change(&self) -> Result<WorkspaceChangeGuard, WorkspaceFailure> {
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| WorkspaceChangeGuard(self.0.clone()))
            .map_err(|state| WorkspaceFailure {
                code: if state == 2 {
                    WorkspaceErrorCode::ScanInProgress
                } else {
                    WorkspaceErrorCode::OperationInProgress
                },
                existing_id: None,
            })
    }
    pub(crate) fn try_scan(&self) -> Result<ScanReservation, ScanFailure> {
        self.0
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ScanReservation(self.0.clone()))
            .map_err(|state| ScanFailure {
                code: if state == 2 {
                    ScanErrorCode::ScanBusy
                } else {
                    ScanErrorCode::WorkspaceOperationInProgress
                },
            })
    }
    pub(crate) fn owns(&self, guard: &WorkspaceChangeGuard) -> bool {
        Arc::ptr_eq(&self.0, &guard.0)
    }
}

impl Drop for WorkspaceChangeGuard {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}
impl Drop for ScanReservation {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dev_storage_core::{scanner::ScanErrorCode, workspaces::WorkspaceErrorCode};

    // Catches release on moved guard, simultaneous owners and failure to release on drop.
    #[test]
    fn scan_activity_serializes_owned_guards() {
        let gate = Arc::new(ActivityGate::default());
        let picker = gate.try_workspace_change().unwrap();
        assert_eq!(
            gate.try_scan().err().unwrap().code,
            ScanErrorCode::WorkspaceOperationInProgress
        );
        assert_eq!(
            gate.try_workspace_change().err().unwrap().code,
            WorkspaceErrorCode::OperationInProgress
        );
        let callback = move || drop(picker);
        assert!(gate.try_scan().is_err());
        callback();
        let scan = gate.try_scan().unwrap();
        assert_eq!(gate.try_scan().err().unwrap().code, ScanErrorCode::ScanBusy);
        assert_eq!(
            gate.try_workspace_change().err().unwrap().code,
            WorkspaceErrorCode::ScanInProgress
        );
        drop(scan);
        assert!(gate.try_workspace_change().is_ok());
    }
}
