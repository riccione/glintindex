pub mod app;
pub mod config;
pub mod error;
pub mod index;
pub mod logging;
pub mod metadata;
pub mod model;
pub mod parser;
pub mod preview;
pub mod scanner;
pub mod tasks;
pub mod traits;
pub mod watcher;

pub use app::{ApplicationService, ApplicationStatistics, WatcherStatus};
pub use config::{AppConfig, AppPaths, Theme};
pub use error::{GlintIndexError, Result};
pub use index::{IndexService, IndexStatistics};
pub use model::{Document, IndexedFolder, SearchQuery, SearchResult};
pub use parser::{DocumentMetadata, DocumentParser, ParseResult, ParserRegistry, PlainTextParser};
pub use preview::{
    Encoding, EncodingResult, HighlightedMatch, LoadConfig, LoadResult, PreviewConfig, PreviewLine,
    PreviewOutput, PreviewService, Style, SyntaxHighlighter,
};
pub use scanner::{FilesystemScanner, NoopReporter, ProgressReporter, ScannerStatistics};
pub use tasks::{JobId, JobState, JobStatus, JobType, Progress, TaskManager};
pub use traits::{DocumentIndexer, DocumentScanner, SearchEngine};
pub use watcher::{FileWatcher, WatchEvent};
