use clap::Parser as _;

use crate::commands::{self, Command};

#[derive(clap::Parser)]
#[command(
    name = "glintindex-cli",
    version,
    about = "Local desktop search engine"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Search(args) => commands::search::execute(args),
    }
}
