use clap::Parser as _;

use crate::commands::{self, Command};

/// GlintIndex - Local desktop search engine
#[derive(clap::Parser)]
#[command(
    name = "glintindex",
    version,
    about = "Local desktop search engine for indexing and searching files"
)]
pub struct Cli {
    /// Enable verbose logging output
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to the configuration file
    #[arg(short, long, default_value = "index.toml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Command,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_logging(cli.verbose);

    match cli.command {
        Command::Index(args) => commands::index::execute(&cli.config, args),
        Command::Search(args) => commands::search::execute(&cli.config, args),
        Command::Stats => commands::stats::execute(&cli.config),
        Command::Rebuild => commands::rebuild::execute(&cli.config),
        Command::Config => commands::config::execute(&cli.config),
    }
}

fn init_logging(verbose: bool) {
    if verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_verbose_flag() {
        let cli = Cli::try_parse_from(["glintindex", "--verbose", "stats"]).unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.command, Command::Stats));
    }

    #[test]
    fn parse_config_flag() {
        let cli =
            Cli::try_parse_from(["glintindex", "--config", "/tmp/test.toml", "stats"]).unwrap();
        assert_eq!(cli.config, "/tmp/test.toml");
    }

    #[test]
    fn parse_search_command() {
        let cli = Cli::try_parse_from(["glintindex", "search", "invoice"]).unwrap();
        match cli.command {
            Command::Search(args) => assert_eq!(args.query, "invoice"),
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn parse_index_command() {
        let cli = Cli::try_parse_from(["glintindex", "index"]).unwrap();
        assert!(matches!(cli.command, Command::Index(_)));
    }

    #[test]
    fn parse_stats_command() {
        let cli = Cli::try_parse_from(["glintindex", "stats"]).unwrap();
        assert!(matches!(cli.command, Command::Stats));
    }

    #[test]
    fn parse_rebuild_command() {
        let cli = Cli::try_parse_from(["glintindex", "rebuild"]).unwrap();
        assert!(matches!(cli.command, Command::Rebuild));
    }

    #[test]
    fn parse_config_command() {
        let cli = Cli::try_parse_from(["glintindex", "config"]).unwrap();
        assert!(matches!(cli.command, Command::Config));
    }
}
