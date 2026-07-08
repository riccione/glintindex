use crate::error::Result;
use crate::model::Document;

/// A trait for managing the document index.
///
/// Implementations of this trait handle the storage and retrieval of
/// documents in an index. The trait is decoupled from any specific
/// indexing technology (such as Tantivy) so that the core API remains
/// stable across different backend implementations.
pub trait DocumentIndexer {
    /// Adds a new document to the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be added.
    fn add_document(&self, document: &Document) -> Result<()>;

    /// Updates an existing document in the index.
    ///
    /// The document is matched by its path. If the document does not
    /// exist in the index, this method should return an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be found or updated.
    fn update_document(&self, document: &Document) -> Result<()>;

    /// Removes a document from the index by path.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be found or removed.
    fn remove_document(&self, path: &std::path::Path) -> Result<()>;

    /// Rebuilds the entire index from scratch.
    ///
    /// This is a potentially expensive operation that should be used
    /// when the index is corrupted or needs to be fully refreshed.
    ///
    /// # Errors
    ///
    /// Returns an error if the rebuild process fails.
    fn rebuild(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::UNIX_EPOCH;

    /// A trivial indexer for testing purposes.
    struct DummyIndexer;

    impl DocumentIndexer for DummyIndexer {
        fn add_document(&self, _document: &Document) -> Result<()> {
            Ok(())
        }

        fn update_document(&self, _document: &Document) -> Result<()> {
            Ok(())
        }

        fn remove_document(&self, _path: &Path) -> Result<()> {
            Ok(())
        }

        fn rebuild(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn dummy_indexer_add_document() {
        let indexer = DummyIndexer;
        let doc = Document::new(
            PathBuf::from("/tmp/test.txt"),
            100,
            UNIX_EPOCH,
            "content".into(),
        );
        assert!(indexer.add_document(&doc).is_ok());
    }

    #[test]
    fn dummy_indexer_remove_document() {
        let indexer = DummyIndexer;
        assert!(indexer.remove_document(Path::new("/tmp/test.txt")).is_ok());
    }

    #[test]
    fn dummy_indexer_rebuild() {
        let indexer = DummyIndexer;
        assert!(indexer.rebuild().is_ok());
    }
}
