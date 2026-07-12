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

    println!("Index rebuilt successfully.\n");

    if enabled_count > 0 {
        println!("Re-indexing {} configured folders...", enabled_count);

        // Create progress reporter
        let reporter = ProgressBarReporter::new(0);

        let results = service
            .index_all_with_progress(&reporter)
            .context("Failed to re-index folders")?;

        reporter.finish_and_clear();

        let total_indexed: u64 = results.iter().map(|s| s.files_indexed).sum();

        println!("Re-indexing completed: {} files indexed", total_indexed);
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
