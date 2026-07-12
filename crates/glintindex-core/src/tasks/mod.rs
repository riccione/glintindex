//! Background task management for long-running indexing operations.
//!
//! This module provides the [`TaskManager`] which coordinates background
//! indexing and rebuild operations, along with progress reporting types
//! for monitoring operation status.
//!
//! # Architecture
//!
//! The task manager spawns background threads for long-running operations
//! and communicates progress through shared state protected by `Mutex`.
//! Only one job may execute at a time — attempting to start a second job
//! while one is already running returns an error.
//!
//! # Job Lifecycle
//!
//! ```text
//! Pending → Running → Completed
//!                  ↘ Failed
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use glintindex_core::{IndexService, TaskManager};
//!
//! let index_service = IndexService::open_or_create(std::path::Path::new("/tmp/index")).unwrap();
//! let index_service = Arc::new(Mutex::new(index_service));
//! let manager = TaskManager::new(index_service);
//!
//! // Start a background indexing job
//! // let id = manager.start_index_all(&config).unwrap();
//!
//! // Check progress
//! // if let Some(progress) = manager.current_progress() {
//! //     println!("Progress: {}", progress.status_message);
//! // }
//! ```

pub mod job;
pub mod manager;
pub mod progress;

pub use job::{JobId, JobState, JobStatus, JobType};
pub use manager::TaskManager;
pub use progress::Progress;
