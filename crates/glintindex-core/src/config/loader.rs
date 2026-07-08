use crate::config::config::AppConfig;
use crate::error::Result;
use std::path::Path;

/// Loads an [`AppConfig`] from the given TOML file path.
///
/// If the file does not exist, returns the default configuration
/// without writing anything to disk.
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
    let config: AppConfig = toml::from_str(&contents)?;
    Ok(config)
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
}
