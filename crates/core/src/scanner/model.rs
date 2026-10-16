use super::MeasurementProtection;
use crate::workspaces::WorkspaceId;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn canonical_decimal(value: &str) -> bool {
    !value.is_empty()
        && (value.len() == 1 || !value.starts_with('0'))
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_epoch(value: &str) -> bool {
    let Some((pid, rest)) = value.strip_prefix('p').and_then(|v| v.split_once("-t")) else {
        return false;
    };
    let Some((time, generation)) = rest.split_once("-g") else {
        return false;
    };
    canonical_decimal(pid)
        && pid.parse::<u32>().is_ok()
        && canonical_decimal(time)
        && time.parse::<u128>().is_ok()
        && canonical_decimal(generation)
        && generation.parse::<u64>().is_ok()
}

fn valid_session_id(value: &str) -> bool {
    let Some((epoch, counter)) = value.strip_prefix("s:").and_then(|v| v.split_once(':')) else {
        return false;
    };
    valid_epoch(epoch) && canonical_decimal(counter) && counter.parse::<u64>().is_ok_and(|v| v > 0)
}

fn valid_request_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

macro_rules! opaque_token {
    ($name:ident, $validate:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = &'static str;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if $validate(&value) {
                    Ok(Self(value))
                } else {
                    Err(concat!("Invalid ", stringify!($name), "."))
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::try_from(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

opaque_token!(ScanEpoch, valid_epoch);
opaque_token!(ScanSessionId, valid_session_id);
opaque_token!(ScanRequestId, valid_request_id);

impl ScanEpoch {
    /// UTC startup nanoseconds come from the application clock, never a fallback.
    pub fn new(process_id: u32, startup_nanos: u128, generation: u64) -> Self {
        Self(format!("p{process_id}-t{startup_nanos}-g{generation}"))
    }
}

impl ScanSessionId {
    pub fn new(epoch: &ScanEpoch, counter: u64) -> Result<Self, &'static str> {
        if counter == 0 {
            return Err("Scan session counters must be positive.");
        }
        Ok(Self(format!("s:{}:{counter}", epoch.as_str())))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanRequest {
    pub request_id: ScanRequestId,
    pub expected_epoch: ScanEpoch,
    pub workspace_ids: Vec<WorkspaceId>,
    pub mode: ScanMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    CompleteWithinPolicy,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanMetrics {
    pub observed_logical_bytes: DecimalQuantity,
    pub observed_allocated_bytes: Option<DecimalQuantity>,
    pub unique_files: DecimalQuantity,
    pub unique_directories: DecimalQuantity,
    pub references_deduplicated: DecimalQuantity,
    pub allocation_measured_files: DecimalQuantity,
    pub allocation_unknown_files: DecimalQuantity,
    pub policy_excluded_entries: DecimalQuantity,
    pub unmeasured_entries_lower_bound: DecimalQuantity,
    pub unmeasured_bytes: Option<DecimalQuantity>,
    pub logical_coverage: Coverage,
    pub allocated_coverage: Coverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceScanState {
    Pending,
    Validating,
    Running,
    Completed,
    Partial,
    Unavailable,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScanResult {
    pub workspace_id: WorkspaceId,
    pub display_name: String,
    pub selected_path: String,
    pub state: WorkspaceScanState,
    pub metrics: Option<ScanMetrics>,
    pub coverage: Coverage,
    pub covered_by_workspace_ids: Vec<WorkspaceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSnapshot {
    pub session_id: ScanSessionId,
    pub request_id: ScanRequestId,
    pub epoch: ScanEpoch,
    pub mode: ScanMode,
    pub state: ScanState,
    pub revision: DecimalQuantity,
    /// UTC Unix milliseconds.
    pub started_at: DecimalQuantity,
    /// UTC Unix milliseconds; absent until the native worker has stopped.
    pub finished_at: Option<DecimalQuantity>,
    pub worker_active: bool,
    pub stop_reason: Option<StopReason>,
    pub failure: Option<ScanFailure>,
    pub workspaces: Vec<WorkspaceScanResult>,
    pub metrics: ScanMetrics,
    pub coverage: Coverage,
    pub protection: MeasurementProtection,
    pub diagnostics: Vec<ScanDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanDiagnosticCode {
    RootMissing,
    RootDenied,
    RootChanged,
    RootTooBroad,
    UnsupportedFilesystem,
    ChangedDuringScan,
    EntryMissing,
    EntryDenied,
    SymlinkExcluded,
    MountExcluded,
    SpecialExcluded,
    CloudNotMaterialized,
    CloudScopeUnsupported,
    OutsideReadScope,
    UnrepresentableName,
    AllocationUnknown,
    DepthLimit,
    EntryLimit,
    IdentityLimit,
    DescriptorLimit,
    TimeBudget,
    DiagnosticLimit,
    ResultLimit,
    QuantityOverflow,
    Cancelled,
}

impl ScanDiagnosticCode {
    pub const fn message(self) -> &'static str {
        match self {
            Self::RootMissing => "The workspace folder could not be found.",
            Self::RootDenied => "Access to the workspace folder was denied.",
            Self::RootChanged => "The workspace folder changed since it was selected.",
            Self::RootTooBroad => "Choose a narrower workspace folder to scan.",
            Self::UnsupportedFilesystem => {
                "This filesystem's metadata capabilities are unsupported."
            }
            Self::ChangedDuringScan => "An entry changed during the scan.",
            Self::EntryMissing => "An entry disappeared during the scan.",
            Self::EntryDenied => "Access to an entry was denied.",
            Self::SymlinkExcluded => "Symbolic links are excluded by scan policy.",
            Self::MountExcluded => "Mount crossings are excluded by scan policy.",
            Self::SpecialExcluded => "Special filesystem entries are excluded by scan policy.",
            Self::CloudNotMaterialized => "A cloud folder is not materialized locally.",
            Self::CloudScopeUnsupported => "Cloud metadata scope is unsupported.",
            Self::OutsideReadScope => "An entry is outside the authorized read scope.",
            Self::UnrepresentableName => "An entry name cannot be represented in the interface.",
            Self::AllocationUnknown => "Allocated space is unavailable for an observed file.",
            Self::DepthLimit => "The scan depth limit was reached.",
            Self::EntryLimit => "The scan entry limit was reached.",
            Self::IdentityLimit => "The scan identity limit was reached.",
            Self::DescriptorLimit => "The scan descriptor limit was reached.",
            Self::TimeBudget => "The scan time budget was reached.",
            Self::DiagnosticLimit => "The scan diagnostic limit was reached.",
            Self::ResultLimit => "The scan result size limit was reached.",
            Self::QuantityOverflow => "An observation exceeded the supported quantity range.",
            Self::Cancelled => "The scan was cancelled.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    TimeBudget,
    EntryLimit,
    IdentityLimit,
    DescriptorLimit,
    DiagnosticLimit,
    ResultLimit,
    QuantityOverflow,
    Cancelled,
}

impl From<StopReason> for ScanDiagnosticCode {
    fn from(reason: StopReason) -> Self {
        match reason {
            StopReason::TimeBudget => Self::TimeBudget,
            StopReason::EntryLimit => Self::EntryLimit,
            StopReason::IdentityLimit => Self::IdentityLimit,
            StopReason::DescriptorLimit => Self::DescriptorLimit,
            StopReason::DiagnosticLimit => Self::DiagnosticLimit,
            StopReason::ResultLimit => Self::ResultLimit,
            StopReason::QuantityOverflow => Self::QuantityOverflow,
            StopReason::Cancelled => Self::Cancelled,
        }
    }
}

/// Grouped root-level diagnostics cannot contain arbitrary text or child paths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDiagnostic {
    pub workspace_id: Option<WorkspaceId>,
    pub code: ScanDiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub count: DecimalQuantity,
}

impl Serialize for ScanDiagnostic {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serializer.serialize_struct("ScanDiagnostic", 5)?;
        value.serialize_field("workspaceId", &self.workspace_id)?;
        value.serialize_field("code", &self.code)?;
        value.serialize_field("severity", &self.severity)?;
        value.serialize_field("count", &self.count)?;
        value.serialize_field("message", self.code.message())?;
        value.end()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanState {
    Validating,
    Running,
    Stopping,
    Cancelling,
    Completed,
    Partial,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanErrorCode {
    InvalidRequest,
    ScopeEmpty,
    ScopeTooLarge,
    WorkspaceNotFound,
    WorkspaceDisabled,
    ScanBusy,
    WorkspaceOperationInProgress,
    StaleEpoch,
    RequestConflict,
    SessionNotFound,
    PersistenceUnavailable,
    ResourceUnavailable,
    ApplicationUnavailable,
}

impl ScanErrorCode {
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "The scan request is invalid.",
            Self::ScopeEmpty => "Select at least one enabled workspace to scan.",
            Self::ScopeTooLarge => "The requested scan scope exceeds supported limits.",
            Self::WorkspaceNotFound => "A requested workspace is no longer configured.",
            Self::WorkspaceDisabled => "A requested workspace is disabled.",
            Self::ScanBusy => "A scan is already in progress.",
            Self::WorkspaceOperationInProgress => "A workspace operation is in progress.",
            Self::StaleEpoch => "The scan state changed. Refresh and try again.",
            Self::RequestConflict => {
                "This request ID was already used for a different scan request."
            }
            Self::SessionNotFound => "This scan session is no longer available.",
            Self::PersistenceUnavailable => "Workspace configuration is unavailable. Try again.",
            Self::ResourceUnavailable => "Scan resources are unavailable. Try again.",
            Self::ApplicationUnavailable => "Scan services are unavailable. Try again.",
        }
    }
}

/// Error text is derived from a closed code, never an OS/SQL error string.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ScanFailure {
    pub code: ScanErrorCode,
}

impl Serialize for ScanFailure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serializer.serialize_struct("ScanFailure", 2)?;
        value.serialize_field("code", &self.code)?;
        value.serialize_field("message", self.code.message())?;
        value.end()
    }
}

/// Exact nonnegative IPC quantity; JSON numbers are deliberately not accepted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecimalQuantity(pub u64);

impl Serialize for DecimalQuantity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalQuantity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if !canonical_decimal(&value) {
            return Err(serde::de::Error::custom(
                "Expected a canonical nonnegative decimal quantity.",
            ));
        }
        value
            .parse::<u64>()
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    Quick,
    Deep,
}

/// Mode-derived budgets. No caller-supplied construction or mutation is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    max_requested_roots: usize,
    max_root_descriptor_bytes: usize,
    max_entry_attempts: u64,
    max_child_depth: usize,
    max_elapsed_millis: u64,
    max_traversal_threads: usize,
    max_queued_sessions: usize,
    max_descriptors: usize,
    max_identity_records: u64,
    identity_slots: usize,
    identity_slot_bytes: usize,
    max_diagnostic_groups: usize,
    max_child_path_samples: usize,
    max_snapshot_bytes: usize,
    query_interval_millis: u64,
}

impl ScanLimits {
    pub const fn for_mode(mode: ScanMode) -> Self {
        let (entries, depth, elapsed, slots) = match mode {
            ScanMode::Quick => (25_000, 8, 5_000, 65_536),
            ScanMode::Deep => (250_000, 32, 120_000, 524_288),
        };
        Self {
            max_requested_roots: 32,
            max_root_descriptor_bytes: 32_768,
            max_entry_attempts: entries,
            max_child_depth: depth,
            max_elapsed_millis: elapsed,
            max_traversal_threads: 1,
            max_queued_sessions: 0,
            max_descriptors: 36,
            max_identity_records: entries,
            identity_slots: slots,
            identity_slot_bytes: 64,
            max_diagnostic_groups: 128,
            max_child_path_samples: 0,
            max_snapshot_bytes: 131_072,
            query_interval_millis: 250,
        }
    }

    pub const fn max_requested_roots(self) -> usize {
        self.max_requested_roots
    }
    pub const fn max_root_descriptor_bytes(self) -> usize {
        self.max_root_descriptor_bytes
    }
    pub const fn max_entry_attempts(self) -> u64 {
        self.max_entry_attempts
    }
    pub const fn max_child_depth(self) -> usize {
        self.max_child_depth
    }
    pub const fn max_elapsed_millis(self) -> u64 {
        self.max_elapsed_millis
    }
    pub const fn max_traversal_threads(self) -> usize {
        self.max_traversal_threads
    }
    pub const fn max_queued_sessions(self) -> usize {
        self.max_queued_sessions
    }
    pub const fn max_descriptors(self) -> usize {
        self.max_descriptors
    }
    pub const fn max_identity_records(self) -> u64 {
        self.max_identity_records
    }
    pub const fn identity_slots(self) -> usize {
        self.identity_slots
    }
    pub const fn identity_slot_bytes(self) -> usize {
        self.identity_slot_bytes
    }
    pub const fn max_diagnostic_groups(self) -> usize {
        self.max_diagnostic_groups
    }
    pub const fn max_child_path_samples(self) -> usize {
        self.max_child_path_samples
    }
    pub const fn max_snapshot_bytes(self) -> usize {
        self.max_snapshot_bytes
    }
    pub const fn query_interval_millis(self) -> u64 {
        self.query_interval_millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Catches rounding, numeric IPC, noncanonical forms and overflow acceptance.
    #[test]
    fn scan_contract_quantities_are_lossless_canonical_strings() {
        for (quantity, json) in [
            (0, "\"0\""),
            (9_007_199_254_740_993, "\"9007199254740993\""),
            (u64::MAX, "\"18446744073709551615\""),
        ] {
            assert_eq!(
                serde_json::to_string(&DecimalQuantity(quantity)).unwrap(),
                json
            );
            assert_eq!(
                serde_json::from_str::<DecimalQuantity>(json).unwrap(),
                DecimalQuantity(quantity)
            );
        }
        for json in [
            "0",
            "9007199254740993",
            "-1",
            "\"-1\"",
            "\"01\"",
            "\"00\"",
            "\"+1\"",
            "\" 1\"",
            "\"1.0\"",
            "\"\"",
            "\"18446744073709551616\"",
            "null",
        ] {
            assert!(
                serde_json::from_str::<DecimalQuantity>(json).is_err(),
                "accepted {json}"
            );
        }
    }

    // Catches widened work/resource limits and accidental caller-controlled budgets.
    #[test]
    fn scan_contract_mode_limits_match_bounded_read_scope() {
        for (mode, entries, depth, elapsed, slots) in [
            (ScanMode::Quick, 25_000, 8, 5_000, 65_536),
            (ScanMode::Deep, 250_000, 32, 120_000, 524_288),
        ] {
            let limits = ScanLimits::for_mode(mode);
            assert_eq!(limits.max_requested_roots(), 32);
            assert_eq!(limits.max_root_descriptor_bytes(), 32_768);
            assert_eq!(limits.max_entry_attempts(), entries);
            assert_eq!(limits.max_child_depth(), depth);
            assert_eq!(limits.max_elapsed_millis(), elapsed);
            assert_eq!(limits.max_traversal_threads(), 1);
            assert_eq!(limits.max_queued_sessions(), 0);
            assert_eq!(limits.max_descriptors(), 36);
            assert_eq!(limits.max_identity_records(), entries);
            assert_eq!(limits.identity_slots(), slots);
            assert_eq!(limits.identity_slot_bytes(), 64);
            assert_eq!(limits.max_diagnostic_groups(), 128);
            assert_eq!(limits.max_child_path_samples(), 0);
            assert_eq!(limits.max_snapshot_bytes(), 131_072);
            assert_eq!(limits.query_interval_millis(), 250);
        }
    }

    // Catches wire casing changes and loss of stable typed terminal/failure reasons.
    #[test]
    fn scan_contract_modes_states_and_failures_round_trip() {
        for (mode, json) in [(ScanMode::Quick, "\"quick\""), (ScanMode::Deep, "\"deep\"")] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), json);
            assert_eq!(serde_json::from_str::<ScanMode>(json).unwrap(), mode);
        }
        for (state, json) in [
            (ScanState::Validating, "\"validating\""),
            (ScanState::Running, "\"running\""),
            (ScanState::Stopping, "\"stopping\""),
            (ScanState::Cancelling, "\"cancelling\""),
            (ScanState::Completed, "\"completed\""),
            (ScanState::Partial, "\"partial\""),
            (ScanState::Cancelled, "\"cancelled\""),
            (ScanState::Error, "\"error\""),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), json);
            assert_eq!(serde_json::from_str::<ScanState>(json).unwrap(), state);
        }
        let failure = ScanFailure {
            code: ScanErrorCode::ResourceUnavailable,
        };
        assert_eq!(
            serde_json::to_value(&failure).unwrap(),
            serde_json::json!({
                "code": "resource_unavailable", "message": "Scan resources are unavailable. Try again."
            })
        );
        assert_eq!(
            serde_json::from_value::<ScanFailure>(serde_json::json!({
                "code": "resource_unavailable", "message": "untrusted raw OS error"
            }))
            .unwrap(),
            failure
        );
        assert_eq!(
            serde_json::to_value(ScanFailure {
                code: ScanErrorCode::ScopeEmpty
            })
            .unwrap(),
            serde_json::json!({
                "code": "scope_empty", "message": "Select at least one enabled workspace to scan."
            })
        );
        for (code, json) in [
            (ScanErrorCode::InvalidRequest, "\"invalid_request\""),
            (ScanErrorCode::ScopeEmpty, "\"scope_empty\""),
            (ScanErrorCode::ScopeTooLarge, "\"scope_too_large\""),
            (ScanErrorCode::WorkspaceNotFound, "\"workspace_not_found\""),
            (ScanErrorCode::WorkspaceDisabled, "\"workspace_disabled\""),
            (ScanErrorCode::ScanBusy, "\"scan_busy\""),
            (
                ScanErrorCode::WorkspaceOperationInProgress,
                "\"workspace_operation_in_progress\"",
            ),
            (ScanErrorCode::StaleEpoch, "\"stale_epoch\""),
            (ScanErrorCode::RequestConflict, "\"request_conflict\""),
            (ScanErrorCode::SessionNotFound, "\"session_not_found\""),
            (
                ScanErrorCode::PersistenceUnavailable,
                "\"persistence_unavailable\"",
            ),
            (
                ScanErrorCode::ResourceUnavailable,
                "\"resource_unavailable\"",
            ),
            (
                ScanErrorCode::ApplicationUnavailable,
                "\"application_unavailable\"",
            ),
        ] {
            assert_eq!(serde_json::to_string(&code).unwrap(), json);
            assert_eq!(serde_json::from_str::<ScanErrorCode>(json).unwrap(), code);
        }
    }

    // Catches malformed/reused token representations and numeric token coercion.
    #[test]
    fn scan_contract_opaque_tokens_validate_canonical_components() {
        let epoch = ScanEpoch::new(42, 1_234_567_890_123_456_789, 9);
        assert_eq!(
            serde_json::to_string(&epoch).unwrap(),
            "\"p42-t1234567890123456789-g9\""
        );
        let session = ScanSessionId::new(&epoch, 1).unwrap();
        assert_eq!(
            serde_json::to_string(&session).unwrap(),
            "\"s:p42-t1234567890123456789-g9:1\""
        );
        assert!(ScanSessionId::new(&epoch, 0).is_err());
        for invalid in [
            "\"p042-t1-g1\"",
            "\"p42-t01-g1\"",
            "\"p42-t1-g01\"",
            "\"p4294967296-t1-g1\"",
            "\"p1-t340282366920938463463374607431768211456-g1\"",
            "\"p1-t1-g18446744073709551616\"",
            "42",
        ] {
            assert!(
                serde_json::from_str::<ScanEpoch>(invalid).is_err(),
                "accepted {invalid}"
            );
        }
        for invalid in [
            "\"s:p42-t1-g1:0\"",
            "\"s:p42-t1-g1:01\"",
            "\"s:p42-t1-g1:18446744073709551616\"",
            "\"s:p42-t1-g1:1:2\"",
        ] {
            assert!(
                serde_json::from_str::<ScanSessionId>(invalid).is_err(),
                "accepted {invalid}"
            );
        }
        let request = "01234567-89ab-cdef-0123-456789abcdef";
        assert_eq!(
            ScanRequestId::try_from(request.to_owned())
                .unwrap()
                .as_str(),
            request
        );
        for invalid in [
            "01234567-89AB-cdef-0123-456789abcdef",
            "0123456789abcdef0123456789abcdef",
            "01234567-89ab-cdef-0123-456789abcdeg",
            "",
            "/private/root",
        ] {
            assert!(
                ScanRequestId::try_from(invalid.to_owned()).is_err(),
                "accepted {invalid}"
            );
        }
        assert_eq!(
            serde_json::from_str::<ScanEpoch>("\"p0-t0-g0\"")
                .unwrap()
                .as_str(),
            "p0-t0-g0"
        );
        assert_eq!(
            serde_json::from_str::<ScanSessionId>(&serde_json::to_string(&session).unwrap())
                .unwrap(),
            session
        );
    }

    // Catches null-to-zero coercion, wrong field casing and leakage of internal traversal data.
    #[test]
    fn scan_contract_snapshot_distinguishes_unavailable_observations_and_zero() {
        let json = serde_json::json!({
            "sessionId": "s:p42-t1234567890123456789-g1:1",
            "requestId": "01234567-89ab-cdef-0123-456789abcdef",
            "epoch": "p42-t1234567890123456789-g1",
            "mode": "quick", "state": "validating", "revision": "9007199254740993",
            "startedAt": "1792108800000", "finishedAt": null, "workerActive": true,
            "stopReason": null, "failure": null,
            "workspaces": [{
                "workspaceId": "1", "displayName": "fixture", "selectedPath": "/fixture",
                "state": "pending", "metrics": null, "coverage": "unavailable", "coveredByWorkspaceIds": []
            }],
            "metrics": {
                "observedLogicalBytes": "0", "observedAllocatedBytes": null,
                "uniqueFiles": "0", "uniqueDirectories": "0", "referencesDeduplicated": "0",
                "allocationMeasuredFiles": "0", "allocationUnknownFiles": "0", "policyExcludedEntries": "0",
                "unmeasuredEntriesLowerBound": "0", "unmeasuredBytes": null,
                "logicalCoverage": "unavailable", "allocatedCoverage": "unavailable"
            },
            "coverage": "unavailable",
            "protection": {"safetyLevel": "protected", "policyVersion": "scanner-measurement-v1", "allowedActions": [], "reclaimability": "unknown"},
            "diagnostics": [{"workspaceId": "1", "code": "root_too_broad", "severity": "warning", "count": "1", "message": "Choose a narrower workspace folder to scan."}]
        });
        let mut snapshot: ScanSnapshot = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), json);
        snapshot.metrics.observed_allocated_bytes = Some(DecimalQuantity(0));
        snapshot.metrics.unmeasured_bytes = Some(DecimalQuantity(0));
        snapshot.metrics.logical_coverage = Coverage::CompleteWithinPolicy;
        snapshot.metrics.allocated_coverage = Coverage::CompleteWithinPolicy;
        snapshot.finished_at = Some(DecimalQuantity(u64::MAX));
        let measured = serde_json::to_value(snapshot).unwrap();
        assert_eq!(measured["metrics"]["observedAllocatedBytes"], "0");
        assert_eq!(measured["metrics"]["unmeasuredBytes"], "0");
        assert_eq!(measured["finishedAt"], "18446744073709551615");
        assert_eq!(
            measured["metrics"]["logicalCoverage"],
            "complete_within_policy"
        );
        assert_eq!(
            measured["workspaces"][0]["metrics"],
            serde_json::Value::Null
        );
        // No identity or child path fields exist in this closed serialized DTO.
        let request: ScanRequest = serde_json::from_value(serde_json::json!({
            "requestId": "01234567-89ab-cdef-0123-456789abcdef", "expectedEpoch": "p42-t1-g0",
            "workspaceIds": ["1", "2"], "mode": "deep"
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap()["workspaceIds"],
            serde_json::json!(["1", "2"])
        );
        assert!(serde_json::from_value::<ScanRequest>(serde_json::json!({
            "requestId": "01234567-89ab-cdef-0123-456789abcdef", "expectedEpoch": "p42-t1-g0",
            "workspaceIds": ["1"], "mode": "deep", "path": "/unrequested"
        }))
        .is_err());
    }

    // Catches collapsing per-metric coverage and allowing branch skips as session-stop causes.
    #[test]
    fn scan_contract_partial_allocation_and_stop_reasons_are_distinct() {
        let metrics: ScanMetrics = serde_json::from_value(serde_json::json!({
            "observedLogicalBytes": "9007199254740993", "observedAllocatedBytes": "512",
            "uniqueFiles": "2", "uniqueDirectories": "1", "referencesDeduplicated": "0",
            "allocationMeasuredFiles": "1", "allocationUnknownFiles": "1", "policyExcludedEntries": "0",
            "unmeasuredEntriesLowerBound": "0", "unmeasuredBytes": null,
            "logicalCoverage": "complete_within_policy", "allocatedCoverage": "partial"
        })).unwrap();
        assert_eq!(metrics.logical_coverage, Coverage::CompleteWithinPolicy);
        assert_eq!(metrics.allocated_coverage, Coverage::Partial);
        assert_eq!(metrics.observed_allocated_bytes, Some(DecimalQuantity(512)));
        for (reason, json, code) in [
            (
                StopReason::TimeBudget,
                "\"time_budget\"",
                ScanDiagnosticCode::TimeBudget,
            ),
            (
                StopReason::EntryLimit,
                "\"entry_limit\"",
                ScanDiagnosticCode::EntryLimit,
            ),
            (
                StopReason::IdentityLimit,
                "\"identity_limit\"",
                ScanDiagnosticCode::IdentityLimit,
            ),
            (
                StopReason::DescriptorLimit,
                "\"descriptor_limit\"",
                ScanDiagnosticCode::DescriptorLimit,
            ),
            (
                StopReason::DiagnosticLimit,
                "\"diagnostic_limit\"",
                ScanDiagnosticCode::DiagnosticLimit,
            ),
            (
                StopReason::ResultLimit,
                "\"result_limit\"",
                ScanDiagnosticCode::ResultLimit,
            ),
            (
                StopReason::QuantityOverflow,
                "\"quantity_overflow\"",
                ScanDiagnosticCode::QuantityOverflow,
            ),
            (
                StopReason::Cancelled,
                "\"cancelled\"",
                ScanDiagnosticCode::Cancelled,
            ),
        ] {
            assert_eq!(serde_json::to_string(&reason).unwrap(), json);
            assert_eq!(serde_json::from_str::<StopReason>(json).unwrap(), reason);
            assert_eq!(ScanDiagnosticCode::from(reason), code);
        }
        for json in [
            "\"depth_limit\"",
            "\"allocation_unknown\"",
            "\"root_missing\"",
        ] {
            assert!(serde_json::from_str::<StopReason>(json).is_err());
        }
    }
}
