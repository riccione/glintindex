pub mod config;
pub mod index;
pub mod init;
pub mod rebuild;
pub mod search;
pub mod stats;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Create a default configuration file
    Init,

    /// Index configured folders
    Index(index::IndexArgs),

    /// Search indexed documents
    Search(search::SearchArgs),

    /// Display index statistics
    Stats,

    /// Rebuild the index from scratch
    Rebuild,

    /// Display current configuration
    Config,
}
