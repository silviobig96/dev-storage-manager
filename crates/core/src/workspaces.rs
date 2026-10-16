//! Workspace configuration contracts. These types do not inspect the filesystem.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceId(i64);

impl TryFrom<i64> for WorkspaceId {
    type Error = &'static str;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err("Workspace IDs must be positive integers.")
        }
    }
}

impl Serialize for WorkspaceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for WorkspaceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(serde::de::Error::custom(
                "Expected a positive decimal workspace ID.",
            ));
        }
        let parsed = value.parse::<i64>().map_err(serde::de::Error::custom)?;
        Self::try_from(parsed).map_err(serde::de::Error::custom)
    }
}

impl WorkspaceId {
    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryIdentity(String);

impl DirectoryIdentity {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedWorkspace {
    pub selected_path: String,
    pub canonical_path: String,
    pub identity: DirectoryIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWorkspace {
    pub id: WorkspaceId,
    pub root: ValidatedWorkspace,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: WorkspaceId,
    pub selected_path: String,
    pub display_name: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&StoredWorkspace> for WorkspaceRecord {
    fn from(value: &StoredWorkspace) -> Self {
        Self {
            id: value.id,
            selected_path: value.root.selected_path.clone(),
            display_name: Path::new(&value.root.selected_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Filesystem root")
                .to_owned(),
            enabled: value.enabled,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupState {
    Undecided,
    Skipped,
    Configured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceConfiguration {
    pub setup_state: SetupState,
    pub records: Vec<StoredWorkspace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListing {
    pub setup_state: SetupState,
    pub workspaces: Vec<WorkspaceRecord>,
    pub overlaps: Vec<WorkspaceOverlap>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    Pending,
    Present,
    Missing,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityReason {
    Timeout,
    IdentityChanged,
    NotDirectory,
    UnsupportedIdentity,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityObservation {
    pub state: AvailabilityState,
    pub reason: Option<AvailabilityReason>,
    pub checked_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceErrorCode {
    OperationInProgress,
    ScanInProgress,
    InvalidSelection,
    UnsupportedPathEncoding,
    NotDirectory,
    FolderMissing,
    AccessDenied,
    IdentityUnavailable,
    LocationChanged,
    PathAlreadyConfigured,
    ValidationTimeout,
    WorkspaceNotFound,
    PersistenceUnavailable,
    ApplicationUnavailable,
}

impl WorkspaceErrorCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::OperationInProgress => "Another workspace operation is in progress.",
            Self::ScanInProgress => "A scan is in progress.",
            Self::InvalidSelection => "Select a local folder.",
            Self::UnsupportedPathEncoding => "This folder's path cannot be represented as UTF-8.",
            Self::NotDirectory => "The selected location is not a folder.",
            Self::FolderMissing => "The folder could not be found.",
            Self::AccessDenied => "Access to this folder was denied.",
            Self::IdentityUnavailable => {
                "A reliable identity could not be obtained for this folder."
            }
            Self::LocationChanged => {
                "The folder changed during validation. Try selecting it again."
            }
            Self::PathAlreadyConfigured => {
                "This location has changed. Remove its old entry before adding it again."
            }
            Self::ValidationTimeout => "The folder check timed out. Try again.",
            Self::WorkspaceNotFound => "This workspace is no longer configured.",
            Self::PersistenceUnavailable => "Couldn't save workspace changes. Try again.",
            Self::ApplicationUnavailable => "Workspace services are unavailable. Try again.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFailure {
    pub code: WorkspaceErrorCode,
    pub existing_id: Option<WorkspaceId>,
}

impl Serialize for WorkspaceFailure {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut failure = serializer.serialize_struct(
            "WorkspaceFailure",
            if self.existing_id.is_some() { 3 } else { 2 },
        )?;
        failure.serialize_field("code", &self.code)?;
        failure.serialize_field("message", self.code.message())?;
        if let Some(id) = self.existing_id {
            failure.serialize_field("existingId", &id)?;
        }
        failure.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOverlap {
    pub parent_id: WorkspaceId,
    pub child_id: WorkspaceId,
}

/// Advisory component ancestry from saved paths, never a filesystem or scope check.
pub fn known_overlaps(records: &[StoredWorkspace]) -> Vec<WorkspaceOverlap> {
    let mut overlaps = Vec::new();
    for (index, first) in records.iter().enumerate() {
        for second in &records[index + 1..] {
            if first.root.identity == second.root.identity {
                continue;
            }
            let first_path = Path::new(&first.root.canonical_path);
            let second_path = Path::new(&second.root.canonical_path);
            if first_path == second_path {
                continue;
            }
            let pair = if second_path.starts_with(first_path) {
                Some((first.id, second.id))
            } else if first_path.starts_with(second_path) {
                Some((second.id, first.id))
            } else {
                None
            };
            if let Some((parent_id, child_id)) = pair {
                overlaps.push(WorkspaceOverlap {
                    parent_id,
                    child_id,
                });
            }
        }
    }
    overlaps
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceInsertOutcome {
    Added(WorkspaceRecord),
    Duplicate(WorkspaceId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};

    fn stored(id: i64, selected: &str, canonical: &str, identity: &str) -> StoredWorkspace {
        StoredWorkspace {
            id: WorkspaceId::try_from(id).unwrap(),
            root: ValidatedWorkspace {
                selected_path: selected.into(),
                canonical_path: canonical.into(),
                identity: DirectoryIdentity::new(identity),
            },
            enabled: true,
            created_at: 100,
            updated_at: 200,
        }
    }

    fn assert_wire<T>(value: &T, expected: Value)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
        assert_eq!(serde_json::from_value::<T>(expected).unwrap(), *value);
    }

    // Catches path/name transformations, private-field leakage, and DTO wire drift.
    #[test]
    fn workspace_contract() {
        let root = stored(
            7,
            "/fixture/Développement/Root",
            "/resolved/Root",
            "identity-a",
        );
        let record = WorkspaceRecord::from(&root);
        assert_eq!(record.display_name, "Root");
        let record_json = json!({
            "id": "7", "selectedPath": "/fixture/Développement/Root",
            "displayName": "Root", "enabled": true, "createdAt": 100, "updatedAt": 200
        });
        assert_wire(&record, record_json.clone());
        assert_eq!(
            WorkspaceRecord::from(&stored(8, "/", "/", "root")).display_name,
            "Filesystem root"
        );
        let same_name =
            WorkspaceRecord::from(&stored(9, "/other/Root", "/other/Root", "identity-b"));
        assert_eq!(same_name.display_name, record.display_name);
        assert_ne!(same_name.id, record.id);
        assert_ne!(same_name.selected_path, record.selected_path);
        for (path, basename) in [
            ("/fixture/café", "café"),
            ("/fixture/cafe\u{301}", "cafe\u{301}"),
            ("/fixture/ROOT", "ROOT"),
            ("/fixture/root", "root"),
            ("/fixture/folder with spaces/", "folder with spaces"),
        ] {
            let result = WorkspaceRecord::from(&stored(10, path, path, "spelling"));
            assert_eq!(result.selected_path, path);
            assert_eq!(result.display_name, basename);
        }
        for (state, wire) in [
            (SetupState::Undecided, "undecided"),
            (SetupState::Skipped, "skipped"),
            (SetupState::Configured, "configured"),
        ] {
            assert_wire(&state, json!(wire));
        }
        let relation = WorkspaceOverlap {
            parent_id: root.id,
            child_id: same_name.id,
        };
        assert_wire(&relation, json!({"parentId":"7", "childId":"9"}));
        assert_wire(
            &WorkspaceListing {
                setup_state: SetupState::Configured,
                workspaces: vec![record],
                overlaps: vec![relation],
            },
            json!({"setupState":"configured", "workspaces":[record_json], "overlaps":[{"parentId":"7", "childId":"9"}]}),
        );
        for (state, wire) in [
            (AvailabilityState::Pending, "pending"),
            (AvailabilityState::Present, "present"),
            (AvailabilityState::Missing, "missing"),
            (AvailabilityState::Denied, "denied"),
            (AvailabilityState::Unknown, "unknown"),
        ] {
            assert_wire(
                &AvailabilityObservation {
                    state,
                    reason: None,
                    checked_at: None,
                },
                json!({"state":wire, "reason":null, "checkedAt":null}),
            );
        }
        for (reason, wire) in [
            (AvailabilityReason::Timeout, "timeout"),
            (AvailabilityReason::IdentityChanged, "identity_changed"),
            (AvailabilityReason::NotDirectory, "not_directory"),
            (
                AvailabilityReason::UnsupportedIdentity,
                "unsupported_identity",
            ),
            (AvailabilityReason::Io, "io"),
        ] {
            assert_wire(
                &AvailabilityObservation {
                    state: AvailabilityState::Unknown,
                    reason: Some(reason),
                    checked_at: Some(123),
                },
                json!({"state":"unknown", "reason":wire, "checkedAt":123}),
            );
        }
        assert!(serde_json::from_value::<AvailabilityReason>(json!("unrecognized")).is_err());
        for (code, wire, message) in [
            (
                WorkspaceErrorCode::OperationInProgress,
                "operation_in_progress",
                "Another workspace operation is in progress.",
            ),
            (
                WorkspaceErrorCode::InvalidSelection,
                "invalid_selection",
                "Select a local folder.",
            ),
            (
                WorkspaceErrorCode::UnsupportedPathEncoding,
                "unsupported_path_encoding",
                "This folder's path cannot be represented as UTF-8.",
            ),
            (
                WorkspaceErrorCode::NotDirectory,
                "not_directory",
                "The selected location is not a folder.",
            ),
            (
                WorkspaceErrorCode::FolderMissing,
                "folder_missing",
                "The folder could not be found.",
            ),
            (
                WorkspaceErrorCode::AccessDenied,
                "access_denied",
                "Access to this folder was denied.",
            ),
            (
                WorkspaceErrorCode::IdentityUnavailable,
                "identity_unavailable",
                "A reliable identity could not be obtained for this folder.",
            ),
            (
                WorkspaceErrorCode::LocationChanged,
                "location_changed",
                "The folder changed during validation. Try selecting it again.",
            ),
            (
                WorkspaceErrorCode::PathAlreadyConfigured,
                "path_already_configured",
                "This location has changed. Remove its old entry before adding it again.",
            ),
            (
                WorkspaceErrorCode::ValidationTimeout,
                "validation_timeout",
                "The folder check timed out. Try again.",
            ),
            (
                WorkspaceErrorCode::WorkspaceNotFound,
                "workspace_not_found",
                "This workspace is no longer configured.",
            ),
            (
                WorkspaceErrorCode::PersistenceUnavailable,
                "persistence_unavailable",
                "Couldn't save workspace changes. Try again.",
            ),
            (
                WorkspaceErrorCode::ApplicationUnavailable,
                "application_unavailable",
                "Workspace services are unavailable. Try again.",
            ),
        ] {
            assert_wire(
                &WorkspaceFailure {
                    code,
                    existing_id: None,
                },
                json!({"code":wire, "message":message}),
            );
        }
        assert_wire(
            &WorkspaceFailure {
                code: WorkspaceErrorCode::PathAlreadyConfigured,
                existing_id: Some(root.id),
            },
            json!({
                "code":"path_already_configured", "message":"This location has changed. Remove its old entry before adding it again.", "existingId":"7"
            }),
        );
        assert!(serde_json::from_value::<WorkspaceFailure>(
            json!({"code":"arbitrary", "message":"raw error"})
        )
        .is_err());
    }

    // Catches numeric transport and accepting IDs that cannot be canonical DB keys.
    #[test]
    fn workspace_id_validation() {
        assert_wire(&WorkspaceId::try_from(7).unwrap(), json!("7"));
        for value in [9_007_199_254_740_992, i64::MAX] {
            assert_wire(
                &WorkspaceId::try_from(value).unwrap(),
                json!(value.to_string()),
            );
        }
        for invalid in [0, -1, i64::MIN] {
            assert!(WorkspaceId::try_from(invalid).is_err());
        }
        for invalid in [
            "0",
            "-1",
            "+1",
            "01",
            " 1",
            "1 ",
            "",
            "../folder",
            "1.0",
            "1e2",
            "９",
            "9223372036854775808",
        ] {
            assert!(
                serde_json::from_value::<WorkspaceId>(json!(invalid)).is_err(),
                "accepted {invalid:?}"
            );
        }
        for invalid in [json!(7), json!(0), json!(-1), json!(null), json!(true)] {
            assert!(serde_json::from_value::<WorkspaceId>(invalid).is_err());
        }
    }

    // Catches string-prefix ancestry and mistakenly reporting duplicates as overlap.
    #[test]
    fn workspace_overlap() {
        let parent = stored(1, "/selected/link", "/fixture/dev", "a");
        let child = stored(2, "/elsewhere/nested", "/fixture/dev/nested", "b");
        let sibling = stored(3, "/fixture/developer", "/fixture/developer", "c");
        let relation = WorkspaceOverlap {
            parent_id: parent.id,
            child_id: child.id,
        };
        assert_eq!(
            known_overlaps(&[parent.clone(), child.clone()]),
            vec![relation.clone()]
        );
        assert_eq!(
            known_overlaps(&[child.clone(), parent.clone()]),
            vec![relation]
        );
        assert!(known_overlaps(&[parent.clone(), sibling]).is_empty());
        assert!(known_overlaps(&[]).is_empty());
        assert!(known_overlaps(std::slice::from_ref(&parent)).is_empty());
        let duplicate = stored(4, "/alias", "/fixture/dev/nested/alias", "a");
        assert!(known_overlaps(&[parent.clone(), duplicate]).is_empty());
        let equal_path = stored(5, "/other", "/fixture/dev", "d");
        assert!(known_overlaps(&[parent, equal_path]).is_empty());
        let ancestor = stored(6, "/fixture", "/fixture", "e");
        let grandchild = stored(
            7,
            "/fixture/dev/nested/more",
            "/fixture/dev/nested/more",
            "f",
        );
        assert_eq!(
            known_overlaps(&[ancestor, child, grandchild]),
            vec![
                WorkspaceOverlap {
                    parent_id: WorkspaceId::try_from(6).unwrap(),
                    child_id: WorkspaceId::try_from(2).unwrap()
                },
                WorkspaceOverlap {
                    parent_id: WorkspaceId::try_from(6).unwrap(),
                    child_id: WorkspaceId::try_from(7).unwrap()
                },
                WorkspaceOverlap {
                    parent_id: WorkspaceId::try_from(2).unwrap(),
                    child_id: WorkspaceId::try_from(7).unwrap()
                },
            ]
        );
    }
}
