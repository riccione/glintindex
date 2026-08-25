use clap::Parser as _;
use glintindex_core::AppPaths;
use glintindex_core::config::loader;
use glintindex_core::logging::{LoggingConfig, init as init_logging};

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

    let config_path = match &cli.config {
        Some(path) => path.clone(),
        None => AppPaths::new().config_file().to_string_lossy().into_owned(),
    };

    // Load config to get logging settings (fallback to defaults on error)
    let config = loader::load(std::path::Path::new(&config_path)).unwrap_or_default();

    // Resolution order:
    // 1. RUST_LOG env var (handled by EnvFilter::try_from_default_env)
    // 2. --verbose flag → "debug"
    // 3. config.toml logging.level
    // 4. hardcoded "error"
    let log_level = if cli.verbose {
        "debug".to_string()
    } else {
        config.logging.level.clone()
    };

    let log_to_stderr = cli.verbose || std::env::var("RUST_LOG").is_ok();
    init_logging(LoggingConfig {
        default_level: log_level,
        log_to_stderr,
        log_to_file: true,
        max_retention_days: config.logging.max_retention_days,
    });

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
            Command::Search(args) => {
                assert_eq!(args.query, "invoice");
                assert_eq!(args.page, 1);
                assert!(args.limit.is_none());
            }
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn parse_search_with_page_and_limit() {
        let cli = Cli::try_parse_from([
            "glintindex-cli",
            "search",
            "rust",
            "--page",
            "2",
            "--limit",
            "10",
        ])
        .unwrap();
        match cli.command {
            Command::Search(args) => {
                assert_eq!(args.query, "rust");
                assert_eq!(args.page, 2);
                assert_eq!(args.limit, Some(10));
            }
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn parse_search_with_short_flags() {
        let cli = Cli::try_parse_from(["glintindex-cli", "search", "test", "-p", "3", "-l", "5"])
            .unwrap();
        match cli.command {
            Command::Search(args) => {
                assert_eq!(args.query, "test");
                assert_eq!(args.page, 3);
                assert_eq!(args.limit, Some(5));
            }
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
