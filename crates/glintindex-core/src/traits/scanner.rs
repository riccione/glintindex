use crate::error::Result;
use crate::model::Document;
use std::path::Path;

/// A trait for scanning directories to discover documents.
///
/// Implementations of this trait are responsible for walking a directory
/// tree and producing [`Document`] instances for each discovered file.
/// The trait is intentionally decoupled from any specific filesystem
/// walker or file type parser.
pub trait DocumentScanner {
    /// Scans the given directory and returns all discovered documents.
    ///
    /// The implementation decides which files to include based on
    /// its own criteria (file extensions, size limits, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or if a
    /// file's metadata cannot be accessed.
    fn scan(&self, directory: &Path) -> Result<Vec<Document>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    /// A trivial scanner for testing purposes.
    struct DummyScanner;

    impl DocumentScanner for DummyScanner {
        fn scan(&self, _directory: &Path) -> Result<Vec<Document>> {
            Ok(vec![Document::new(
                PathBuf::from("/tmp/file.txt"),
                100,
                UNIX_EPOCH,
                "content".into(),
            )])
        }
    }

    #[test]
    fn dummy_scanner_returns_documents() {
        let scanner = DummyScanner;
        let docs = scanner.scan(Path::new("/tmp")).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].filename(), "file.txt");
    }
}
