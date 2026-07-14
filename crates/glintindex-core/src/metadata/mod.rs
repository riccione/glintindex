//! Metadata storage for indexed files.
//!
//! Provides a SQLite-backed metadata database that tracks file
//! properties for each indexed document. This enables future
//! incremental indexing by allowing the system to determine
//! whether a file has changed since it was last indexed.
//!
//! # Storage Layout
//!
//! The metadata database is stored alongside the Tantivy index:
//!
//! ```text
//! index/
//!     tantivy/
//!     metadata.db
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use glintindex_core::metadata::Repository;
//! use glintindex_core::metadata::FileMetadata;
//!
//! let repo = Repository::initialize(std::path::Path::new("/tmp/index/metadata.db")).unwrap();
//! let meta = FileMetadata::from_path(std::path::Path::new("/home/user/test.txt")).unwrap();
//! repo.upsert(&meta).unwrap();
//! ```

pub mod database;
pub mod migrations;
pub mod models;
pub mod repository;

pub use models::FileMetadata;
pub use repository::Repository;
