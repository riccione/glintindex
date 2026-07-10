use std::path::Path;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use glintindex_core::app::ApplicationService;
use glintindex_core::config::loader;

#[derive(Args)]
pub struct FoldersArgs {
    #[command(subcommand)]
    pub command: FoldersCommand,
}

#[derive(Subcommand)]
pub enum FoldersCommand {
    /// List all configured folders and their status
    ///
    /// Shows every folder in the configuration with its enabled/disabled state.
    ///
    /// # Example
    ///
    ///   glintindex folders list
    List,

    /// Add a folder to the configuration
    ///
    /// Adds the folder as an enabled entry in index.toml.
    /// The path is resolved to an absolute path.
    /// Does not start indexing automatically.
    ///
    /// # Example
    ///
    ///   glintindex folders add ~/Documents
    Add {
        /// Path to the folder to add
        path: String,
    },

    /// Remove a folder from the configuration
    ///
    /// Removes the folder entry from index.toml.
    /// Does not modify the existing search index.
    ///
    /// # Example
    ///
    ///   glintindex folders remove ~/Documents
    Remove {
        /// Path to the folder to remove
        path: String,
    },

    /// Enable a disabled folder
    ///
    /// Marks the folder as enabled in index.toml.
    /// Does not trigger indexing automatically.
    ///
    /// # Example
    ///
    ///   glintindex folders enable ~/Documents
    Enable {
        /// Path to the folder to enable
        path: String,
    },

    /// Disable a folder without removing it
    ///
    /// Marks the folder as disabled in index.toml.
    /// Does not remove indexed documents from the search index.
    ///
    /// # Example
    ///
    ///   glintindex folders disable ~/Documents
    Disable {
        /// Path to the folder to disable
        path: String,
    },
}

pub fn execute(config_path: &str, command: FoldersCommand) -> Result<()> {
    let path = Path::new(config_path);

    if !loader::config_exists(path) {
        let created =
            loader::generate_default(path).context("Failed to create configuration file")?;
        if created {
            println!("Created configuration file: {}", config_path);
            println!();
        }
    }

    match command {
        FoldersCommand::List => execute_list(path),
        FoldersCommand::Add { path: folder } => execute_add(path, &folder),
        FoldersCommand::Remove { path: folder } => execute_remove(path, &folder),
        FoldersCommand::Enable { path: folder } => execute_enable(path, &folder),
        FoldersCommand::Disable { path: folder } => execute_disable(path, &folder),
    }
}

fn execute_list(config_path: &Path) -> Result<()> {
    let service = ApplicationService::with_config_path(config_path)
        .context("Failed to initialize application service")?;

    let folders = service.indexed_folders();

    if folders.is_empty() {
        println!("No folders configured.");
        println!();
        println!("Add a folder:");
        println!("  glintindex folders add /path/to/folder");
        return Ok(());
    }

    println!("Configured folders\n");
    for folder in &folders {
        let icon = if folder.enabled { "✓" } else { "✗" };
        println!("{icon} {}", folder.path.display());
    }

    Ok(())
}

fn execute_add(config_path: &Path, folder: &str) -> Result<()> {
    let folder_path = Path::new(folder);

    let mut service = ApplicationService::with_config_path(config_path)
        .context("Failed to initialize application service")?;

    service
        .add_folder(folder_path)
        .context("Failed to add folder")?;

    let resolved = folder_path
        .canonicalize()
        .context("Failed to resolve folder path")?;

    println!("Folder added:\n");
    println!("{}", resolved.display());

    Ok(())
}

fn execute_remove(config_path: &Path, folder: &str) -> Result<()> {
    let folder_path = Path::new(folder);

    let mut service = ApplicationService::with_config_path(config_path)
        .context("Failed to initialize application service")?;

    let resolved = folder_path
        .canonicalize()
        .context("Failed to resolve folder path")?;

    service
        .remove_folder(folder_path)
        .context("Failed to remove folder")?;

    println!("Folder removed:\n");
    println!("{}", resolved.display());

    Ok(())
}

fn execute_enable(config_path: &Path, folder: &str) -> Result<()> {
    let folder_path = Path::new(folder);

    let mut service = ApplicationService::with_config_path(config_path)
        .context("Failed to initialize application service")?;

    let resolved = folder_path
        .canonicalize()
        .context("Failed to resolve folder path")?;

    service
        .enable_folder(folder_path)
        .context("Failed to enable folder")?;

    println!("Folder enabled:\n");
    println!("{}", resolved.display());

    Ok(())
}

fn execute_disable(config_path: &Path, folder: &str) -> Result<()> {
    let folder_path = Path::new(folder);

    let mut service = ApplicationService::with_config_path(config_path)
        .context("Failed to initialize application service")?;

    let resolved = folder_path
        .canonicalize()
        .context("Failed to resolve folder path")?;

    service
        .disable_folder(folder_path)
        .context("Failed to disable folder")?;

    println!("Folder disabled:\n");
    println!("{}", resolved.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn list_args_no_folders() {
        let dir = std::env::temp_dir().join("glintindex_folders_list_test");
        fs::create_dir_all(&dir).ok();
        let config_path = dir.join("index.toml");
        fs::remove_file(&config_path).ok();

        let config = glintindex_core::config::AppConfig {
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        execute_list(&config_path).unwrap();

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_folder_to_config() {
        let dir = std::env::temp_dir().join("glintindex_folders_add_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let config = glintindex_core::config::AppConfig {
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        execute_add(&config_path, test_dir.to_str().unwrap()).unwrap();

        let loaded = loader::load(&config_path).unwrap();
        assert_eq!(loaded.indexed_folders.len(), 1);
        assert!(loaded.indexed_folders[0].enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_duplicate_folder_fails() {
        let dir = std::env::temp_dir().join("glintindex_folders_dupe_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let resolved = test_dir.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![glintindex_core::model::IndexedFolder::enabled(resolved)],
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let result = execute_add(&config_path, test_dir.to_str().unwrap());
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_folder_from_config() {
        let dir = std::env::temp_dir().join("glintindex_folders_remove_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let resolved = test_dir.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![glintindex_core::model::IndexedFolder::enabled(resolved)],
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        execute_remove(&config_path, test_dir.to_str().unwrap()).unwrap();

        let loaded = loader::load(&config_path).unwrap();
        assert!(loaded.indexed_folders.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_unconfigured_folder_fails() {
        let dir = std::env::temp_dir().join("glintindex_folders_remove_unconfigured_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let config = glintindex_core::config::AppConfig {
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let result = execute_remove(&config_path, test_dir.to_str().unwrap());
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enable_folder_in_config() {
        let dir = std::env::temp_dir().join("glintindex_folders_enable_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let resolved = test_dir.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![glintindex_core::model::IndexedFolder::disabled(resolved)],
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        execute_enable(&config_path, test_dir.to_str().unwrap()).unwrap();

        let loaded = loader::load(&config_path).unwrap();
        assert!(loaded.indexed_folders[0].enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn disable_folder_in_config() {
        let dir = std::env::temp_dir().join("glintindex_folders_disable_test");
        let test_dir = dir.join("docs");
        fs::create_dir_all(&test_dir).unwrap();

        let config_path = dir.join("index.toml");
        let resolved = test_dir.canonicalize().unwrap();
        let config = glintindex_core::config::AppConfig {
            indexed_folders: vec![glintindex_core::model::IndexedFolder::enabled(resolved)],
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        execute_disable(&config_path, test_dir.to_str().unwrap()).unwrap();

        let loaded = loader::load(&config_path).unwrap();
        assert!(!loaded.indexed_folders[0].enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_invalid_path_fails() {
        let dir = std::env::temp_dir().join("glintindex_folders_invalid_test");
        fs::create_dir_all(&dir).ok();
        let config_path = dir.join("index.toml");
        let config = glintindex_core::config::AppConfig {
            index_directory: dir.join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let result = execute_add(&config_path, "/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }
}
