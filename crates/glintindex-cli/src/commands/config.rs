use std::path::Path;

use anyhow::{Context, Result};
use glintindex_core::app::ApplicationService;

pub fn execute(config_path: &str) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    let config = service.config();

    println!("Configuration\n");
    println!("Config file:      {}", config_path);
    println!("Index directory:  {}", config.index_directory.display());
    println!("Max preview size: {}", config.max_preview_size);
    println!("Theme:            {:?}", config.theme);

    println!("\nIndexed folders:");
    if config.indexed_folders.is_empty() {
        println!("  (none)");
    } else {
        for folder in &config.indexed_folders {
            let status = if folder.enabled {
                "enabled"
            } else {
                "disabled"
            };
            println!("  [{}] {}", status, folder.path.display());
        }
    }

    println!("\nIgnored folders:");
    if config.ignored_folders.is_empty() {
        println!("  (none)");
    } else {
        for ignored in &config.ignored_folders {
            println!("  {}", ignored);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use glintindex_core::AppConfig;

    #[test]
    fn config_display_fields() {
        let config = AppConfig::default();
        assert!(config.indexed_folders.is_empty());
        assert!(!config.ignored_folders.is_empty());
        assert_eq!(config.max_preview_size, 200);
    }
}
