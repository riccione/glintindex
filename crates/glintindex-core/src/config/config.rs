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
    /// Recent search queries, newest first.
    #[serde(default)]
    pub recent_searches: Vec<String>,
    /// Application font size in pixels.
    ///
    /// Controls the base font size for all UI elements.
    /// Valid range: 8–32. Default: 12.
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    /// Split ratio for the main window's Results/Preview panes.
    ///
    /// Stored as a percentage (0–100) to avoid f32 precision issues.
    /// A value of 50 means equal widths. Range: 15–85.
    /// Default: 50.
    #[serde(default = "default_main_split_ratio")]
    pub main_split_ratio: u32,
}

/// Returns the default font size for deserialization.
fn default_font_size() -> u32 {
    12
}

/// Returns the default main split ratio for deserialization.
///
/// Returns 50 (representing 0.50 as a percentage).
fn default_main_split_ratio() -> u32 {
    50
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            indexed_folders: Vec::new(),
            ignored_folders: defaults::default_ignored_folders(),
            index_directory: defaults::default_index_directory(),
            theme: Theme::default(),
            max_preview_size: defaults::default_max_preview_size(),
            recent_searches: Vec::new(),
            font_size: default_font_size(),
            main_split_ratio: default_main_split_ratio(),
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

    /// Adds a query to the recent searches list.
    ///
    /// Moves the query to the front if it already exists.
    /// Trims whitespace and ignores empty queries.
    /// Limits the list to 20 entries maximum.
    pub fn add_recent_search(&mut self, query: String) {
        let trimmed = query.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        self.recent_searches.retain(|s| s != &trimmed);
        self.recent_searches.insert(0, trimmed);
        self.recent_searches.truncate(20);
    }

    /// Returns a reference to the recent searches list.
    pub fn recent_searches(&self) -> &[String] {
        &self.recent_searches
    }

    /// Returns a clamped font size within the valid range (8–32).
    pub fn clamped_font_size(&self) -> u32 {
        self.font_size.clamp(8, 32)
    }

    /// Returns the main split ratio as a float, clamped to [0.15, 0.85].
    pub fn main_split_ratio_f32(&self) -> f32 {
        (self.main_split_ratio as f32 / 100.0).clamp(0.15, 0.85)
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
        assert!(config.recent_searches.is_empty());
        assert_eq!(config.font_size, 12);
    }

    #[test]
    fn enabled_folders() {
        let config = AppConfig {
            indexed_folders: vec![
                IndexedFolder::enabled(PathBuf::from("/a")),
                IndexedFolder::disabled(PathBuf::from("/b")),
                IndexedFolder::enabled(PathBuf::from("/c")),
            ],
            ..Default::default()
        };
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

    #[test]
    fn add_recent_search_adds_to_front() {
        let mut config = AppConfig::default();
        config.add_recent_search("hello".to_string());
        config.add_recent_search("world".to_string());
        assert_eq!(config.recent_searches(), &["world", "hello"]);
    }

    #[test]
    fn add_recent_search_removes_duplicates() {
        let mut config = AppConfig::default();
        config.add_recent_search("hello".to_string());
        config.add_recent_search("world".to_string());
        config.add_recent_search("hello".to_string());
        assert_eq!(config.recent_searches(), &["hello", "world"]);
    }

    #[test]
    fn add_recent_search_ignores_empty() {
        let mut config = AppConfig::default();
        config.add_recent_search("".to_string());
        config.add_recent_search("  ".to_string());
        assert!(config.recent_searches().is_empty());
    }

    #[test]
    fn add_recent_search_trims_whitespace() {
        let mut config = AppConfig::default();
        config.add_recent_search("  hello  ".to_string());
        assert_eq!(config.recent_searches(), &["hello"]);
    }

    #[test]
    fn add_recent_search_limits_to_20() {
        let mut config = AppConfig::default();
        for i in 0..25 {
            config.add_recent_search(format!("query{i}"));
        }
        assert_eq!(config.recent_searches().len(), 20);
        assert_eq!(config.recent_searches()[0], "query24");
        assert_eq!(config.recent_searches()[19], "query5");
    }

    #[test]
    fn recent_searches_roundtrip_serde() {
        let mut config = AppConfig::default();
        config.add_recent_search("hello".to_string());
        config.add_recent_search("world".to_string());
        let toml_str = toml::to_string(&config).unwrap();
        let restored: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.recent_searches, restored.recent_searches);
    }
}
