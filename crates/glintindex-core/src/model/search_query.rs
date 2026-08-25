/// Represents a user search request.
///
/// The query struct is designed to accommodate future extensions such as
/// regex matching, fuzzy search, filters, sorting, and file type restrictions
/// without requiring a redesign of the core API.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchQuery {
    /// The raw search query string.
    pub query: String,
    /// 0-based offset for pagination (skip this many results).
    pub offset: usize,
    /// Maximum number of results to return per page.
    pub limit: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            offset: 0,
            limit: 20,
        }
    }
}

impl SearchQuery {
    /// Creates a new search query with the given text.
    ///
    /// Uses default pagination: offset 0, limit 20.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            offset: 0,
            limit: 20,
        }
    }

    /// Returns `true` if the query string is empty.
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Sets pagination parameters on this query (builder pattern).
    pub fn with_pagination(mut self, offset: usize, limit: usize) -> Self {
        self.offset = offset;
        self.limit = limit;
        self
    }

    /// Creates a paged query from a 1-based page number.
    ///
    /// `page` is clamped to a minimum of 1. Offset is computed as
    /// `(page - 1) * per_page`.
    pub fn paged(query: impl Into<String>, page: usize, per_page: usize) -> Self {
        let page = page.max(1);
        let offset = (page - 1) * per_page;
        Self {
            query: query.into(),
            offset,
            limit: per_page,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_query() {
        let q = SearchQuery::new("hello world");
        assert_eq!(q.query, "hello world");
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 20);
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
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 20);
    }

    #[test]
    fn with_pagination() {
        let q = SearchQuery::new("test").with_pagination(40, 10);
        assert_eq!(q.offset, 40);
        assert_eq!(q.limit, 10);
    }

    #[test]
    fn paged_first_page() {
        let q = SearchQuery::paged("test", 1, 20);
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 20);
    }

    #[test]
    fn paged_third_page() {
        let q = SearchQuery::paged("test", 3, 20);
        assert_eq!(q.offset, 40);
        assert_eq!(q.limit, 20);
    }

    #[test]
    fn paged_clamps_page_to_one() {
        let q = SearchQuery::paged("test", 0, 20);
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 20);
    }

    #[test]
    fn roundtrip_serde() {
        let q = SearchQuery::new("test query").with_pagination(10, 5);
        let json = serde_json::to_string(&q).unwrap();
        let restored: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, restored);
    }
}
