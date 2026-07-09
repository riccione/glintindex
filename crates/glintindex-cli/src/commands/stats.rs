use std::path::Path;

use anyhow::{Context, Result};
use glintindex_core::app::ApplicationService;

pub fn execute(config_path: &str) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    let stats = service
        .statistics()
        .context("Failed to retrieve statistics")?;

    println!("Index Statistics\n");
    println!("Indexed documents: {}", stats.indexed_documents);
    println!("Indexed folders:   {}", stats.indexed_folders);
    println!("Index location:    {}", service.index_path().display());

    if let Some(ref last) = stats.last_indexing_result {
        println!("\nLast indexing run:");
        println!("  Files indexed: {}", last.files_indexed);
        println!("  Files skipped: {}", last.files_skipped);
        println!("  Files failed:  {}", last.files_failed);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use glintindex_core::app::{ApplicationStatistics, IndexingResult};

    #[test]
    fn stats_output_formatting() {
        let stats = ApplicationStatistics::new(100, 5);
        assert_eq!(stats.indexed_documents, 100);
        assert_eq!(stats.indexed_folders, 5);
    }

    #[test]
    fn stats_with_indexing_result() {
        let result = IndexingResult::new(10, 200, 180, 15, 5);
        let stats = ApplicationStatistics::new(180, 3).with_last_indexing_result(result);
        let last = stats.last_indexing_result.unwrap();
        assert_eq!(last.files_indexed, 180);
    }
}
