/// Represents a user search request.
///
/// The query struct is designed to accommodate future extensions such as
/// regex matching, fuzzy search, filters, sorting, and file type restrictions
/// without requiring a redesign of the core API.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    /// The raw search query string.
    pub query: String,
}

impl SearchQuery {
    /// Creates a new search query with the given text.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
        }
    }

    /// Returns `true` if the query string is empty.
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_query() {
        let q = SearchQuery::new("hello world");
        assert_eq!(q.query, "hello world");
        assert!(!q.is_empty());
    }

    #[test]
    fn empty_query() {
        let q = SearchQuery::new("");
        assert!(q.is_empty());
    }

    #[test]
    fn default_is_empty() {
        let q = SearchQuery::default();
        assert!(q.is_empty());
        assert_eq!(q.query, "");
    }

    #[test]
    fn roundtrip_serde() {
        let q = SearchQuery::new("test query");
        let json = serde_json::to_string(&q).unwrap();
        let restored: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, restored);
    }
}
