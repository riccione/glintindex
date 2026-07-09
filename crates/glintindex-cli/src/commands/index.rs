use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use glintindex_core::app::ApplicationService;
use glintindex_core::config::loader;
use glintindex_core::model::IndexedFolder;

#[derive(Args)]
pub struct IndexArgs {
    /// Index a specific folder instead of all configured folders
    #[arg(short, long)]
    pub folder: Option<String>,
}

pub fn execute(config_path: &str, args: IndexArgs) -> Result<()> {
    let path = Path::new(config_path);

    // Auto-generate config on first run
    if !loader::config_exists(path) {
        let created =
            loader::generate_default(path).context("Failed to create configuration file")?;
        if created {
            println!("Created configuration file: {}", config_path);
            println!();
        }
    }

    let service = ApplicationService::with_config_path(path)
        .context("Failed to initialize application service. Check your configuration file.")?;

    match args.folder {
        Some(folder) => {
            let folder_path = Path::new(&folder);
            if !folder_path.exists() {
                anyhow::bail!("Folder does not exist: {}", folder);
            }

            let resolved = folder_path
                .canonicalize()
                .context("Failed to resolve folder path")?;

            let mut config = loader::load(path).context("Failed to load configuration")?;
            let already_configured = config.indexed_folders.iter().any(|f| f.path == resolved);

            if !already_configured {
                config
                    .indexed_folders
                    .push(IndexedFolder::enabled(resolved.clone()));
                loader::save(path, &config).context("Failed to save configuration")?;
                println!("Added folder to configuration: {}", resolved.display());
            } else {
                let folder = config
                    .indexed_folders
                    .iter()
                    .find(|f| f.path == resolved)
                    .unwrap();
                if folder.enabled {
                    println!("Folder already configured: {}", resolved.display());
                } else {
                    println!(
                        "Folder already configured (disabled in config): {}",
                        resolved.display()
                    );
                }
            }
            println!();

            tracing::info!("Indexing folder: {}", resolved.display());
            let stats = service
                .index_folder(&resolved)
                .context("Failed to index folder")?;

            println!("Indexing completed\n");
            println!("Folders:       1");
            println!("Files indexed: {}", stats.files_indexed);

            if stats.files_skipped > 0 {
                println!("Files skipped: {}", stats.files_skipped);
            }
            if stats.files_failed > 0 {
                println!("Files failed:  {}", stats.files_failed);
            }
        }
        None => {
            let enabled = service.enabled_folders();
            if enabled.is_empty() {
                println!("No folders configured for indexing.");
                println!();
                println!("Edit {} to add folders:", config_path);
                println!();
                println!("  indexed_folders = [");
                println!("    {{ path = \"/home/user/documents\", enabled = true }},");
                println!("  ]");
                println!();
                println!("Or index a specific folder:");
                println!("  glintindex index --folder /path/to/docs");
                return Ok(());
            }

            tracing::info!("Indexing all configured folders");
            let results = service.index_all().context("Failed to index folders")?;

            let total_indexed: u64 = results.iter().map(|s| s.files_indexed).sum();
            let total_skipped: u64 = results.iter().map(|s| s.files_skipped).sum();
            let total_failed: u64 = results.iter().map(|s| s.files_failed).sum();

            println!("Indexing completed\n");
            println!("Folders:       {}", enabled.len());
            println!("Files indexed: {}", total_indexed);

            if total_skipped > 0 {
                println!("Files skipped: {}", total_skipped);
            }
            if total_failed > 0 {
                println!("Files failed:  {}", total_failed);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn index_args_defaults() {
        let args = IndexArgs { folder: None };
        assert!(args.folder.is_none());
    }

    #[test]
    fn index_args_with_folder() {
        let args = IndexArgs {
            folder: Some("/tmp/test".to_string()),
        };
        assert_eq!(args.folder.unwrap(), "/tmp/test");
    }

    #[test]
    fn folder_added_to_config_when_indexing() {
        let dir = std::env::temp_dir().join("glintindex_index_add_test");
        let test_folder = dir.join("docs");
        fs::create_dir_all(&test_folder).unwrap();
        fs::write(test_folder.join("file.txt"), "hello").unwrap();

        let config_path = dir.join("index.toml");
        let index_dir = dir.join("tantivy");
        fs::remove_file(&config_path).ok();

        let config = glintindex_core::config::AppConfig {
            index_directory: index_dir,
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let args = IndexArgs {
            folder: Some(test_folder.to_str().unwrap().to_string()),
        };
        execute(config_path.to_str().unwrap(), args).unwrap();

        let config = loader::load(&config_path).unwrap();
        let resolved = test_folder.canonicalize().unwrap();
        assert_eq!(config.indexed_folders.len(), 1);
        assert_eq!(config.indexed_folders[0].path, resolved);
        assert!(config.indexed_folders[0].enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn folder_already_in_config_not_duplicated() {
        let dir = std::env::temp_dir().join("glintindex_index_dupe_test");
        let test_folder = dir.join("docs");
        fs::create_dir_all(&test_folder).unwrap();
        fs::write(test_folder.join("file.txt"), "hello").unwrap();

        let config_path = dir.join("index.toml");
        let index_dir = dir.join("tantivy");
        let resolved = test_folder.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![IndexedFolder::enabled(resolved.clone())],
            index_directory: index_dir,
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let args = IndexArgs {
            folder: Some(test_folder.to_str().unwrap().to_string()),
        };
        execute(config_path.to_str().unwrap(), args).unwrap();

        let config = loader::load(&config_path).unwrap();
        assert_eq!(config.indexed_folders.len(), 1);
        assert_eq!(config.indexed_folders[0].path, resolved);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn disabled_folder_not_reenabled() {
        let dir = std::env::temp_dir().join("glintindex_index_disabled_test");
        let test_folder = dir.join("docs");
        fs::create_dir_all(&test_folder).unwrap();
        fs::write(test_folder.join("file.txt"), "hello").unwrap();

        let config_path = dir.join("index.toml");
        let index_dir = dir.join("tantivy");
        let resolved = test_folder.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![IndexedFolder::disabled(resolved)],
            index_directory: index_dir,
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let args = IndexArgs {
            folder: Some(test_folder.to_str().unwrap().to_string()),
        };
        execute(config_path.to_str().unwrap(), args).unwrap();

        let config = loader::load(&config_path).unwrap();
        assert_eq!(config.indexed_folders.len(), 1);
        assert!(!config.indexed_folders[0].enabled);

        fs::remove_dir_all(&dir).ok();
    }
}
