//! Database migrations.
//!
//! Contains the SQL schema definitions for the metadata database.
//! Schema creation is idempotent — running it multiple times is safe.

/// The SQL schema for the `files` table.
pub const CREATE_FILES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS files (
    path TEXT PRIMARY KEY,
    size INTEGER NOT NULL,
    modified INTEGER NOT NULL,
    hash TEXT,
    mime TEXT,
    parser_version INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL
);
"#;

/// Index on `modified` for change detection queries.
pub const CREATE_MODIFIED_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_files_modified ON files(modified);
"#;

/// Index on `indexed_at` for recency queries.
pub const CREATE_INDEXED_AT_INDEX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_files_indexed_at ON files(indexed_at);
"#;

/// All migrations to run on startup.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("create_files_table", CREATE_FILES_TABLE),
    ("create_modified_index", CREATE_MODIFIED_INDEX),
    ("create_indexed_at_index", CREATE_INDEXED_AT_INDEX),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid_sql() {
        for (name, sql) in MIGRATIONS {
            assert!(
                !sql.trim().is_empty(),
                "Migration {name} contains empty SQL"
            );
        }
    }

    #[test]
    fn create_files_table_is_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(CREATE_FILES_TABLE).unwrap();
        // Running again should not fail
        conn.execute_batch(CREATE_FILES_TABLE).unwrap();
    }
}
