use clap::Parser as _;
use glintindex_core::logging::{LoggingConfig, init as init_logging};
use glintindex_core::AppPaths;

use crate::commands::{self, Command};

/// GlintIndex - Local desktop search engine
#[derive(clap::Parser)]
#[command(
    name = "glintindex-cli",
    version,
    about = "Local desktop search engine for indexing and searching files"
)]
pub struct Cli {
    /// Enable verbose logging output to stderr (logs are always written to file)
    #[arg(short, long)]
    pub verbose: bool,

    /// Path to the configuration file (default: platform-specific location)
    #[arg(short, long)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize structured logging with file output
    // The CLI always logs to file; stderr is enabled with --verbose or RUST_LOG
    let log_to_stderr = cli.verbose || std::env::var("RUST_LOG").is_ok();
    init_logging(LoggingConfig {
        default_level: if cli.verbose { "debug" } else { "info" }.to_string(),
        log_to_stderr,
        log_to_file: true,
    });

    let config_path = match &cli.config {
        Some(path) => path.clone(),
        None => AppPaths::new().config_file().to_string_lossy().into_owned(),
    };

    match cli.command {
        Command::Init => commands::init::execute(&config_path),
        Command::Index(args) => commands::index::execute(&config_path, args),
        Command::Search(args) => commands::search::execute(&config_path, args),
        Command::Stats => commands::stats::execute(&config_path),
        Command::Rebuild => commands::rebuild::execute(&config_path),
        Command::Config => commands::config::execute(&config_path),
        Command::Folders(args) => commands::folders::execute(&config_path, args.command),
        Command::Clear(args) => commands::clear::execute(&config_path, args),
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
        let cli = Cli::try_parse_from(["glintindex-cli", "--verbose", "stats"]).unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.command, Command::Stats));
    }

    #[test]
    fn parse_config_flag() {
        let cli =
            Cli::try_parse_from(["glintindex-cli", "--config", "/tmp/test.toml", "stats"]).unwrap();
        assert_eq!(cli.config.as_deref(), Some("/tmp/test.toml"));
    }

    #[test]
    fn parse_search_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "search", "invoice"]).unwrap();
        match cli.command {
            Command::Search(args) => assert_eq!(args.query, "invoice"),
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn parse_index_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "index"]).unwrap();
        assert!(matches!(cli.command, Command::Index(_)));
    }

    #[test]
    fn parse_stats_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "stats"]).unwrap();
        assert!(matches!(cli.command, Command::Stats));
    }

    #[test]
    fn parse_rebuild_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "rebuild"]).unwrap();
        assert!(matches!(cli.command, Command::Rebuild));
    }

    #[test]
    fn parse_config_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "config"]).unwrap();
        assert!(matches!(cli.command, Command::Config));
    }

    #[test]
    fn parse_init_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "init"]).unwrap();
        assert!(matches!(cli.command, Command::Init));
    }

    #[test]
    fn parse_folders_list_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "folders", "list"]).unwrap();
        match cli.command {
            Command::Folders(args) => assert!(matches!(args.command, FoldersCommand::List)),
            _ => panic!("expected Folders command"),
        }
    }

    #[test]
    fn parse_folders_add_command() {
        let cli = Cli::try_parse_from(["glintindex-cli", "folders", "add", "~/Documents"]).unwrap();
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
        let cli =
            Cli::try_parse_from(["glintindex-cli", "folders", "remove", "~/Documents"]).unwrap();
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
        let cli =
            Cli::try_parse_from(["glintindex-cli", "folders", "enable", "~/Documents"]).unwrap();
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
        let cli =
            Cli::try_parse_from(["glintindex-cli", "folders", "disable", "~/Documents"]).unwrap();
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
        let cli = Cli::try_parse_from(["glintindex-cli", "clear"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(!args.yes),
            _ => panic!("expected Clear command"),
        }
    }

    #[test]
    fn parse_clear_yes_flag() {
        let cli = Cli::try_parse_from(["glintindex-cli", "clear", "--yes"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(args.yes),
            _ => panic!("expected Clear command"),
        }
    }

    #[test]
    fn parse_clear_short_yes_flag() {
        let cli = Cli::try_parse_from(["glintindex-cli", "clear", "-y"]).unwrap();
        match cli.command {
            Command::Clear(args) => assert!(args.yes),
            _ => panic!("expected Clear command"),
        }
    }
}
