//! Filesystem watching and automatic index updates.
//!
//! This module provides the [`FileWatcher`] which monitors configured
//! directories for filesystem changes and incrementally updates the
//! search index.
//!
//! The watcher is designed to integrate with [`ApplicationService`](crate::app::ApplicationService)
//! and [`IndexService`](crate::index::IndexService), providing automatic
//! index maintenance without requiring manual reindexing.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────┐
//! │   notify crate   │  Monitors filesystem for changes
//! └────────┬─────────┘
//!          │
//!          ▼
//! ┌──────────────────┐
//! │    FileWatcher   │  Converts events, applies changes
//! │  (background     │  Skips unsupported types, binaries
//! │   thread)        │  Handles errors gracefully
//! └────────┬─────────┘
//!          │
//!          ▼
//! ┌──────────────────┐
//! │   IndexService   │  add / update / remove documents
//! └──────────────────┘
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use glintindex_core::index::IndexService;
//! use glintindex_core::watcher::FileWatcher;
//! use std::path::Path;
//!
//! let index_service = IndexService::open_or_create(Path::new("/tmp/index")).unwrap();
//! let shared = Arc::new(Mutex::new(index_service));
//!
//! let mut watcher = FileWatcher::new(shared).unwrap();
//! watcher.watch_dir(Path::new("/home/user/docs")).unwrap();
//! watcher.start().unwrap();
//!
//! // The watcher now monitors /home/user/docs and automatically
//! // updates the search index when files change.
//! ```

mod events;
mod service;

pub use events::WatchEvent;
pub use service::FileWatcher;
