pub mod app;
pub mod config;
pub mod error;
pub mod index;
pub mod logging;
pub mod model;
pub mod scanner;
pub mod traits;
pub mod watcher;

pub use app::{ApplicationService, ApplicationStatistics, WatcherStatus};
pub use config::{AppConfig, Theme};
pub use error::{GlintIndexError, Result};
pub use index::{IndexService, IndexStatistics};
pub use model::{Document, IndexedFolder, SearchQuery, SearchResult};
pub use scanner::{FilesystemScanner, ScannerStatistics};
pub use traits::{DocumentIndexer, DocumentScanner, SearchEngine};
pub use watcher::{FileWatcher, WatchEvent};
