use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use thiserror::Error;

mod workspaces;

#[derive(Debug, Error)]
#[error("{kind}")]
pub struct PersistenceError {
    kind: PersistenceErrorKind,
}

#[derive(Debug, Error)]
enum PersistenceErrorKind {
    #[error("failed to open persistence database")]
    Open,
    #[error("failed to query persistence database")]
    Query,
    #[error("failed to migrate persistence database")]
    Migration,
}

impl PersistenceError {
    fn from_open(_error: rusqlite::Error) -> Self {
        Self {
            kind: PersistenceErrorKind::Open,
        }
    }

    fn from_query(_error: rusqlite::Error) -> Self {
        Self {
            kind: PersistenceErrorKind::Query,
        }
    }

    fn from_migration(_error: rusqlite_migration::Error) -> Self {
        Self {
            kind: PersistenceErrorKind::Migration,
        }
    }
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        let connection = Connection::open(path).map_err(PersistenceError::from_open)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let connection = Connection::open_in_memory().map_err(PersistenceError::from_open)?;
        Self::from_connection(connection)
    }

    pub fn schema_version(&self) -> Result<u32, PersistenceError> {
        self.connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(PersistenceError::from_query)
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) -> Result<(), PersistenceError> {
        self.connection
            .execute(
                "INSERT INTO application_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(PersistenceError::from_query)?;
        Ok(())
    }

    pub fn metadata(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT value FROM application_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(PersistenceError::from_query)
    }

    fn from_connection(connection: Connection) -> Result<Self, PersistenceError> {
        connection
            .busy_timeout(std::time::Duration::from_secs(1))
            .map_err(PersistenceError::from_open)?;
        let mut database = Self { connection };
        database.apply_migrations()?;
        Ok(database)
    }

    fn apply_migrations(&mut self) -> Result<(), PersistenceError> {
        let migrations = Migrations::new(vec![
            M::up(include_str!("../migrations/0001_application_metadata.sql")),
            M::up(include_str!("../migrations/0002_workspaces.sql")),
        ]);
        migrations
            .to_latest(&mut self.connection)
            .map_err(PersistenceError::from_migration)
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, path::PathBuf};

    use super::{Database, PersistenceError};

    fn assert_error_is_sanitized(error: PersistenceError, expected_message: &str, sentinel: &str) {
        assert_eq!(error.to_string(), expected_message);
        assert_eq!(
            (
                error.to_string().contains(sentinel),
                format!("{error:?}").contains(sentinel),
                error.source().is_some(),
            ),
            (false, false, false)
        );
    }

    #[test]
    fn open_error_does_not_expose_a_local_path() {
        const SENTINEL_PATH: &str = "/private/sentinel/customer-secret/app.db";
        let error =
            PersistenceError::from_open(rusqlite::Error::InvalidPath(PathBuf::from(SENTINEL_PATH)));

        assert_error_is_sanitized(error, "failed to open persistence database", SENTINEL_PATH);
    }

    #[test]
    fn migration_error_does_not_expose_attempted_sql() {
        const SENTINEL_SQL: &str = "SELECT sentinel_customer_secret FROM private_records";
        let error = PersistenceError::from_migration(rusqlite_migration::Error::RusqliteError {
            query: SENTINEL_SQL.to_owned(),
            err: rusqlite::Error::InvalidQuery,
        });

        assert_error_is_sanitized(
            error,
            "failed to migrate persistence database",
            SENTINEL_SQL,
        );
    }

    #[test]
    fn query_failure_retains_its_sanitized_error_kind() {
        let error = PersistenceError::from_query(rusqlite::Error::InvalidQuery);

        assert_eq!(error.to_string(), "failed to query persistence database");
        assert!(error.source().is_none());
    }

    #[test]
    fn fresh_database_applies_the_latest_schema_version() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(database.schema_version().unwrap(), 2);
    }

    #[test]
    fn migration_is_repeatable_for_an_existing_connection() {
        let mut database = Database::open_in_memory().unwrap();
        database.apply_migrations().unwrap();
        assert_eq!(database.schema_version().unwrap(), 2);
    }

    #[test]
    fn migration_persists_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");

        {
            let mut database = Database::open(&path).unwrap();
            database.set_metadata("installation", "local").unwrap();
            assert_eq!(database.schema_version().unwrap(), 2);
        }

        let database = Database::open(&path).unwrap();
        assert_eq!(database.schema_version().unwrap(), 2);
        assert_eq!(
            database.metadata("installation").unwrap(),
            Some("local".to_owned())
        );
    }

    // Catches losing existing metadata or failing to append the workspace migration.
    #[test]
    fn workspace_migrates_schema_one_without_data_loss() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection
                .execute_batch(include_str!("../migrations/0001_application_metadata.sql"))
                .unwrap();
            connection.pragma_update(None, "user_version", 1).unwrap();
            connection
                .execute(
                    "INSERT INTO application_metadata VALUES (?1, ?2)",
                    ["sentinel", "unchanged"],
                )
                .unwrap();
        }
        for _ in 0..2 {
            let mut database = Database::open(&path).unwrap();
            database.apply_migrations().unwrap();
            assert_eq!(database.schema_version().unwrap(), 2);
            assert_eq!(
                database.metadata("sentinel").unwrap().as_deref(),
                Some("unchanged")
            );
            let count: i64 = database
                .connection
                .query_row("SELECT COUNT(*) FROM workspaces", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 0);
            assert_eq!(database.metadata("workspace_setup_state_v1").unwrap(), None);
        }
    }

    // Catches partial migration DDL or version writes escaping rollback.
    #[test]
    fn workspace_failed_migration_rolls_back_every_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");
        let mut connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_application_metadata.sql"))
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection
            .execute(
                "INSERT INTO application_metadata VALUES ('sentinel', 'unchanged')",
                [],
            )
            .unwrap();
        let migrations = rusqlite_migration::Migrations::new(vec![
            rusqlite_migration::M::up(include_str!("../migrations/0001_application_metadata.sql")),
            rusqlite_migration::M::up("CREATE TABLE partial (id INTEGER); UPDATE application_metadata SET value = 'changed'; INVALID SQL;"),
        ]);
        assert!(migrations.to_latest(&mut connection).is_err());
        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT value FROM application_metadata WHERE key = 'sentinel'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "unchanged"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = 'partial'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
