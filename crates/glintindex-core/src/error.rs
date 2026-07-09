/// The project-wide error type for all glintindex-core operations.
///
/// This enum centralizes every error condition that can arise in the core
/// library. Downstream crates should depend on this type rather than
/// inventing their own error variants for core operations.
#[derive(Debug, thiserror::Error)]
pub enum GlintIndexError {
    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A configuration error occurred.
    #[error("configuration error: {0}")]
    Config(String),

    /// The index could not be read or written.
    #[error("index error: {0}")]
    Index(String),

    /// A search operation failed.
    #[error("search error: {0}")]
    Search(String),

    /// A value could not be parsed.
    #[error("parse error: {0}")]
    Parse(String),

    /// An input value was invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// An error that does not fit the other categories.
    #[error("{0}")]
    Other(String),
}

/// A convenience `Result` type alias for glintindex-core.
pub type Result<T> = std::result::Result<T, GlintIndexError>;

impl From<toml::de::Error> for GlintIndexError {
    fn from(err: toml::de::Error) -> Self {
        GlintIndexError::Config(format!("TOML parse error: {err}"))
    }
}

impl From<toml::ser::Error> for GlintIndexError {
    fn from(err: toml::ser::Error) -> Self {
        GlintIndexError::Config(format!("TOML serialize error: {err}"))
    }
}

impl From<tantivy::error::TantivyError> for GlintIndexError {
    fn from(err: tantivy::error::TantivyError) -> Self {
        GlintIndexError::Index(format!("Tantivy error: {err}"))
    }
}

impl From<tantivy::directory::error::OpenDirectoryError> for GlintIndexError {
    fn from(err: tantivy::directory::error::OpenDirectoryError) -> Self {
        GlintIndexError::Index(format!("Tantivy directory error: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let err = GlintIndexError::from(io_err);
        assert!(matches!(err, GlintIndexError::Io(_)));
        assert!(err.to_string().contains("missing file"));
    }

    #[test]
    fn config_error_display() {
        let err = GlintIndexError::Config("bad value".into());
        assert_eq!(err.to_string(), "configuration error: bad value");
    }

    #[test]
    fn index_error_display() {
        let err = GlintIndexError::Index("corrupt".into());
        assert_eq!(err.to_string(), "index error: corrupt");
    }

    #[test]
    fn search_error_display() {
        let err = GlintIndexError::Search("timeout".into());
        assert_eq!(err.to_string(), "search error: timeout");
    }

    #[test]
    fn parse_error_display() {
        let err = GlintIndexError::Parse("invalid regex".into());
        assert_eq!(err.to_string(), "parse error: invalid regex");
    }

    #[test]
    fn invalid_input_display() {
        let err = GlintIndexError::InvalidInput("negative size".into());
        assert_eq!(err.to_string(), "invalid input: negative size");
    }

    #[test]
    fn other_error_display() {
        let err = GlintIndexError::Other("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn tantivy_error_conversion() {
        let tantivy_err = tantivy::error::TantivyError::SchemaError("test".into());
        let err = GlintIndexError::from(tantivy_err);
        assert!(matches!(err, GlintIndexError::Index(_)));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn result_type_alias() {
        let _: Result<()> = Ok(());
        let err: Result<()> = Err(GlintIndexError::Other("test".into()));
        assert!(err.is_err());
    }
}
