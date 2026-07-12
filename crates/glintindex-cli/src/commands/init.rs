use std::path::Path;

use anyhow::{Context, Result};
use glintindex_core::config::loader;

pub fn execute(config_path: &str) -> Result<()> {
    let path = Path::new(config_path);

    if path.exists() {
        println!("Configuration file already exists: {}", config_path);
        println!();
        println!("Edit it directly or delete it to regenerate with defaults.");
        return Ok(());
    }

    let created = loader::generate_default(path).context("Failed to create configuration file")?;

    if created {
        println!("Created configuration file: {}", config_path);
        println!();
        println!("Next steps:");
        println!("  1. Edit the file to add folders you want to index");
        println!("  2. Run `glintindex index` to build the search index");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn init_creates_config_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("init_test.toml");

        execute(path.to_str().unwrap()).unwrap();
        assert!(path.exists());

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("GlintIndex Configuration"));
    }

    #[test]
    fn init_noop_when_config_exists() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("init_existing.toml");
        fs::write(&path, "existing").unwrap();

        execute(path.to_str().unwrap()).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "existing");
    }
}
