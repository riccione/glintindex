pub mod clear;
pub mod config;
pub mod folders;
pub mod index;
pub mod init;
pub mod rebuild;
pub mod search;
pub mod stats;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Create a default configuration file
    ///
    /// Generates a new index.toml with platform-specific defaults.
    /// No-op if the configuration file already exists.
    ///
    /// # Example
    ///
    ///   glintindex init
    Init,

    /// Index configured folders
    ///
    /// Indexes all enabled folders, or a specific folder with --folder.
    /// When using --folder, the folder is automatically added to the
    /// configuration for future indexing.
    ///
    /// # Examples
    ///
    ///   glintindex index
    ///   glintindex index --folder ~/Documents
    Index(index::IndexArgs),

    /// Search indexed documents
    ///
    /// Searches the index for documents matching the query.
    /// Returns matching files with preview snippets.
    ///
    /// # Example
    ///
    ///   glintindex search "invoice"
    Search(search::SearchArgs),

    /// Display index statistics
    ///
    /// Shows the number of indexed documents, configured folders,
    /// and index size on disk.
    ///
    /// # Example
    ///
    ///   glintindex stats
    Stats,

    /// Rebuild the index from scratch
    ///
    /// Discards all existing index data and recreates the index structure.
    /// Does not re-index documents automatically.
    ///
    /// # Example
    ///
    ///   glintindex rebuild
    Rebuild,

    /// Display current configuration
    ///
    /// Shows all settings from index.toml including indexed folders,
    /// ignored folders, and index directory path.
    ///
    /// # Example
    ///
    ///   glintindex config
    Config,

    /// Manage indexed folders
    ///
    /// Add, remove, enable, or disable folders in the configuration.
    /// Use the subcommands to perform specific operations.
    ///
    /// # Examples
    ///
    ///   glintindex folders list
    ///   glintindex folders add ~/Documents
    ///   glintindex folders remove ~/Documents
    ///   glintindex folders enable ~/Documents
    ///   glintindex folders disable ~/Documents
    Folders(folders::FoldersArgs),

    /// Clear all indexed documents
    ///
    /// Removes every indexed document from the search index.
    /// Preserves the index structure, configuration, and indexed folders.
    /// Requires confirmation unless --yes is used.
    ///
    /// # Examples
    ///
    ///   glintindex clear
    ///   glintindex clear --yes
    Clear(clear::ClearArgs),
}
