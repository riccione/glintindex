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
        Command::Init => commands::init::execute(&cli.config),
        Command::Index(args) => commands::index::execute(&cli.config, args),
        Command::Search(args) => commands::search::execute(&cli.config, args),
        Command::Stats => commands::stats::execute(&cli.config),
        Command::Rebuild => commands::rebuild::execute(&cli.config),
        Command::Config => commands::config::execute(&cli.config),
        Command::Folders(args) => commands::folders::execute(&cli.config, args.command),
        Command::Clear(args) => commands::clear::execute(&cli.config, args),
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
    use crate::commands::folders::FoldersCommand;
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

    #[test]
    fn parse_init_command() {
        let cli = Cli::try_parse_from(["glintindex", "init"]).unwrap();
        assert!(matches!(cli.command, Command::Init));
    }

    #[test]
    fn parse_folders_list_command() {
        let cli = Cli::try_parse_from(["glintindex", "folders", "list"]).unwrap();
        match cli.command {
            Command::Folders(args) => assert!(matches!(args.command, FoldersCommand::List)),
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_folders_add_command() {
        let cli = Cli::try_parse_from(["glintindex", "folders", "add", "~/Documents"]).unwrap();
        match cli.command {
            Command::Folders(args) => match args.command {
                FoldersCommand::Add { path } => assert_eq!(path, "~/Documents"),
                _ => panic!("expected Folders Add command"),
            },
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_folders_remove_command() {
        let cli = Cli::try_parse_from(["glintindex", "folders", "remove", "~/Documents"]).unwrap();
        match cli.command {
            Command::Folders(args) => match args.command {
                FoldersCommand::Remove { path } => assert_eq!(path, "~/Documents"),
                _ => panic!("expected Folders Remove command"),
            },
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_folders_enable_command() {
        let cli = Cli::try_parse_from(["glintindex", "folders", "enable", "~/Documents"]).unwrap();
        match cli.command {
            Command::Folders(args) => match args.command {
                FoldersCommand::Enable { path } => assert_eq!(path, "~/Documents"),
                _ => panic!("expected Folders Enable command"),
            },
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_folders_disable_command() {
        let cli = Cli::try_parse_from(["glintindex", "folders", "disable", "~/Documents"]).unwrap();
        match cli.command {
            Command::Folders(args) => match args.command {
                FoldersCommand::Disable { path } => assert_eq!(path, "~/Documents"),
                _ => panic!("expected Folders Disable command"),
            },
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_clear_command() {
        let cli = Cli::try_parse_from(["glintindex", "clear"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(!args.yes),
            _ => panic!("expected Clear command"),
        }
    }

    #[test]
    fn parse_clear_yes_flag() {
        let cli = Cli::try_parse_from(["glintindex", "clear", "--yes"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(args.yes),
            _ => panic!("expected Clear command"),
        }
    }

    #[test]
    fn parse_clear_short_yes_flag() {
        let cli = Cli::try_parse_from(["glintindex", "clear", "-y"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(args.yes),
            _ => panic!("expected Clear command"),
        }
    }
}
