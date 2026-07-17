use std::path::PathBuf;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Default log file name.
const LOG_FILE_NAME: &str = "glintindex.log";

// Note: tracing-appender 0.2.x only supports time-based rotation (HOURLY, DAILY).
// Size-based rotation is not available. For production deployments, consider using
// system log rotation (e.g., logrotate) or manually pruning old log files.

/// Returns the platform-appropriate log directory for GlintIndex.
///
/// Uses the `dirs` crate to locate the standard application directory:
/// - Linux: `~/.config/glintindex/logs/`
/// - macOS: `~/Library/Logs/GlintIndex/`
/// - Windows: `%APPDATA%\GlintIndex\logs\`
pub fn log_dir() -> Option<PathBuf> {
    // Use config_dir() for consistent cross-platform behavior
    // On Windows this returns %APPDATA%, on Linux ~/.config, on macOS ~/Library/Application Support
    let config_dir = dirs::config_dir()?;
    Some(config_dir.join("glintindex").join("logs"))
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
    pub default_level: String,
    /// Whether to log to stderr in addition to the file.
    pub log_to_stderr: bool,
    /// Whether to log to a file.
    pub log_to_file: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_level: "info".to_string(),
            log_to_stderr: false,
            log_to_file: true,
        }
    }
}

/// Initializes the tracing subscriber with file logging and optional stderr output.
///
/// This function sets up a [`tracing_subscriber`] that:
/// - Writes structured logs to a rotating file in the platform's log directory
/// - Optionally also logs to stderr (useful for development)
/// - Respects the `RUST_LOG` environment variable if set
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
        init_with_file(env_filter, config.log_to_stderr)
    } else {
        init_stderr_only(env_filter)
    }
}

/// Initialize tracing with a rotating file appender.
fn init_with_file(env_filter: EnvFilter, also_stderr: bool) {
    let log_dir = match ensure_log_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Warning: could not create log directory, file logging disabled: {e}");
            init_stderr_only(env_filter);
            return;
        }
    };

    let file_appender = RollingFileAppender::new(Rotation::NEVER, &log_dir, LOG_FILE_NAME);

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
        assert_eq!(config.default_level, "info");
        assert!(!config.log_to_stderr);
        assert!(config.log_to_file);
    }
}
