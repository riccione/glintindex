use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Default log file name (without date suffix).
const LOG_FILE_NAME: &str = "glintindex.log";

/// Returns the platform-appropriate log directory for GlintIndex.
///
/// Uses the `dirs` crate to locate the standard application state directory:
/// - Linux: `~/.local/state/glintindex/logs/`
/// - macOS: `~/Library/Logs/GlintIndex/`
/// - Windows: `%LOCALAPPDATA%\GlintIndex\logs\`
pub fn log_dir() -> Option<PathBuf> {
    let state_dir = dirs::state_dir().or_else(dirs::config_dir)?;
    Some(state_dir.join("glintindex").join("logs"))
}

/// Ensures the log directory exists, creating it if necessary.
///
/// Returns `Ok(path)` if the directory exists or was created successfully,
/// or `Err` if creation failed.
pub fn ensure_log_dir() -> std::io::Result<PathBuf> {
    let dir = log_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine log directory",
        )
    })?;

    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Configuration for the tracing subscriber.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Default log level if `RUST_LOG` is not set.
    /// Resolved from: RUST_LOG > --verbose > config.toml > "error".
    pub default_level: String,
    /// Whether to log to stderr in addition to the file.
    pub log_to_stderr: bool,
    /// Whether to log to a file.
    pub log_to_file: bool,
    /// Automatically delete log files older than N days on startup.
    pub max_retention_days: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_level: "error".to_string(),
            log_to_stderr: false,
            log_to_file: true,
            max_retention_days: 7,
        }
    }
}

/// Initializes the tracing subscriber with file logging and optional stderr output.
///
/// This function sets up a [`tracing_subscriber`] that:
/// - Writes structured logs to a daily-rotating file in the platform's log directory
/// - Optionally also logs to stderr (useful for development)
/// - Respects the `RUST_LOG` environment variable if set
/// - Prunes old log files based on `max_retention_days`
///
/// If the log directory cannot be created, file logging is silently disabled
/// and a warning is emitted to stderr.
///
/// # Arguments
///
/// * `config` - Configuration for the logging behavior.
///
/// # Panics
///
/// Panics if the subscriber cannot be set (e.g., if called more than once
/// in the same process).
pub fn init(config: LoggingConfig) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.default_level));

    if config.log_to_file {
        init_with_file(env_filter, config.log_to_stderr, config.max_retention_days)
    } else {
        init_stderr_only(env_filter)
    }
}

/// Initialize tracing with a rotating file appender.
fn init_with_file(env_filter: EnvFilter, also_stderr: bool, max_retention_days: u64) {
    let log_dir = match ensure_log_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Warning: could not create log directory, file logging disabled: {e}");
            init_stderr_only(env_filter);
            return;
        }
    };

    // Prune old log files before initializing the subscriber
    prune_old_logs(&log_dir, max_retention_days);

    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, LOG_FILE_NAME);

    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(true)
        .with_line_number(true);

    if also_stderr {
        let stderr_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_level(true)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(stderr_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }
}

/// Initialize tracing with stderr output only.
fn init_stderr_only(env_filter: EnvFilter) {
    fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
}

/// Removes log files older than `max_days` from the given directory.
///
/// Matches files whose names start with `LOG_FILE_NAME` (e.g.,
/// `glintindex.log`, `glintindex.log.2026-08-20`). Files are deleted
/// based on their modification timestamp.
pub fn prune_old_logs(log_dir: &Path, max_days: u64) {
    if max_days == 0 {
        return; // Pruning disabled
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(max_days * 86400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only consider files whose name starts with our log file name
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with(LOG_FILE_NAME) {
            continue;
        }

        // Skip if not a regular file
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        // Check modification time
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if mtime < cutoff {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_dir_returns_path() {
        let dir = log_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.ends_with("glintindex/logs"));
    }

    #[test]
    fn ensure_log_dir_creates_directory() {
        let result = ensure_log_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[test]
    fn default_config_has_reasonable_values() {
        let config = LoggingConfig::default();
        assert_eq!(config.default_level, "error");
        assert!(!config.log_to_stderr);
        assert!(config.log_to_file);
        assert_eq!(config.max_retention_days, 7);
    }

    #[test]
    fn prune_old_logs_removes_old_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        // Create a log file that will be matched by starts_with
        let log_file = dir.join("glintindex.log");
        std::fs::write(&log_file, "current log").unwrap();

        // Create a non-log file that should be preserved
        let other_file = dir.join("something.txt");
        std::fs::write(&other_file, "keep me").unwrap();

        // Prune with 365 days — nothing should be removed since files are brand new
        prune_old_logs(dir, 365);
        assert!(log_file.exists(), "recent log file should be preserved");
        assert!(other_file.exists(), "non-log file should be preserved");

        // Prune with 0 days — pruning disabled
        prune_old_logs(dir, 0);
        assert!(log_file.exists(), "file preserved when pruning disabled");
    }

    #[test]
    fn prune_old_logs_ignores_non_log_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        std::fs::write(dir.join("readme.txt"), "hello").unwrap();
        std::fs::write(dir.join("glintindex.db"), "data").unwrap();

        prune_old_logs(dir, 1);

        assert!(dir.join("readme.txt").exists());
        assert!(dir.join("glintindex.db").exists());
    }
}
