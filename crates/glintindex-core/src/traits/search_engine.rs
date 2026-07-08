use crate::error::Result;
use crate::model::{SearchQuery, SearchResult};

/// A trait for executing searches against the document index.
///
/// Implementations of this trait translate a [`SearchQuery`] into a
/// ranked list of [`SearchResult`] items. The trait is decoupled from
/// any specific search backend so that the core API remains stable.
pub trait SearchEngine {
    /// Executes a search query and returns matching results.
    ///
    /// Results are returned in descending order of relevance score.
    ///
    /// # Errors
    ///
    /// Returns an error if the search cannot be performed.
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    /// A trivial search engine for testing purposes.
    struct DummySearchEngine;

    impl SearchEngine for DummySearchEngine {
        fn search(&self, _query: &SearchQuery) -> Result<Vec<SearchResult>> {
            let doc = Document::new(
                PathBuf::from("/tmp/result.txt"),
                50,
                UNIX_EPOCH,
                "matched content".into(),
            );
            Ok(vec![SearchResult::new(doc, 1.0, "matched".into())])
        }
    }

    #[test]
    fn dummy_search_returns_results() {
        let engine = DummySearchEngine;
        let query = SearchQuery::new("test");
        let results = engine.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
    }
}
