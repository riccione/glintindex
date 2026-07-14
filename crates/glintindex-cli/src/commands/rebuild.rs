use std::path::Path;

use anyhow::{Context, Result};
use glintindex_core::app::ApplicationService;

use crate::progress::ProgressBarReporter;

pub fn execute(config_path: &str) -> Result<()> {
    let service = ApplicationService::with_config_path(Path::new(config_path))
        .context("Failed to initialize application service. Check your configuration file.")?;

    let config = service.config();
    let enabled_count = config.enabled_folders().len();

    println!("Rebuilding index...\n");

    service.rebuild_index().context("Failed to rebuild index")?;

    println!("Index cleared successfully.\n");

    if enabled_count > 0 {
        println!("Re-indexing {} configured folders...\n", enabled_count);

        // Create progress reporter
        let reporter = ProgressBarReporter::new(0);

        let results = service
            .index_all_with_progress(&reporter)
            .context("Failed to re-index folders")?;

        reporter.finish_and_clear();

        let total_indexed: u64 = results.iter().map(|s| s.files_indexed).sum();
        let total_reindexed: u64 = results.iter().map(|s| s.files_reindexed).sum();
        let total_skipped: u64 = results.iter().map(|s| s.files_skipped).sum();
        let total_failed: u64 = results.iter().map(|s| s.files_failed).sum();
        let total_errors: u64 = results.iter().map(|s| s.parser_errors).sum();
        let total_panics: u64 = results.iter().map(|s| s.parser_panics).sum();

        println!("Re-indexing completed\n");
        println!("Folders:            {}", enabled_count);
        println!("Files indexed:      {}", total_indexed);
        println!("Files re-indexed:   {}", total_reindexed);
        println!("Files skipped:      {}", total_skipped);

        if total_failed > 0 {
            println!("Files failed:      {}", total_failed);
        }
        if total_errors > 0 {
            println!("Parser errors:      {}", total_errors);
        }
        if total_panics > 0 {
            println!("Parser panics:      {}", total_panics);
        }
    } else {
        println!("No folders configured for indexing.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuild_command_exists() {
        // Just verify the module compiles and the function signature is correct
        let _ = execute;
    }
}
