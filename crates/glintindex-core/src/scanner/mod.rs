//! Filesystem scanning and file discovery.
//!
//! This module provides the [`FilesystemScanner`] which recursively
//! discovers files, applies ignore rules, filters by type, reads
//! content, and indexes documents via the [`IndexService`].
//!
//! The scanner hides all `walkdir` implementation details behind
//! a clean, application-specific API.
//!
//! # Examples
//!
//! ```no_run
//! use glintindex_core::scanner::FilesystemScanner;
//! use glintindex_core::index::IndexService;
//! use std::path::Path;
//!
//! let index_service = IndexService::open_or_create(Path::new("/tmp/index")).unwrap();
//! let scanner = FilesystemScanner::new(&index_service);
//! let stats = scanner.scan_directory(Path::new("/home/user/projects")).unwrap();
//! ```

pub(crate) mod ignore;
pub(crate) mod parser;
pub mod statistics;

pub use service::FilesystemScanner;
pub use statistics::ScannerStatistics;

mod service;
