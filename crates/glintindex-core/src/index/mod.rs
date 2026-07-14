//! Tantivy-based search index implementation.
//!
//! This module provides the core indexing and search functionality
//! built on top of the Tantivy search engine library.
//!
//! The public API is exposed through [`IndexService`], which owns
//! all Tantivy resources and provides a clean, application-specific
//! interface. Internal implementation details (schema, mapper) are
//! kept private.
//!
//! # Examples
//!
//! ```no_run
//! use glintindex_core::index::IndexService;
//! use glintindex_core::{Document, DocumentIndexer, SearchEngine, SearchQuery};
//! use std::path::Path;
//! use std::time::UNIX_EPOCH;
//!
//! let service = IndexService::open_or_create(Path::new("/tmp/index")).unwrap();
//!
//! let doc = Document::new(
//!     "/home/user/notes.txt".into(),
//!     1024,
//!     UNIX_EPOCH,
//!     "Hello world".to_string(),
//! );
//! service.add_document(&doc).unwrap();
//! service.commit().unwrap();
//!
//! let results = service.search(&SearchQuery::new("hello")).unwrap();
//! ```

mod mapper;
mod schema;
mod service;
pub mod statistics;
pub mod stats;

pub use service::IndexService;
pub use statistics::IndexStatistics;
pub use stats::IndexingStats;
