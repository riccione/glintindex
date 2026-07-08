use crate::model::Document;

/// Represents a single search result returned by the search engine.
///
/// Each result pairs a matching document with a relevance score and an
/// optional text snippet for preview. Future extensions may add highlight
/// positions, ranking metadata, and preview information.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SearchResult {
    /// The matching document.
    pub document: Document,
    /// Relevance score (higher is more relevant).
    pub score: f32,
    /// A short text excerpt from the document for preview purposes.
    pub snippet: String,
}

impl SearchResult {
    /// Creates a new search result.
    pub fn new(document: Document, score: f32, snippet: String) -> Self {
        Self {
            document,
            score,
            snippet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    fn sample_document() -> Document {
        Document::new(
            PathBuf::from("/tmp/test.txt"),
            100,
            UNIX_EPOCH,
            "hello world".to_string(),
        )
    }

    #[test]
    fn new_result() {
        let doc = sample_document();
        let result = SearchResult::new(doc.clone(), 0.95, "hello".to_string());
        assert_eq!(result.document, doc);
        assert!((result.score - 0.95).abs() < f32::EPSILON);
        assert_eq!(result.snippet, "hello");
    }

    #[test]
    fn roundtrip_serde() {
        let result = SearchResult::new(sample_document(), 0.8, "snippet".to_string());
        let json = serde_json::to_string(&result).unwrap();
        let restored: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.document, restored.document);
        assert!((result.score - restored.score).abs() < f32::EPSILON);
    }
}
