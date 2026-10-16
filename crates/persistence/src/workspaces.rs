use dev_storage_core::workspaces::{
    DirectoryIdentity, SetupState, StoredWorkspace, ValidatedWorkspace, WorkspaceConfiguration,
    WorkspaceErrorCode, WorkspaceFailure, WorkspaceId, WorkspaceInsertOutcome, WorkspaceRecord,
};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

use crate::Database;

const SETUP_KEY: &str = "workspace_setup_state_v1";
const WORKSPACE_COLUMNS: &str =
    "id, selected_path, canonical_path, identity_key, enabled, created_at, updated_at";

fn unavailable() -> WorkspaceFailure {
    WorkspaceFailure {
        code: WorkspaceErrorCode::PersistenceUnavailable,
        existing_id: None,
    }
}

fn not_found() -> WorkspaceFailure {
    WorkspaceFailure {
        code: WorkspaceErrorCode::WorkspaceNotFound,
        existing_id: None,
    }
}

fn setup_state(connection: &Connection) -> Result<SetupState, WorkspaceFailure> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM application_metadata WHERE key = ?1",
            [SETUP_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| unavailable())?;
    match value.as_deref() {
        None => Ok(SetupState::Undecided),
        Some("skipped") => Ok(SetupState::Skipped),
        Some("configured") => Ok(SetupState::Configured),
        Some(_) => Err(unavailable()),
    }
}

fn read_workspace(row: &Row<'_>) -> rusqlite::Result<StoredWorkspace> {
    let raw_id: i64 = row.get(0)?;
    let id = WorkspaceId::try_from(raw_id)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, raw_id))?;
    Ok(StoredWorkspace {
        id,
        root: ValidatedWorkspace {
            selected_path: row.get(1)?,
            canonical_path: row.get(2)?,
            identity: DirectoryIdentity::new(row.get::<_, String>(3)?),
        },
        enabled: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn workspace_by_id(
    connection: &Connection,
    id: WorkspaceId,
) -> Result<StoredWorkspace, WorkspaceFailure> {
    connection
        .query_row(
            &format!("SELECT {WORKSPACE_COLUMNS} FROM workspaces WHERE id = ?1"),
            [id.get()],
            read_workspace,
        )
        .optional()
        .map_err(|_| unavailable())?
        .ok_or_else(not_found)
}

impl Database {
    pub fn workspace_configuration(&self) -> Result<WorkspaceConfiguration, WorkspaceFailure> {
        // One read snapshot keeps the setup decision and rows consistent with concurrent commits.
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| unavailable())?;
        let setup_state = setup_state(&transaction)?;
        let records = {
            let mut statement = transaction
                .prepare(&format!(
                    "SELECT {WORKSPACE_COLUMNS} FROM workspaces ORDER BY created_at, id"
                ))
                .map_err(|_| unavailable())?;
            let records = statement
                .query_map([], read_workspace)
                .map_err(|_| unavailable())?;
            records
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| unavailable())?
        };
        transaction.commit().map_err(|_| unavailable())?;
        Ok(WorkspaceConfiguration {
            setup_state,
            records,
        })
    }

    pub fn insert_workspace(
        &mut self,
        root: &ValidatedWorkspace,
        now: i64,
    ) -> Result<WorkspaceInsertOutcome, WorkspaceFailure> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let setup = setup_state(&transaction)?;
        let duplicate: Option<i64> = transaction
            .query_row(
                "SELECT id FROM workspaces WHERE identity_key = ?1",
                [root.identity.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| unavailable())?;
        if let Some(raw_id) = duplicate {
            let id = WorkspaceId::try_from(raw_id).map_err(|_| unavailable())?;
            transaction.commit().map_err(|_| unavailable())?;
            return Ok(WorkspaceInsertOutcome::Duplicate(id));
        }
        let path_conflict: Option<i64> = transaction.query_row(
            "SELECT id FROM workspaces WHERE selected_path = ?1 OR canonical_path = ?2 ORDER BY id LIMIT 1",
            params![root.selected_path, root.canonical_path],
            |row| row.get(0),
        ).optional().map_err(|_| unavailable())?;
        if let Some(raw_id) = path_conflict {
            return Err(WorkspaceFailure {
                code: WorkspaceErrorCode::PathAlreadyConfigured,
                existing_id: Some(WorkspaceId::try_from(raw_id).map_err(|_| unavailable())?),
            });
        }
        transaction.execute(
            "INSERT INTO workspaces (selected_path, canonical_path, identity_key, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![root.selected_path, root.canonical_path, root.identity.as_str(), now],
        ).map_err(|_| unavailable())?;
        let id =
            WorkspaceId::try_from(transaction.last_insert_rowid()).map_err(|_| unavailable())?;
        if setup != SetupState::Configured {
            transaction.execute(
                "INSERT INTO application_metadata (key, value) VALUES (?1, 'configured') ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [SETUP_KEY],
            ).map_err(|_| unavailable())?;
        }
        let record = WorkspaceRecord::from(&workspace_by_id(&transaction, id)?);
        transaction.commit().map_err(|_| unavailable())?;
        Ok(WorkspaceInsertOutcome::Added(record))
    }

    pub fn remove_workspace(&mut self, id: WorkspaceId) -> Result<(), WorkspaceFailure> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        setup_state(&transaction)?;
        let removed = transaction
            .execute("DELETE FROM workspaces WHERE id = ?1", [id.get()])
            .map_err(|_| unavailable())?;
        if removed == 0 {
            return Err(not_found());
        }
        transaction.commit().map_err(|_| unavailable())?;
        Ok(())
    }

    pub fn set_workspace_enabled(
        &mut self,
        id: WorkspaceId,
        enabled: bool,
        now: i64,
    ) -> Result<WorkspaceRecord, WorkspaceFailure> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        setup_state(&transaction)?;
        let mut stored = workspace_by_id(&transaction, id)?;
        if stored.enabled != enabled {
            transaction
                .execute(
                    "UPDATE workspaces SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                    params![enabled, now, id.get()],
                )
                .map_err(|_| unavailable())?;
            stored = workspace_by_id(&transaction, id)?;
        }
        let record = WorkspaceRecord::from(&stored);
        transaction.commit().map_err(|_| unavailable())?;
        Ok(record)
    }

    pub fn skip_workspace_setup(&mut self) -> Result<SetupState, WorkspaceFailure> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| unavailable())?;
        let state = setup_state(&transaction)?;
        let state = if state == SetupState::Undecided {
            transaction
                .execute(
                    "INSERT INTO application_metadata (key, value) VALUES (?1, 'skipped')",
                    [SETUP_KEY],
                )
                .map_err(|_| unavailable())?;
            SetupState::Skipped
        } else {
            state
        };
        transaction.commit().map_err(|_| unavailable())?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use dev_storage_core::workspaces::DirectoryIdentity;

    use super::*;

    fn root(selected: &str, canonical: &str, identity: &str) -> ValidatedWorkspace {
        ValidatedWorkspace {
            selected_path: selected.to_owned(),
            canonical_path: canonical.to_owned(),
            identity: DirectoryIdentity::new(identity),
        }
    }

    fn added(outcome: WorkspaceInsertOutcome) -> WorkspaceRecord {
        match outcome {
            WorkspaceInsertOutcome::Added(record) => record,
            WorkspaceInsertOutcome::Duplicate(id) => panic!("unexpected duplicate {id:?}"),
        }
    }

    fn assert_unavailable<T: std::fmt::Debug>(result: Result<T, WorkspaceFailure>) {
        let error = result.unwrap_err();
        assert_eq!(error, unavailable());
        assert!(!format!("{error:?}").contains("sentinel"));
    }

    // Catches lossy path storage, missing timestamps/order, and non-persistent/idempotent changes.
    #[test]
    fn workspace_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let first_root = root(
            "/fixture/cafe\u{301}/O'Brien",
            "/resolved/first",
            "identity-a",
        );
        let second_root = root("/fixture/Café", "/resolved/second", "identity-b");
        let (first, second) = {
            let mut database = Database::open(&path).unwrap();
            assert_eq!(
                database.workspace_configuration().unwrap().setup_state,
                SetupState::Undecided
            );
            let first = added(database.insert_workspace(&first_root, 200).unwrap());
            let second = added(database.insert_workspace(&second_root, 100).unwrap());
            assert_ne!(first.id, second.id);
            assert_eq!(
                (first.created_at, first.updated_at, first.enabled),
                (200, 200, true)
            );
            assert_eq!(
                (second.created_at, second.updated_at, second.enabled),
                (100, 100, true)
            );
            (first, second)
        };
        let mut database = Database::open(&path).unwrap();
        let configuration = database.workspace_configuration().unwrap();
        assert_eq!(configuration.setup_state, SetupState::Configured);
        assert_eq!(configuration.records.len(), 2);
        assert_eq!(configuration.records[0].id, second.id);
        assert_eq!(configuration.records[0].root, second_root);
        assert_eq!(configuration.records[1].id, first.id);
        assert_eq!(configuration.records[1].root, first_root);
        assert_eq!(configuration.records[1].created_at, 200);
        assert_eq!(
            database.set_workspace_enabled(first.id, true, 300).unwrap(),
            first
        );
        let disabled = database
            .set_workspace_enabled(first.id, false, 400)
            .unwrap();
        assert!(!disabled.enabled);
        assert_eq!((disabled.created_at, disabled.updated_at), (200, 400));
        assert_eq!(
            database
                .set_workspace_enabled(first.id, false, 500)
                .unwrap(),
            disabled
        );
        assert_eq!(
            database.skip_workspace_setup().unwrap(),
            SetupState::Configured
        );
        drop(database);
        let database = Database::open(&path).unwrap();
        let configuration = database.workspace_configuration().unwrap();
        assert_eq!(WorkspaceRecord::from(&configuration.records[1]), disabled);
        assert_eq!(configuration.setup_state, SetupState::Configured);
    }

    // Catches skip being transient or accidentally downgrading a configured installation.
    #[test]
    fn workspace_skip_persists_and_add_configures() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        {
            let mut database = Database::open(&path).unwrap();
            assert_eq!(
                database.skip_workspace_setup().unwrap(),
                SetupState::Skipped
            );
            assert_eq!(
                database.skip_workspace_setup().unwrap(),
                SetupState::Skipped
            );
        }
        let mut database = Database::open(&path).unwrap();
        assert_eq!(
            database.workspace_configuration().unwrap().setup_state,
            SetupState::Skipped
        );
        database
            .insert_workspace(&root("/selected", "/canonical", "identity"), 100)
            .unwrap();
        assert_eq!(
            database.skip_workspace_setup().unwrap(),
            SetupState::Configured
        );
    }

    // Catches UI-only uniqueness, non-atomic duplicate handling, or accidental upserts.
    #[test]
    fn workspace_unique_across_connections() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let first = Database::open(&path).unwrap();
        let second = Database::open(&path).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = [first, second]
            .into_iter()
            .enumerate()
            .map(|(index, mut database)| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let root = root(
                        &format!("/selected/{index}"),
                        &format!("/canonical/{index}"),
                        "shared",
                    );
                    barrier.wait();
                    let result = database.insert_workspace(&root, 100);
                    (database, root, result)
                })
            })
            .collect();
        let mut results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let successful: Vec<_> = results
            .iter()
            .filter_map(|(_, _, result)| match result {
                Ok(WorkspaceInsertOutcome::Added(record)) => Some(record.id),
                _ => None,
            })
            .collect();
        assert_eq!(successful.len(), 1);
        let id = successful[0];
        for (database, root, result) in &mut results {
            if matches!(result, Err(error) if error.code == WorkspaceErrorCode::PersistenceUnavailable)
            {
                *result = database.insert_workspace(root, 100);
            }
            match result.as_ref().unwrap() {
                WorkspaceInsertOutcome::Added(record) => assert_eq!(record.id, id),
                WorkspaceInsertOutcome::Duplicate(existing) => assert_eq!(*existing, id),
            }
        }
        assert_eq!(
            results
                .iter()
                .filter(|(_, _, result)| matches!(result, Ok(WorkspaceInsertOutcome::Duplicate(_))))
                .count(),
            1
        );
        let configuration = results[0].0.workspace_configuration().unwrap();
        assert_eq!(configuration.records.len(), 1);
        assert_eq!(configuration.setup_state, SetupState::Configured);
    }

    // Catches rebinding an existing path, conflating identity with spelling, or case folding.
    #[test]
    fn workspace_path_conflicts_preserve_existing_record() {
        let mut database = Database::open_in_memory().unwrap();
        let original = root("/selected/Root", "/canonical/Root", "original");
        let record = added(database.insert_workspace(&original, 100).unwrap());
        database
            .set_workspace_enabled(record.id, false, 200)
            .unwrap();
        for conflicting in [
            root("/selected/Root", "/canonical/new", "different-a"),
            root("/selected/new", "/canonical/Root", "different-b"),
        ] {
            assert_eq!(
                database.insert_workspace(&conflicting, 300),
                Err(WorkspaceFailure {
                    code: WorkspaceErrorCode::PathAlreadyConfigured,
                    existing_id: Some(record.id)
                })
            );
        }
        let alias = root("/alias", "/alias", "original");
        assert_eq!(
            database.insert_workspace(&alias, 300).unwrap(),
            WorkspaceInsertOutcome::Duplicate(record.id)
        );
        let configuration = database.workspace_configuration().unwrap();
        assert_eq!(configuration.records.len(), 1);
        assert_eq!(configuration.records[0].root, original);
        assert!(!configuration.records[0].enabled);
        assert_eq!(configuration.records[0].updated_at, 200);
        database
            .insert_workspace(
                &root("/selected/root", "/canonical/root", "case-distinct"),
                100,
            )
            .unwrap();
        let configuration = database.workspace_configuration().unwrap();
        assert_eq!(configuration.records.len(), 2);
        assert_eq!(configuration.records[0].id, record.id);
    }

    // Catches a workspace commit escaping an aborted setup write or raw SQLite errors leaking.
    #[test]
    fn workspace_failure_rolls_back_setup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let mut database = Database::open(&path).unwrap();
        database.set_metadata("sentinel", "unchanged").unwrap();
        database.connection.execute_batch("CREATE TRIGGER reject_setup BEFORE INSERT ON application_metadata WHEN NEW.key = 'workspace_setup_state_v1' BEGIN SELECT RAISE(ABORT, 'sentinel constraint details'); END;").unwrap();
        assert_unavailable(database.insert_workspace(
            &root("/sentinel/path", "/resolved", "sentinel-identity"),
            100,
        ));
        assert_unavailable(database.skip_workspace_setup());
        let configuration = database.workspace_configuration().unwrap();
        assert!(configuration.records.is_empty());
        assert_eq!(configuration.setup_state, SetupState::Undecided);
        assert_eq!(
            database.metadata("sentinel").unwrap().as_deref(),
            Some("unchanged")
        );
        drop(database);
        let database = Database::open(&path).unwrap();
        assert!(database
            .workspace_configuration()
            .unwrap()
            .records
            .is_empty());
        assert_eq!(
            database.workspace_configuration().unwrap().setup_state,
            SetupState::Undecided
        );
    }

    // Catches silently overwriting corrupt setup metadata or returning a plausible empty list.
    #[test]
    fn workspace_invalid_setup_is_an_error() {
        let mut database = Database::open_in_memory().unwrap();
        let original = added(
            database
                .insert_workspace(&root("/existing", "/existing", "existing"), 100)
                .unwrap(),
        );
        database
            .set_metadata("workspace_setup_state_v1", "sentinel-invalid")
            .unwrap();
        assert_unavailable(database.workspace_configuration());
        assert_unavailable(database.skip_workspace_setup());
        assert_unavailable(database.set_workspace_enabled(original.id, false, 200));
        assert_unavailable(database.remove_workspace(original.id));
        assert_unavailable(
            database.insert_workspace(&root("/selected", "/canonical", "identity"), 100),
        );
        assert_eq!(
            database
                .metadata("workspace_setup_state_v1")
                .unwrap()
                .as_deref(),
            Some("sentinel-invalid")
        );
        assert_eq!(
            database
                .connection
                .query_row("SELECT COUNT(*) FROM workspaces", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            WorkspaceRecord::from(&workspace_by_id(&database.connection, original.id).unwrap()),
            original
        );
    }

    // Catches treating absent IDs as successful mutations or exposing failed query details.
    #[test]
    fn workspace_unknown_ids_and_query_failures_are_sanitized() {
        let mut database = Database::open_in_memory().unwrap();
        let id = WorkspaceId::try_from(99).unwrap();
        let expected = WorkspaceFailure {
            code: WorkspaceErrorCode::WorkspaceNotFound,
            existing_id: None,
        };
        assert_eq!(database.remove_workspace(id), Err(expected.clone()));
        assert_eq!(
            database.set_workspace_enabled(id, false, 100),
            Err(expected)
        );
        assert_eq!(
            database.workspace_configuration().unwrap().setup_state,
            SetupState::Undecided
        );
        database
            .connection
            .execute_batch("ALTER TABLE workspaces RENAME TO sentinel_private;")
            .unwrap();
        assert_unavailable(database.workspace_configuration());
        assert_unavailable(database.remove_workspace(id));
        assert_unavailable(database.set_workspace_enabled(id, false, 100));
        assert_unavailable(
            database.insert_workspace(&root("/sentinel", "/canonical", "identity"), 100),
        );
    }

    // Catches final removal resetting onboarding, deleting unrelated metadata, or reusing IDs.
    #[test]
    fn workspace_remove_last_keeps_setup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let first_id = {
            let mut database = Database::open(&path).unwrap();
            database.set_metadata("sentinel", "unchanged").unwrap();
            let id = added(
                database
                    .insert_workspace(&root("/selected", "/canonical", "identity"), 100)
                    .unwrap(),
            )
            .id;
            database.remove_workspace(id).unwrap();
            assert!(database
                .workspace_configuration()
                .unwrap()
                .records
                .is_empty());
            assert_eq!(
                database.workspace_configuration().unwrap().setup_state,
                SetupState::Configured
            );
            id
        };
        let mut database = Database::open(&path).unwrap();
        let configuration = database.workspace_configuration().unwrap();
        assert!(configuration.records.is_empty());
        assert_eq!(configuration.setup_state, SetupState::Configured);
        assert_eq!(
            database.metadata("sentinel").unwrap().as_deref(),
            Some("unchanged")
        );
        let next = added(
            database
                .insert_workspace(&root("/selected", "/canonical", "identity"), 200)
                .unwrap(),
        );
        assert!(next.id.get() > first_id.get());
    }

    // Catches losing SQLite's own uniqueness/STRICT/check constraints behind API prechecks.
    #[test]
    fn workspace_schema_enforces_storage_invariants() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .insert_workspace(&root("/selected", "/canonical", "identity"), 100)
            .unwrap();
        let sql = "INSERT INTO workspaces (selected_path, canonical_path, identity_key, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        for (selected, canonical, identity) in [
            ("/different", "/different", "identity"),
            ("/selected", "/different", "different"),
            ("/different", "/canonical", "different"),
        ] {
            assert!(database
                .connection
                .execute(
                    sql,
                    rusqlite::params![selected, canonical, identity, 1, 100, 100]
                )
                .is_err());
        }
        assert!(database
            .connection
            .execute(
                sql,
                rusqlite::params!["/different", "/different", "different", 2, 100, 100]
            )
            .is_err());
        assert!(database
            .connection
            .execute(
                sql,
                rusqlite::params![
                    "/different",
                    "/different",
                    "different",
                    1,
                    "invalid-timestamp",
                    100
                ]
            )
            .is_err());
        assert_eq!(database.workspace_configuration().unwrap().records.len(), 1);
    }

    // Catches returning a mutation success before SQLite accepts the row update/delete.
    #[test]
    fn workspace_failed_row_mutations_preserve_configuration() {
        let mut database = Database::open_in_memory().unwrap();
        database.set_metadata("sentinel", "unchanged").unwrap();
        let original = added(
            database
                .insert_workspace(&root("/selected", "/canonical", "identity"), 100)
                .unwrap(),
        );
        database.connection.execute_batch("CREATE TRIGGER reject_update BEFORE UPDATE ON workspaces BEGIN SELECT RAISE(ABORT, 'sentinel update details'); END; CREATE TRIGGER reject_remove BEFORE DELETE ON workspaces BEGIN SELECT RAISE(ABORT, 'sentinel remove details'); END;").unwrap();
        assert_unavailable(database.set_workspace_enabled(original.id, false, 200));
        assert_unavailable(database.remove_workspace(original.id));
        let configuration = database.workspace_configuration().unwrap();
        assert_eq!(configuration.records.len(), 1);
        assert_eq!(WorkspaceRecord::from(&configuration.records[0]), original);
        assert_eq!(configuration.setup_state, SetupState::Configured);
        assert_eq!(
            database.metadata("sentinel").unwrap().as_deref(),
            Some("unchanged")
        );
    }

    // Catches unbounded/default contention waits or exposing the driver's locked error.
    #[test]
    fn workspace_contention_returns_a_sanitized_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let mut first = Database::open(&path).unwrap();
        let mut second = Database::open(&path).unwrap();
        let transaction = first
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let start = std::time::Instant::now();
        assert_unavailable(
            second.insert_workspace(&root("/selected", "/canonical", "identity"), 100),
        );
        assert!(start.elapsed() >= std::time::Duration::from_millis(900));
        assert_eq!(
            second
                .connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1000
        );
        transaction.commit().unwrap();
        assert_eq!(
            second.workspace_configuration().unwrap().setup_state,
            SetupState::Undecided
        );
        assert!(second.workspace_configuration().unwrap().records.is_empty());
        assert!(matches!(
            second
                .insert_workspace(&root("/selected", "/canonical", "identity"), 100)
                .unwrap(),
            WorkspaceInsertOutcome::Added(_)
        ));
    }
}
