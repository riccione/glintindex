//! Application service layer.
//!
//! This module provides the high-level application facade that coordinates
//! configuration, indexing, scanning, and searching. Future GUI and CLI
//! implementations should interact with this layer instead of directly
//! accessing lower-level components.
//!
//! # Architecture
//!
//! The application layer sits between the user-facing frontends (GUI, CLI)
//! and the core subsystems (config, index, scanner). It:
//!
//! - Owns the application configuration and index service
//! - Creates scanners on-the-fly for indexing operations
//! - Provides clean, domain-oriented methods for common workflows
//! - Hides Tantivy and walkdir implementation details
//!
//! # Usage
//!
//! ```no_run
//! use std::path::Path;
//! use glintindex_core::app::ApplicationService;
//! use glintindex_core::SearchQuery;
//!
//! let service = ApplicationService::with_config_path(Path::new("config.toml")).unwrap();
//!
//! // Index a folder
//! service.index_folder(Path::new("/home/user/docs")).unwrap();
//!
//! // Search
//! let results = service.search(&SearchQuery::new("hello")).unwrap();
//! ```

mod service;
pub mod statistics;

pub use service::{ApplicationService, WatcherStatus};
pub use statistics::{ApplicationStatistics, IndexingResult};
