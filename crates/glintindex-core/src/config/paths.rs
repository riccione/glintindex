//! Centralized application path management.
//!
//! `AppPaths` is the single source of truth for all filesystem locations
//! used by GlintIndex. Neither the CLI nor the GUI should determine
//! paths independently — all location logic lives here.
//!
//! # Platform Layout
//!
//! | Platform | Configuration                          | Application Data                     |
//! |----------|----------------------------------------|--------------------------------------|
//! | Linux    | `~/.config/glintindex/`                | `~/.local/share/glintindex/`         |
//! | macOS    | `~/Library/Application Support/GlintIndex/` | `~/Library/Application Support/GlintIndex/` |
//! | Windows  | `%APPDATA%/GlintIndex/`                | `%LOCALAPPDATA%/GlintIndex/`         |
//!
//! The default configuration file is `config.toml` inside the configuration
//! directory. The default search index lives at `index/` inside the
//! application data directory.

use std::path::{Path, PathBuf};

use crate::error::Result;

/// Centralized application path resolver.
///
/// All filesystem locations used by GlintIndex are derived from platform
/// conventions via the `dirs` crate. This struct provides a single,
/// testable interface for path resolution.
///
/// # Examples
///
/// ```
/// use glintindex_core::AppPaths;
///
/// let paths = AppPaths::new();
/// assert!(paths.config_file().ends_with("config.toml"));
/// assert!(paths.index_dir().ends_with("index"));
/// ```
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Platform-specific configuration directory.
    config_dir: PathBuf,
    /// Platform-specific application data directory.
    data_dir: PathBuf,
}

impl AppPaths {
    /// Creates a new `AppPaths` using platform-standard directories.
    ///
    /// Falls back to the current working directory if the platform
    /// directory cannot be determined.
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("glintindex");

        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("glintindex");

        Self { config_dir, data_dir }
    }

    /// Returns the application configuration directory.
    ///
    /// # Examples
    ///
    /// | Platform | Path                                      |
    /// |----------|-------------------------------------------|
    /// | Linux    | `~/.config/glintindex`                    |
    /// | macOS    | `~/Library/Application Support/GlintIndex`|
    /// | Windows  | `%APPDATA%/GlintIndex`                    |
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Returns the default configuration file path.
    ///
    /// This is `config.toml` inside the configuration directory.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Returns the application data directory.
    ///
    /// # Examples
    ///
    /// | Platform | Path                                           |
    /// |----------|------------------------------------------------|
    /// | Linux    | `~/.local/share/glintindex`                    |
    /// | macOS    | `~/Library/Application Support/GlintIndex`     |
    /// | Windows  | `%LOCALAPPDATA%/GlintIndex`                    |
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Returns the default search index directory.
    ///
    /// This is `index/` inside the application data directory.
    pub fn index_dir(&self) -> PathBuf {
        self.data_dir.join("index")
    }

    /// Ensures all required application directories exist.
    ///
    /// Creates the configuration directory, data directory, and index
    /// directory if they do not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Io`] if any directory cannot be created.
    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(self.index_dir())?;
        Ok(())
    }
}

impl Default for AppPaths {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_valid_paths() {
        let paths = AppPaths::new();
        assert!(paths.config_dir().is_absolute());
        assert!(paths.data_dir().is_absolute());
    }

    #[test]
    fn config_file_inside_config_dir() {
        let paths = AppPaths::new();
        let config_file = paths.config_file();
        assert!(config_file.starts_with(paths.config_dir()));
        assert_eq!(config_file.file_name().unwrap(), "config.toml");
    }

    #[test]
    fn index_dir_inside_data_dir() {
        let paths = AppPaths::new();
        let index_dir = paths.index_dir();
        assert!(index_dir.starts_with(paths.data_dir()));
        assert!(index_dir.ends_with("index"));
    }

    #[test]
    fn config_dir_contains_glintindex() {
        let paths = AppPaths::new();
        let s = paths.config_dir().to_string_lossy();
        assert!(
            s.to_lowercase().contains("glintindex"),
            "config dir should contain 'glintindex', got: {s}"
        );
    }

    #[test]
    fn data_dir_contains_glintindex() {
        let paths = AppPaths::new();
        let s = paths.data_dir().to_string_lossy();
        assert!(
            s.to_lowercase().contains("glintindex"),
            "data dir should contain 'glintindex', got: {s}"
        );
    }

    #[test]
    fn ensure_directories_creates_all() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = tmp.path().join("config");
        let data_dir = tmp.path().join("data");

        let paths = AppPaths {
            config_dir,
            data_dir,
        };

        paths.ensure_directories().unwrap();
        assert!(paths.config_dir().exists());
        assert!(paths.data_dir().exists());
        assert!(paths.index_dir().exists());
    }

    #[test]
    fn ensure_directories_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = AppPaths {
            config_dir: tmp.path().join("cfg"),
            data_dir: tmp.path().join("data"),
        };

        paths.ensure_directories().unwrap();
        paths.ensure_directories().unwrap();
        assert!(paths.config_dir().exists());
    }
}
