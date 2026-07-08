use std::path::PathBuf;

use crate::config::defaults;
use crate::model::IndexedFolder;

/// Visual theme preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Light color scheme.
    Light,
    /// Dark color scheme.
    Dark,
    /// Follow the system preference.
    #[default]
    System,
}

/// Application configuration.
///
/// Holds all persistent settings for the GlintIndex application.
/// The configuration is serializable to TOML and can be loaded from
/// disk or created with sensible defaults.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    /// Folders to be indexed.
    pub indexed_folders: Vec<IndexedFolder>,
    /// Folder names or patterns to exclude from indexing.
    pub ignored_folders: Vec<String>,
    /// Directory where the search index is stored.
    pub index_directory: PathBuf,
    /// Visual theme preference.
    pub theme: Theme,
    /// Maximum number of characters in a preview snippet.
    pub max_preview_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            indexed_folders: Vec::new(),
            ignored_folders: defaults::default_ignored_folders(),
            index_directory: defaults::default_index_directory(),
            theme: Theme::default(),
            max_preview_size: defaults::default_max_preview_size(),
        }
    }
}

impl AppConfig {
    /// Returns only the enabled indexed folders.
    pub fn enabled_folders(&self) -> Vec<&IndexedFolder> {
        self.indexed_folders.iter().filter(|f| f.enabled).collect()
    }

    /// Returns `true` if the given folder name should be ignored.
    pub fn is_ignored(&self, name: &str) -> bool {
        self.ignored_folders.iter().any(|i| i == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AppConfig::default();
        assert!(config.indexed_folders.is_empty());
        assert!(!config.ignored_folders.is_empty());
        assert_eq!(config.max_preview_size, 200);
        assert_eq!(config.theme, Theme::System);
    }

    #[test]
    fn enabled_folders() {
        let mut config = AppConfig::default();
        config.indexed_folders = vec![
            IndexedFolder::enabled(PathBuf::from("/a")),
            IndexedFolder::disabled(PathBuf::from("/b")),
            IndexedFolder::enabled(PathBuf::from("/c")),
        ];
        let enabled = config.enabled_folders();
        assert_eq!(enabled.len(), 2);
        assert_eq!(enabled[0].path, PathBuf::from("/a"));
        assert_eq!(enabled[1].path, PathBuf::from("/c"));
    }

    #[test]
    fn is_ignored() {
        let config = AppConfig::default();
        assert!(config.is_ignored(".git"));
        assert!(config.is_ignored("node_modules"));
        assert!(!config.is_ignored("src"));
    }

    #[test]
    fn roundtrip_serde() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let restored: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, restored);
    }
}
