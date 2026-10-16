use super::ScanDiagnosticCode;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    RegularFile,
    Directory,
    Symlink,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountFact {
    Same,
    Boundary,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudFact {
    Materialized,
    Dataless,
    SyncRoot,
    Unknown,
}

/// Adapter-provided metadata facts, never cleanup authority or path/name heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadFacts {
    pub kind: EntryKind,
    pub in_read_scope: bool,
    pub filesystem_supported: bool,
    pub mount: MountFact,
    pub cloud: CloudFact,
    pub root_too_broad: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadSkipReason {
    RootTooBroad,
    OutsideReadScope,
    SymlinkExcluded,
    SpecialExcluded,
    UnsupportedFilesystem,
    MountExcluded,
    CloudScopeUnsupported,
    CloudNotMaterialized,
}

impl From<ReadSkipReason> for ScanDiagnosticCode {
    fn from(reason: ReadSkipReason) -> Self {
        match reason {
            ReadSkipReason::RootTooBroad => Self::RootTooBroad,
            ReadSkipReason::OutsideReadScope => Self::OutsideReadScope,
            ReadSkipReason::SymlinkExcluded => Self::SymlinkExcluded,
            ReadSkipReason::SpecialExcluded => Self::SpecialExcluded,
            ReadSkipReason::UnsupportedFilesystem => Self::UnsupportedFilesystem,
            ReadSkipReason::MountExcluded => Self::MountExcluded,
            ReadSkipReason::CloudScopeUnsupported => Self::CloudScopeUnsupported,
            ReadSkipReason::CloudNotMaterialized => Self::CloudNotMaterialized,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDecision {
    MeasureMetadata,
    EnumerateDirectory,
    Skip(ReadSkipReason),
}

pub struct ScanReadPolicy;

impl ScanReadPolicy {
    pub fn evaluate(facts: &ReadFacts) -> ReadDecision {
        let reason = if facts.root_too_broad {
            Some(ReadSkipReason::RootTooBroad)
        } else if !facts.in_read_scope {
            Some(ReadSkipReason::OutsideReadScope)
        } else if facts.kind == EntryKind::Symlink {
            Some(ReadSkipReason::SymlinkExcluded)
        } else if facts.kind == EntryKind::Special {
            Some(ReadSkipReason::SpecialExcluded)
        } else if !facts.filesystem_supported || facts.mount == MountFact::Unknown {
            Some(ReadSkipReason::UnsupportedFilesystem)
        } else if facts.mount == MountFact::Boundary {
            Some(ReadSkipReason::MountExcluded)
        } else if matches!(facts.cloud, CloudFact::Unknown | CloudFact::SyncRoot) {
            Some(ReadSkipReason::CloudScopeUnsupported)
        } else if facts.cloud == CloudFact::Dataless && facts.kind == EntryKind::Directory {
            Some(ReadSkipReason::CloudNotMaterialized)
        } else {
            None
        };
        match reason {
            Some(reason) => ReadDecision::Skip(reason),
            None if facts.kind == EntryKind::Directory => ReadDecision::EnumerateDirectory,
            None => ReadDecision::MeasureMetadata,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    Safe,
    Review,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reclaimability {
    Unknown,
}

/// A sealed measurement policy. External claims cannot modify its protection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasurementProtection(());

pub const fn measurement_protection() -> MeasurementProtection {
    MeasurementProtection(())
}

impl MeasurementProtection {
    pub const fn safety_level(self) -> SafetyLevel {
        SafetyLevel::Protected
    }
    pub const fn policy_version(self) -> &'static str {
        "scanner-measurement-v1"
    }
    pub const fn allowed_actions(self) -> &'static [&'static str] {
        &[]
    }
    pub const fn reclaimability(self) -> Reclaimability {
        Reclaimability::Unknown
    }
}

impl Serialize for MeasurementProtection {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serializer.serialize_struct("MeasurementProtection", 4)?;
        value.serialize_field("safetyLevel", &self.safety_level())?;
        value.serialize_field("policyVersion", self.policy_version())?;
        value.serialize_field("allowedActions", self.allowed_actions())?;
        value.serialize_field("reclaimability", &self.reclaimability())?;
        value.end()
    }
}

impl<'de> Deserialize<'de> for MeasurementProtection {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Claim {
            safety_level: SafetyLevel,
            policy_version: String,
            allowed_actions: Vec<String>,
            reclaimability: Reclaimability,
        }
        let claim = Claim::deserialize(deserializer)?;
        if claim.safety_level != SafetyLevel::Protected
            || claim.policy_version != "scanner-measurement-v1"
            || !claim.allowed_actions.is_empty()
            || claim.reclaimability != Reclaimability::Unknown
        {
            return Err(serde::de::Error::custom(
                "Scan measurements must remain protected.",
            ));
        }
        Ok(measurement_protection())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{ScanDiagnosticCode, ScanMode};

    fn ordinary(kind: EntryKind) -> ReadFacts {
        ReadFacts {
            kind,
            in_read_scope: true,
            filesystem_supported: true,
            mount: MountFact::Same,
            cloud: CloudFact::Materialized,
            root_too_broad: false,
        }
    }

    // Catches name-based exclusions: names are deliberately absent from policy input.
    #[test]
    fn scan_read_policy_allows_metadata_without_name_heuristics() {
        assert_eq!(
            ScanReadPolicy::evaluate(&ordinary(EntryKind::RegularFile)),
            ReadDecision::MeasureMetadata
        );
        assert_eq!(
            ScanReadPolicy::evaluate(&ordinary(EntryKind::Directory)),
            ReadDecision::EnumerateDirectory
        );
        let mut dataless_file = ordinary(EntryKind::RegularFile);
        dataless_file.cloud = CloudFact::Dataless;
        assert_eq!(
            ScanReadPolicy::evaluate(&dataless_file),
            ReadDecision::MeasureMetadata
        );
    }

    // Catches a permissive fallback crossing links, mounts, unknown capability or cloud scope.
    #[test]
    fn scan_read_policy_skips_unsupported_scope_with_exact_reasons() {
        for (facts, reason) in [
            (
                ReadFacts {
                    root_too_broad: true,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::RootTooBroad,
            ),
            (
                ReadFacts {
                    in_read_scope: false,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::OutsideReadScope,
            ),
            (
                ordinary(EntryKind::Symlink),
                ReadSkipReason::SymlinkExcluded,
            ),
            (
                ordinary(EntryKind::Special),
                ReadSkipReason::SpecialExcluded,
            ),
            (
                ReadFacts {
                    filesystem_supported: false,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::UnsupportedFilesystem,
            ),
            (
                ReadFacts {
                    mount: MountFact::Boundary,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::MountExcluded,
            ),
            (
                ReadFacts {
                    mount: MountFact::Unknown,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::UnsupportedFilesystem,
            ),
            (
                ReadFacts {
                    cloud: CloudFact::Unknown,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::CloudScopeUnsupported,
            ),
            (
                ReadFacts {
                    cloud: CloudFact::SyncRoot,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::CloudScopeUnsupported,
            ),
            (
                ReadFacts {
                    cloud: CloudFact::Dataless,
                    ..ordinary(EntryKind::Directory)
                },
                ReadSkipReason::CloudNotMaterialized,
            ),
        ] {
            assert_eq!(ScanReadPolicy::evaluate(&facts), ReadDecision::Skip(reason));
        }
    }

    // Catches accidentally attributing a skipped branch to a different scope failure.
    #[test]
    fn scan_read_policy_skip_reasons_map_to_sanitized_diagnostics() {
        for (reason, code) in [
            (
                ReadSkipReason::RootTooBroad,
                ScanDiagnosticCode::RootTooBroad,
            ),
            (
                ReadSkipReason::OutsideReadScope,
                ScanDiagnosticCode::OutsideReadScope,
            ),
            (
                ReadSkipReason::SymlinkExcluded,
                ScanDiagnosticCode::SymlinkExcluded,
            ),
            (
                ReadSkipReason::SpecialExcluded,
                ScanDiagnosticCode::SpecialExcluded,
            ),
            (
                ReadSkipReason::UnsupportedFilesystem,
                ScanDiagnosticCode::UnsupportedFilesystem,
            ),
            (
                ReadSkipReason::MountExcluded,
                ScanDiagnosticCode::MountExcluded,
            ),
            (
                ReadSkipReason::CloudScopeUnsupported,
                ScanDiagnosticCode::CloudScopeUnsupported,
            ),
            (
                ReadSkipReason::CloudNotMaterialized,
                ScanDiagnosticCode::CloudNotMaterialized,
            ),
        ] {
            assert_eq!(ScanDiagnosticCode::from(reason), code);
        }
    }

    // Catches accidental coupling between measurement magnitude/configuration and cleanup safety.
    #[test]
    fn scan_read_policy_measurements_cannot_be_downgraded_by_claims() {
        for _mode in [ScanMode::Quick, ScanMode::Deep] {
            for _size in [0_u64, 9_007_199_254_740_993, u64::MAX] {
                for _enabled in [false, true] {
                    let protection = measurement_protection();
                    assert_eq!(protection.safety_level(), SafetyLevel::Protected);
                    assert_eq!(protection.policy_version(), "scanner-measurement-v1");
                    assert!(protection.allowed_actions().is_empty());
                    assert_eq!(protection.reclaimability(), Reclaimability::Unknown);
                    assert_eq!(
                        serde_json::to_value(protection).unwrap(),
                        serde_json::json!({
                            "safetyLevel": "protected", "policyVersion": "scanner-measurement-v1", "allowedActions": [], "reclaimability": "unknown"
                        })
                    );
                }
            }
        }
        for forged in [
            serde_json::json!({"safetyLevel": "safe", "policyVersion": "scanner-measurement-v1", "allowedActions": [], "reclaimability": "unknown"}),
            serde_json::json!({"safetyLevel": "review", "policyVersion": "scanner-measurement-v1", "allowedActions": [], "reclaimability": "unknown"}),
            serde_json::json!({"safetyLevel": "protected", "policyVersion": "scanner-measurement-v1", "allowedActions": ["delete"], "reclaimability": "unknown"}),
            serde_json::json!({"safetyLevel": "protected", "policyVersion": "other", "allowedActions": [], "reclaimability": "unknown"}),
        ] {
            assert!(serde_json::from_value::<MeasurementProtection>(forged).is_err());
        }
    }
}
