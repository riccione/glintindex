pub mod search;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Search for files in the index
    Search(search::SearchArgs),
}
