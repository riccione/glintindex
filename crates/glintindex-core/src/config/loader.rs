use crate::config::config::AppConfig;
use crate::error::Result;
use std::path::{Path, PathBuf};

/// Loads an [`AppConfig`] from the given TOML file path.
///
/// If the file does not exist, returns the default configuration
/// without writing anything to disk.
///
/// Tilde (`~`) in path fields is expanded to the user's home directory.
///
/// # Errors
///
/// Returns [`GlintIndexError::Config`] if the file exists but
/// cannot be parsed as valid TOML configuration.
pub fn load(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = std::fs::read_to_string(path)?;
    let mut config: AppConfig = toml::from_str(&contents)?;
    expand_tilde(&mut config.index_directory);
    Ok(config)
}

/// Expands a leading `~` in a path to the user's home directory.
///
/// If `$HOME` is not set or the path does not start with `~`,
/// the path is returned unchanged.
fn expand_tilde(path: &mut PathBuf) {
    let s = path.to_string_lossy().to_string();
    if let Some(rest) = s.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            *path = PathBuf::from(format!("{home}{rest}"));
        }
    }
}

/// Saves an [`AppConfig`] to the given TOML file path.
///
/// Parent directories are created automatically if they do not exist.
///
/// # Errors
///
/// Returns [`GlintIndexError::Io`] if the file or directories
/// cannot be created, or [`GlintIndexError::Config`] if serialization fails.
pub fn save(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string_pretty(config)?;
    std::fs::write(path, contents)?;
    Ok(())
}

/// Default configuration file content with commented examples.
const DEFAULT_CONFIG_CONTENT: &str = r#"# GlintIndex Configuration
#
# Edit this file to configure which folders are indexed.
# After changes, run `glintindex index` to update the search index.

# Folders to be indexed.
# Add your own entries below, for example:
#   { path = "/home/user/documents", enabled = true }
indexed_folders = []

# Folder names to exclude from indexing.
ignored_folders = [
  ".git",
  ".svn",
  ".hg",
  "node_modules",
  "__pycache__",
  ".DS_Store",
]

# Directory where the search index is stored.
index_directory = "~/.local/share/glintindex/index"

# Maximum number of characters in a preview snippet.
max_preview_size = 200

# Visual theme preference: light, dark, or system.
theme = "system"
"#;

/// Generates a default configuration file at the given path.
///
/// If the file already exists, this is a no-op and returns `Ok(false)`.
/// If the file is created successfully, returns `Ok(true)`.
///
/// # Errors
///
/// Returns an error if the file cannot be written.
pub fn generate_default(path: &Path) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, DEFAULT_CONFIG_CONTENT)?;
    Ok(true)
}

/// Checks whether a configuration file exists at the given path.
pub fn config_exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::{AppConfig, Theme};
    use crate::model::IndexedFolder;
    use std::path::PathBuf;

    fn temp_config_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("glintindex_test");
        std::fs::create_dir_all(&dir).ok();
        dir.join(format!("{name}.toml"))
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let path = temp_config_path("nonexistent");
        let config = load(&path).unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_config_path("roundtrip");
        let config = AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(PathBuf::from("/test"))],
            index_directory: PathBuf::from("/tmp/test-index"),
            theme: Theme::Dark,
            max_preview_size: 500,
            ..Default::default()
        };

        save(&path, &config).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(config, loaded);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_expands_tilde_in_index_directory() {
        let path = temp_config_path("tilde_expansion");
        let content = r#"
indexed_folders = []
ignored_folders = []
index_directory = "~/.local/share/glintindex/index"
max_preview_size = 200
theme = "system"
"#;
        std::fs::write(&path, content).unwrap();

        let config = load(&path).unwrap();
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(
            config.index_directory,
            PathBuf::from(format!("{home}/.local/share/glintindex/index"))
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn expand_tilde_noop_when_no_tilde() {
        let mut path = PathBuf::from("/absolute/path");
        expand_tilde(&mut path);
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn expand_tilde_noop_when_home_unset() {
        let original = std::env::var("HOME");
        // SAFETY: test-only, single-threaded
        unsafe {
            std::env::remove_var("HOME");
        }

        let mut path = PathBuf::from("~/test");
        expand_tilde(&mut path);
        assert_eq!(path, PathBuf::from("~/test"));

        if let Ok(val) = original {
            // SAFETY: test-only, single-threaded
            unsafe {
                std::env::set_var("HOME", val);
            }
        }
    }

    #[test]
    fn save_creates_parent_directories() {
        let path = temp_config_path("deep/nested/config.toml");
        let config = AppConfig::default();

        save(&path, &config).unwrap();
        assert!(path.exists());

        std::fs::remove_dir_all(temp_config_path("deep")).ok();
    }

    #[test]
    fn load_invalid_toml_returns_error() {
        let path = temp_config_path("invalid");
        std::fs::write(&path, "this is not valid toml {{{{").unwrap();

        let result = load(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TOML parse error"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn generate_default_creates_file() {
        let path = temp_config_path("generate_test");
        std::fs::remove_file(&path).ok();

        let created = generate_default(&path).unwrap();
        assert!(created);
        assert!(path.exists());

        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("GlintIndex Configuration"));
        assert!(contents.contains("ignored_folders"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn generate_default_noop_when_exists() {
        let path = temp_config_path("generate_noop");
        std::fs::write(&path, "existing content").unwrap();

        let created = generate_default(&path).unwrap();
        assert!(!created);

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "existing content");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn config_exists_returns_correctly() {
        let path = temp_config_path("exists_test");
        assert!(!config_exists(&path));

        std::fs::write(&path, "").unwrap();
        assert!(config_exists(&path));

        std::fs::remove_file(&path).ok();
    }
}
