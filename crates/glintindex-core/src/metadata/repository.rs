//! Metadata repository abstraction.
//!
//! Provides a high-level API for managing file metadata in the
//! SQLite database. All SQL is encapsulated within this module.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{GlintIndexError, Result};

use super::database::open_database;
use super::models::FileMetadata;

/// Manages file metadata in the SQLite database.
///
/// The repository is the only interface for metadata operations.
/// All SQL queries are encapsulated here — callers never touch
/// raw SQL.
///
/// # Thread Safety
///
/// The repository wraps a `Connection` which is not `Send` or `Sync`.
/// For multi-threaded access, each thread should create its own
/// repository instance, or use a `Mutex<Repository>`.
pub struct Repository {
    conn: Connection,
}

impl Repository {
    /// Opens or creates the metadata database at the given path.
    ///
    /// The database is stored alongside the Tantivy index in the
    /// configured index directory as `metadata.db`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or
    /// migrations fail.
    pub fn initialize(db_path: &Path) -> Result<Self> {
        let conn = open_database(db_path)?;
        Ok(Self { conn })
    }

    /// Retrieves metadata for a file by its path.
    ///
    /// Returns `None` if no record exists for the given path.
    pub fn get(&self, path: &str) -> Result<Option<FileMetadata>> {
        let mut stmt = self.conn
            .prepare("SELECT path, size, modified, hash, mime, parser_version, indexed_at FROM files WHERE path = ?1")
            .map_err(|e| GlintIndexError::Metadata(format!("prepare failed: {e}")))?;

        let result = stmt
            .query_row([path], |row| {
                Ok(FileMetadata {
                    path: row.get(0)?,
                    size: row.get(1)?,
                    modified: row.get(2)?,
                    hash: row.get(3)?,
                    mime: row.get(4)?,
                    parser_version: row.get(5)?,
                    indexed_at: row.get(6)?,
                })
            })
            .optional()
            .map_err(|e| GlintIndexError::Metadata(format!("query failed: {e}")))?;

        Ok(result)
    }

    /// Inserts or updates a file's metadata record.
    ///
    /// If a record with the same path already exists, it is
    /// replaced. This is an upsert operation.
    pub fn upsert(&self, metadata: &FileMetadata) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO files (path, size, modified, hash, mime, parser_version, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    metadata.path,
                    metadata.size,
                    metadata.modified,
                    metadata.hash,
                    metadata.mime,
                    metadata.parser_version,
                    metadata.indexed_at,
                ],
            )
            .map_err(|e| GlintIndexError::Metadata(format!("upsert failed: {e}")))?;
        Ok(())
    }

    /// Removes a file's metadata record by path.
    ///
    /// Returns the number of rows deleted (0 or 1).
    pub fn remove(&self, path: &str) -> Result<usize> {
        let rows = self
            .conn
            .execute("DELETE FROM files WHERE path = ?1", [path])
            .map_err(|e| GlintIndexError::Metadata(format!("remove failed: {e}")))?;
        Ok(rows)
    }

    /// Returns `true` if a metadata record exists for the given path.
    pub fn exists(&self, path: &str) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM files WHERE path = ?1",
                [path],
                |row| row.get(0),
            )
            .map_err(|e| GlintIndexError::Metadata(format!("exists query failed: {e}")))?;
        Ok(count > 0)
    }

    /// Returns the total number of files tracked in the metadata database.
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
            .map_err(|e| GlintIndexError::Metadata(format!("count query failed: {e}")))?;
        Ok(count)
    }

    /// Removes all metadata records from the database.
    ///
    /// This is used during index rebuild to keep the metadata
    /// synchronized with the cleared Tantivy index.
    pub fn clear(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM files", [])
            .map_err(|e| GlintIndexError::Metadata(format!("clear failed: {e}")))?;
        Ok(())
    }
}

/// Extension trait for optional query results.
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo() -> (Repository, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("metadata.db");
        let repo = Repository::initialize(&db_path).unwrap();
        (repo, tmp)
    }

    fn sample_metadata(path: &str) -> FileMetadata {
        FileMetadata::new(
            path.to_string(),
            1024,
            1700000000,
            None,
            Some("text/plain".to_string()),
            1,
        )
    }

    #[test]
    fn upsert_and_get() {
        let (repo, _tmp) = setup_repo();
        let meta = sample_metadata("/home/user/test.txt");

        repo.upsert(&meta).unwrap();

        let retrieved = repo.get("/home/user/test.txt").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.path, meta.path);
        assert_eq!(retrieved.size, meta.size);
        assert_eq!(retrieved.modified, meta.modified);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let (repo, _tmp) = setup_repo();
        let result = repo.get("/nonexistent/path").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn upsert_replaces_existing() {
        let (repo, _tmp) = setup_repo();
        let meta1 = FileMetadata::new(
            "/home/user/test.txt".to_string(),
            100,
            1700000000,
            None,
            None,
            1,
        );
        let meta2 = FileMetadata::new(
            "/home/user/test.txt".to_string(),
            200,
            1700001000,
            None,
            None,
            1,
        );

        repo.upsert(&meta1).unwrap();
        repo.upsert(&meta2).unwrap();

        let retrieved = repo.get("/home/user/test.txt").unwrap().unwrap();
        assert_eq!(retrieved.size, 200);
        assert_eq!(retrieved.modified, 1700001000);
    }

    #[test]
    fn remove_and_exists() {
        let (repo, _tmp) = setup_repo();
        let meta = sample_metadata("/home/user/test.txt");

        assert!(!repo.exists("/home/user/test.txt").unwrap());
        repo.upsert(&meta).unwrap();
        assert!(repo.exists("/home/user/test.txt").unwrap());

        let removed = repo.remove("/home/user/test.txt").unwrap();
        assert_eq!(removed, 1);
        assert!(!repo.exists("/home/user/test.txt").unwrap());
    }

    #[test]
    fn count() {
        let (repo, _tmp) = setup_repo();
        assert_eq!(repo.count().unwrap(), 0);

        repo.upsert(&sample_metadata("/a.txt")).unwrap();
        repo.upsert(&sample_metadata("/b.txt")).unwrap();
        assert_eq!(repo.count().unwrap(), 2);
    }

    #[test]
    fn clear() {
        let (repo, _tmp) = setup_repo();
        repo.upsert(&sample_metadata("/a.txt")).unwrap();
        repo.upsert(&sample_metadata("/b.txt")).unwrap();
        assert_eq!(repo.count().unwrap(), 2);

        repo.clear().unwrap();
        assert_eq!(repo.count().unwrap(), 0);
    }
}
