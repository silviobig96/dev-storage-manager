use super::*;
use crate::workspaces::WorkspaceId;

/// Closed-code groups only. One of the 128 slots is reserved for session stop.
pub struct ScanDiagnostics {
    groups: Vec<ScanDiagnostic>,
    ordinary_limit: usize,
    stop: Option<StopReason>,
}
impl ScanDiagnostics {
    pub fn new(limits: ScanLimits) -> Result<Self, ScanFailure> {
        let mut groups = Vec::new();
        groups
            .try_reserve_exact(limits.max_diagnostic_groups())
            .map_err(|_| ScanFailure {
                code: ScanErrorCode::ResourceUnavailable,
            })?;
        Ok(Self {
            groups,
            ordinary_limit: limits.max_diagnostic_groups() - 1,
            stop: None,
        })
    }

    pub fn record(
        &mut self,
        workspace_id: Option<WorkspaceId>,
        code: ScanDiagnosticCode,
        severity: DiagnosticSeverity,
    ) -> Result<(), StopReason> {
        if let Some(group) = self.groups.iter_mut().find(|group| {
            group.workspace_id == workspace_id && group.code == code && group.severity == severity
        }) {
            let Some(count) = group.count.0.checked_add(1) else {
                self.record_stop(StopReason::QuantityOverflow);
                return Err(StopReason::QuantityOverflow);
            };
            group.count = DecimalQuantity(count);
            return Ok(());
        }
        if let Some(reason) = self.stop {
            return Err(reason);
        }
        if self.groups.len() == self.ordinary_limit {
            self.record_stop(StopReason::DiagnosticLimit);
            return Err(StopReason::DiagnosticLimit);
        }
        self.groups.push(ScanDiagnostic {
            workspace_id,
            code,
            severity,
            count: DecimalQuantity(1),
        });
        Ok(())
    }

    /// First stop wins and always fits, even after all ordinary groups are used.
    /// Returns the effective reason if counting the stop itself overflows.
    pub fn record_stop(&mut self, mut reason: StopReason) -> StopReason {
        if let Some(stopped) = self.stop {
            return stopped;
        }
        loop {
            let code = reason.into();
            if let Some(group) = self.groups.iter_mut().find(|group| {
                group.workspace_id.is_none()
                    && group.code == code
                    && group.severity == DiagnosticSeverity::Warning
            }) {
                if let Some(count) = group.count.0.checked_add(1) {
                    group.count = DecimalQuantity(count);
                } else if reason != StopReason::QuantityOverflow {
                    reason = StopReason::QuantityOverflow;
                    continue;
                }
            } else {
                self.groups.push(ScanDiagnostic {
                    workspace_id: None,
                    code,
                    severity: DiagnosticSeverity::Warning,
                    count: DecimalQuantity(1),
                });
            }
            self.stop = Some(reason);
            return reason;
        }
    }

    pub fn snapshot(&self) -> Vec<ScanDiagnostic> {
        self.groups.clone()
    }
}

/// The publication layer must measure the *entire encoded DTO*, including JSON
/// escaping, before accepting it. Keep the last accepted snapshot when growth
/// fails, add ResultLimit/partial terminal fields, then check_terminal before
/// publication. Never truncate bytes, drop diagnostics, or publish complete on
/// failure. The 4 KiB reserve covers one closed-code stop group, terminal times,
/// revision growth and terminal state/coverage changes across at most 32 roots.
pub struct SnapshotSizeGuard {
    max_bytes: usize,
}
impl SnapshotSizeGuard {
    pub fn new(limits: ScanLimits) -> Self {
        Self {
            max_bytes: limits.max_snapshot_bytes(),
        }
    }
    pub fn check_snapshot(&self, encoded_bytes: usize) -> Result<(), StopReason> {
        if encoded_bytes <= self.max_bytes - 4096 {
            Ok(())
        } else {
            Err(StopReason::ResultLimit)
        }
    }
    pub fn check_terminal(&self, encoded_bytes: usize) -> Result<(), StopReason> {
        if encoded_bytes <= self.max_bytes {
            Ok(())
        } else {
            Err(StopReason::ResultLimit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Catches an unbounded group vector, an off-by-one reserved slot, or path sampling.
    #[test]
    fn scan_diagnostics_bound_groups_and_keep_terminal_reason() {
        let mut diagnostics = ScanDiagnostics::new(ScanLimits::for_mode(ScanMode::Quick)).unwrap();
        for id in 1..=127 {
            diagnostics
                .record(
                    Some(WorkspaceId::try_from(id).unwrap()),
                    ScanDiagnosticCode::EntryDenied,
                    DiagnosticSeverity::Warning,
                )
                .unwrap();
        }
        diagnostics
            .record(
                Some(WorkspaceId::try_from(1).unwrap()),
                ScanDiagnosticCode::EntryDenied,
                DiagnosticSeverity::Warning,
            )
            .unwrap();
        assert_eq!(diagnostics.snapshot()[0].count, DecimalQuantity(2));
        assert_eq!(
            diagnostics.record(
                None,
                ScanDiagnosticCode::EntryMissing,
                DiagnosticSeverity::Warning
            ),
            Err(StopReason::DiagnosticLimit)
        );
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.len(), 128);
        assert_eq!(snapshot[127].code, ScanDiagnosticCode::DiagnosticLimit);
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json[0]["count"], "2");
        assert_eq!(json[127]["count"], "1");
        for group in json.as_array().unwrap() {
            assert_eq!(group.as_object().unwrap().len(), 5);
            assert!(group.get("path").is_none());
            assert!(group.get("samples").is_none());
        }
    }

    // Catches publishing a full complete result with no room for a terminal reason.
    #[test]
    fn scan_diagnostics_bound_real_snapshot_encoding_reserves_terminal_data() {
        let limits = ScanLimits::for_mode(ScanMode::Quick);
        let guard = SnapshotSizeGuard::new(limits);
        let epoch = ScanEpoch::new(1, 1, 1);
        let mut snapshot = ScanSnapshot {
            session_id: ScanSessionId::new(&epoch, 1).unwrap(),
            request_id: ScanRequestId::try_from("11111111-1111-1111-1111-111111111111".to_owned())
                .unwrap(),
            epoch,
            mode: ScanMode::Quick,
            state: ScanState::Running,
            revision: DecimalQuantity(1),
            started_at: DecimalQuantity(0),
            finished_at: None,
            worker_active: true,
            stop_reason: None,
            failure: None,
            entry_attempts: DecimalQuantity(0),
            workspaces: vec![WorkspaceScanResult {
                workspace_id: WorkspaceId::try_from(7).unwrap(),
                display_name: "synthetic".into(),
                selected_path: String::new(),
                state: WorkspaceScanState::Running,
                metrics: None,
                coverage: Coverage::Partial,
                covered_by_workspace_ids: vec![],
            }],
            metrics: ScanAccounting::new(limits).unwrap().snapshot().metrics,
            coverage: Coverage::CompleteWithinPolicy,
            protection: measurement_protection(),
            diagnostics: vec![],
        };
        let baseline = serde_json::to_vec(&snapshot).unwrap().len();
        snapshot.workspaces[0].selected_path = "x".repeat(126_976 - baseline);
        assert_eq!(serde_json::to_vec(&snapshot).unwrap().len(), 126_976);
        assert_eq!(guard.check_snapshot(126_976), Ok(()));
        snapshot.workspaces[0].selected_path.push('x');
        assert_eq!(
            guard.check_snapshot(serde_json::to_vec(&snapshot).unwrap().len()),
            Err(StopReason::ResultLimit)
        );
        snapshot.workspaces[0].selected_path.pop();
        snapshot.state = ScanState::Partial;
        snapshot.coverage = Coverage::Partial;
        snapshot.finished_at = Some(DecimalQuantity(u64::MAX));
        snapshot.worker_active = false;
        snapshot.stop_reason = Some(StopReason::ResultLimit);
        snapshot.diagnostics.push(ScanDiagnostic {
            workspace_id: None,
            code: ScanDiagnosticCode::ResultLimit,
            severity: DiagnosticSeverity::Warning,
            count: DecimalQuantity(1),
        });
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        assert!(encoded.len() < 131_072);
        assert_eq!(guard.check_terminal(encoded.len()), Ok(()));
        assert_eq!(guard.check_terminal(131_073), Err(StopReason::ResultLimit));
        let decoded: ScanSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.state, ScanState::Partial);
        assert_eq!(decoded.stop_reason, Some(StopReason::ResultLimit));
    }

    // Catches duplicate grouping keys for a terminal event and wrapping group counts.
    #[test]
    fn scan_diagnostics_bound_stop_is_grouped_and_overflow_is_checked() {
        let mut diagnostics = ScanDiagnostics::new(ScanLimits::for_mode(ScanMode::Quick)).unwrap();
        diagnostics
            .record(
                None,
                ScanDiagnosticCode::Cancelled,
                DiagnosticSeverity::Warning,
            )
            .unwrap();
        diagnostics.record_stop(StopReason::Cancelled);
        diagnostics.record_stop(StopReason::DiagnosticLimit);
        assert_eq!(diagnostics.snapshot().len(), 1);
        assert_eq!(diagnostics.snapshot()[0].count, DecimalQuantity(2));
        let mut diagnostics = ScanDiagnostics::new(ScanLimits::for_mode(ScanMode::Quick)).unwrap();
        diagnostics
            .record(
                None,
                ScanDiagnosticCode::EntryDenied,
                DiagnosticSeverity::Warning,
            )
            .unwrap();
        diagnostics.groups[0].count = DecimalQuantity(u64::MAX);
        assert_eq!(
            diagnostics.record(
                None,
                ScanDiagnosticCode::EntryDenied,
                DiagnosticSeverity::Warning
            ),
            Err(StopReason::QuantityOverflow)
        );
        assert_eq!(diagnostics.snapshot()[0].count, DecimalQuantity(u64::MAX));
        assert_eq!(
            diagnostics.snapshot()[1].code,
            ScanDiagnosticCode::QuantityOverflow
        );
        let mut diagnostics = ScanDiagnostics::new(ScanLimits::for_mode(ScanMode::Quick)).unwrap();
        diagnostics
            .record(
                None,
                ScanDiagnosticCode::Cancelled,
                DiagnosticSeverity::Warning,
            )
            .unwrap();
        diagnostics.groups[0].count = DecimalQuantity(u64::MAX);
        diagnostics.record_stop(StopReason::Cancelled);
        assert_eq!(diagnostics.snapshot()[0].count, DecimalQuantity(u64::MAX));
        assert_eq!(
            diagnostics.snapshot()[1].code,
            ScanDiagnosticCode::QuantityOverflow
        );
    }
}
