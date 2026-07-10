use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use glintindex_core::app::ApplicationService;
use glintindex_core::config::loader;

#[derive(Args)]
pub struct ClearArgs {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub fn execute(config_path: &str, args: ClearArgs) -> Result<()> {
    let path = Path::new(config_path);

    if !loader::config_exists(path) {
        let created =
            loader::generate_default(path).context("Failed to create configuration file")?;
        if created {
            println!("Created configuration file: {}", config_path);
            println!();
        }
    }

    let service = ApplicationService::with_config_path(path)
        .context("Failed to initialize application service")?;

    let stats = service.statistics().context("Failed to get statistics")?;

    if stats.indexed_documents == 0 {
        println!("Index is already empty.");
        return Ok(());
    }

    if !args.yes {
        println!(
            "This will remove {} indexed document(s) from the search index.",
            stats.indexed_documents
        );
        println!();
        print!("Are you sure? [y/N] ");

        use std::io::Write;
        std::io::stdout()
            .flush()
            .context("Failed to flush stdout")?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    service.clear_index().context("Failed to clear index")?;

    println!("Index cleared successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn clear_args_default_no_flag() {
        let args = ClearArgs { yes: false };
        assert!(!args.yes);
    }

    #[test]
    fn clear_args_yes_flag() {
        let args = ClearArgs { yes: true };
        assert!(args.yes);
    }

    #[test]
    fn clear_empty_index_succeeds() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("index.toml");
        let config = glintindex_core::config::AppConfig {
            index_directory: tmp.path().join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        let args = ClearArgs { yes: true };
        execute(config_path.to_str().unwrap(), args).unwrap();
    }

    #[test]
    fn clear_with_documents_succeeds() {
        let tmp = TempDir::new().unwrap();
        let scan_dir = tmp.path().join("scan");
        fs::create_dir_all(&scan_dir).unwrap();
        fs::write(scan_dir.join("file.txt"), "content").unwrap();

        let config_path = tmp.path().join("index.toml");
        let config = glintindex_core::config::AppConfig {
            index_directory: tmp.path().join("tantivy"),
            ..Default::default()
        };
        loader::save(&config_path, &config).unwrap();

        {
            let service = ApplicationService::with_config_path(&config_path).unwrap();
            service.index_folder(&scan_dir).unwrap();
            let stats = service.statistics().unwrap();
            assert_eq!(stats.indexed_documents, 1);
        }

        let args = ClearArgs { yes: true };
        execute(config_path.to_str().unwrap(), args).unwrap();

        {
            let service = ApplicationService::with_config_path(&config_path).unwrap();
            let stats = service.statistics().unwrap();
            assert_eq!(stats.indexed_documents, 0);
        }
    }
}
