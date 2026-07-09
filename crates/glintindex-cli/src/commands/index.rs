use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use glintindex_core::app::ApplicationService;

#[derive(Args)]
pub struct IndexArgs {
    /// Index a specific folder instead of all configured folders
    #[arg(short, long)]
    pub folder: Option<String>,
}

pub fn execute(config_path: &str, args: IndexArgs) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    match args.folder {
        Some(folder) => {
            let path = Path::new(&folder);
            if !path.exists() {
                anyhow::bail!("Folder does not exist: {}", folder);
            }

            tracing::info!("Indexing folder: {}", folder);
            let stats = service
                .index_folder(path)
                .context("Failed to index folder")?;

            println!("\nIndexing completed\n");
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
                println!("Add folders to your configuration file first.");
                return Ok(());
            }

            tracing::info!("Indexing all configured folders");
            let results = service.index_all().context("Failed to index folders")?;

            let total_indexed: u64 = results.iter().map(|s| s.files_indexed).sum();
            let total_skipped: u64 = results.iter().map(|s| s.files_skipped).sum();
            let total_failed: u64 = results.iter().map(|s| s.files_failed).sum();

            println!("\nIndexing completed\n");
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
}
