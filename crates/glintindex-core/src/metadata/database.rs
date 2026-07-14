//! Database connection and initialization.
//!
//! Manages the SQLite connection lifecycle, including opening
//! the database file and running migrations on startup.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{GlintIndexError, Result};

use super::migrations::MIGRATIONS;

/// Opens or creates the metadata database at the given path.
///
/// Creates the database file if it does not exist, and runs
/// all necessary schema migrations. Migrations are idempotent.
///
/// # Errors
///
/// Returns [`GlintIndexError::Metadata`] if the database cannot
/// be opened or migrations fail.
pub fn open_database(db_path: &Path) -> Result<Connection> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path).map_err(|e| {
        GlintIndexError::Metadata(format!(
            "failed to open database {}: {e}",
            db_path.display()
        ))
    })?;

    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| GlintIndexError::Metadata(format!("failed to set WAL mode: {e}")))?;

    // Run migrations
    for (name, sql) in MIGRATIONS {
        conn.execute_batch(sql)
            .map_err(|e| GlintIndexError::Metadata(format!("migration '{name}' failed: {e}")))?;
    }

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_database_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("metadata.db");

        let conn = open_database(&db_path).unwrap();
        assert!(db_path.exists());

        // Verify table exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn open_database_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("metadata.db");

        let _conn1 = open_database(&db_path).unwrap();
        let _conn2 = open_database(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn open_database_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("subdir").join("metadata.db");

        let _conn = open_database(&db_path).unwrap();
        assert!(db_path.exists());
    }
}
