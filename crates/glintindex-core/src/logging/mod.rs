//! Structured logging infrastructure for GlintIndex.
//!
//! This module provides centralized logging initialization using the `tracing` ecosystem.
//! Logs are written to a rotating file in the platform-appropriate log directory,
//! with optional stderr output for development.
//!
//! # Log Location
//!
//! Logs are stored in the platform's standard application state directory:
//!
//! | Platform | Path |
//! |----------|------|
//! | Linux | `~/.local/state/glintindex/logs/glintindex.log` |
//! | macOS | `~/Library/Logs/GlintIndex/glintindex.log` |
//! | Windows | `%LOCALAPPDATA%\GlintIndex\logs\glintindex.log` |
//!
//! # Log Rotation
//!
//! The log file appends continuously. For production deployments, consider using
//! system log rotation (e.g., `logrotate` on Linux) or manually pruning old logs.
//!
//! # Usage
//!
//! Initialize logging once at application startup:
//!
//! ```rust
//! use glintindex_core::logging;
//!
//! logging::init(logging::LoggingConfig::default());
//! ```
//!
//! Then use tracing macros throughout the codebase:
//!
//! ```rust,ignore
//! tracing::info!("indexing started");
//! tracing::error!(parser = "pdf", path = %file_path, "parse failed");
//! ```

mod init;

pub use init::{ensure_log_dir, init, log_dir, LoggingConfig};
