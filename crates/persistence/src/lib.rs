use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("failed to open persistence database")]
    Open(#[source] rusqlite::Error),
    #[error("failed to query persistence database")]
    Query(#[source] rusqlite::Error),
    #[error("failed to migrate persistence database")]
    Migration(#[source] rusqlite_migration::Error),
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        let connection = Connection::open(path).map_err(PersistenceError::Open)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let connection = Connection::open_in_memory().map_err(PersistenceError::Open)?;
        Self::from_connection(connection)
    }

    pub fn schema_version(&self) -> Result<u32, PersistenceError> {
        self.connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(PersistenceError::Query)
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) -> Result<(), PersistenceError> {
        self.connection
            .execute(
                "INSERT INTO application_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(PersistenceError::Query)?;
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
            .map_err(PersistenceError::Query)
    }

    fn from_connection(connection: Connection) -> Result<Self, PersistenceError> {
        let mut database = Self { connection };
        database.apply_migrations()?;
        Ok(database)
    }

    fn apply_migrations(&mut self) -> Result<(), PersistenceError> {
        let migrations = Migrations::new(vec![M::up(include_str!(
            "../migrations/0001_application_metadata.sql"
        ))]);
        migrations
            .to_latest(&mut self.connection)
            .map_err(PersistenceError::Migration)
    }
}

#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn fresh_database_applies_the_first_schema_version() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(database.schema_version().unwrap(), 1);
    }

    #[test]
    fn migration_is_repeatable_for_an_existing_connection() {
        let mut database = Database::open_in_memory().unwrap();
        database.apply_migrations().unwrap();
        assert_eq!(database.schema_version().unwrap(), 1);
    }

    #[test]
    fn migration_persists_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.db");

        {
            let mut database = Database::open(&path).unwrap();
            database.set_metadata("installation", "local").unwrap();
            assert_eq!(database.schema_version().unwrap(), 1);
        }

        let database = Database::open(&path).unwrap();
        assert_eq!(database.schema_version().unwrap(), 1);
        assert_eq!(
            database.metadata("installation").unwrap(),
            Some("local".to_owned())
        );
    }
}
