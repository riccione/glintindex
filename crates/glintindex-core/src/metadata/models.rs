//! Metadata models for indexed files.
//!
//! Defines the [`FileMetadata`] struct that represents a single
//! file's indexing metadata stored in the SQLite database.

use std::path::Path;

/// Metadata for a single indexed file.
///
/// Stores information about when a file was last indexed, its size,
/// modification time, and other properties needed for future
/// incremental indexing.
///
/// This is distinct from [`crate::model::Document`] — `Document`
/// represents a file's content for the search index, while
/// `FileMetadata` represents the file's properties for change detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// Absolute path to the file.
    pub path: String,
    /// File size in bytes at the time of indexing.
    pub size: u64,
    /// Last modification time as Unix timestamp (seconds since epoch).
    pub modified: i64,
    /// Optional content hash for change detection.
    pub hash: Option<String>,
    /// Optional MIME type of the file.
    pub mime: Option<String>,
    /// Version of the parser used to index this file.
    pub parser_version: i32,
    /// Unix timestamp when the file was last indexed.
    pub indexed_at: i64,
}

impl FileMetadata {
    /// Creates a new `FileMetadata` for a file at the given path.
    ///
    /// Reads the file's metadata from disk to populate size and
    /// modification time. Sets `indexed_at` to the current time.
    pub fn from_path(path: &Path) -> std::result::Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        Ok(Self {
            path: path.to_string_lossy().to_string(),
            size,
            modified,
            hash: None,
            mime: None,
            parser_version: 1,
            indexed_at: chrono_now(),
        })
    }

    /// Creates a new `FileMetadata` with explicit values.
    pub fn new(
        path: String,
        size: u64,
        modified: i64,
        hash: Option<String>,
        mime: Option<String>,
        parser_version: i32,
    ) -> Self {
        Self {
            path,
            size,
            modified,
            hash,
            mime,
            parser_version,
            indexed_at: chrono_now(),
        }
    }
}

/// Returns the current time as a Unix timestamp.
fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_metadata_new() {
        let meta = FileMetadata::new(
            "/home/user/test.txt".to_string(),
            1024,
            1700000000,
            None,
            Some("text/plain".to_string()),
            1,
        );
        assert_eq!(meta.path, "/home/user/test.txt");
        assert_eq!(meta.size, 1024);
        assert_eq!(meta.modified, 1700000000);
        assert!(meta.hash.is_none());
        assert_eq!(meta.mime.as_deref(), Some("text/plain"));
        assert_eq!(meta.parser_version, 1);
        assert!(meta.indexed_at > 0);
    }

    #[test]
    fn file_metadata_from_path() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let meta = FileMetadata::from_path(&file_path).unwrap();
        assert_eq!(meta.path, file_path.to_string_lossy());
        assert_eq!(meta.size, 5);
        assert!(meta.modified > 0);
        assert!(meta.indexed_at > 0);
    }
}
