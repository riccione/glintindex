pub mod config;
pub mod error;
pub mod logging;
pub mod model;
pub mod traits;

pub use config::{AppConfig, Theme};
pub use error::{GlintIndexError, Result};
pub use model::{Document, IndexedFolder, SearchQuery, SearchResult};
pub use traits::{DocumentIndexer, DocumentScanner, SearchEngine};
