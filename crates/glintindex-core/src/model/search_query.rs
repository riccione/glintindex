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
    /// Whether the search tolerates single-character typos.
    ///
    /// When `false`, the index still matches partial words (starts-with)
    /// against exact terms but never supplements with distance-1
    /// neighbors. Defaults to `true` (also for payloads that predate
    /// the field).
    #[serde(default = "default_fuzzy")]
    pub fuzzy: bool,
}

/// Returns the default fuzzy preference for deserialization.
fn default_fuzzy() -> bool {
    true
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            offset: 0,
            limit: 20,
            fuzzy: default_fuzzy(),
        }
    }
}

impl SearchQuery {
    /// Creates a new search query with the given text.
    ///
    /// Uses default pagination: offset 0, limit 20, fuzzy matching on.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            offset: 0,
            limit: 20,
            fuzzy: default_fuzzy(),
        }
    }

    /// Returns `true` if the query string is empty.
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Sets pagination parameters on this query (builder pattern).
    ///
    /// `limit` is clamped to a minimum of 1 — a zero limit would
    /// panic downstream in tantivy's `TopDocs::with_limit`.
    pub fn with_pagination(mut self, offset: usize, limit: usize) -> Self {
        self.offset = offset;
        self.limit = limit.max(1);
        self
    }

    /// Creates a paged query from a 1-based page number.
    ///
    /// `page` is clamped to a minimum of 1, `per_page` to a minimum
    /// of 1. Offset is computed as `(page - 1) * per_page`.
    pub fn paged(query: impl Into<String>, page: usize, per_page: usize) -> Self {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = (page - 1) * per_page;
        Self {
            query: query.into(),
            offset,
            limit: per_page,
            fuzzy: default_fuzzy(),
        }
    }

    /// Sets whether the search tolerates single-character typos.
    pub fn with_fuzzy(mut self, fuzzy: bool) -> Self {
        self.fuzzy = fuzzy;
        self
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
    fn paged_clamps_per_page_to_one() {
        let q = SearchQuery::paged("test", 1, 0);
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 1);
    }

    #[test]
    fn paged_zero_per_page_still_offsets() {
        let q = SearchQuery::paged("test", 3, 0);
        assert_eq!(q.offset, 2, "offset uses the clamped per_page of 1");
        assert_eq!(q.limit, 1);
    }

    #[test]
    fn with_pagination_clamps_limit_to_one() {
        let q = SearchQuery::new("test").with_pagination(40, 0);
        assert_eq!(q.offset, 40);
        assert_eq!(q.limit, 1);
    }

    #[test]
    fn roundtrip_serde() {
        let q = SearchQuery::new("test query").with_pagination(10, 5);
        let json = serde_json::to_string(&q).unwrap();
        let restored: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(q, restored);
    }

    #[test]
    fn fuzzy_defaults_to_true() {
        assert!(SearchQuery::new("hello").fuzzy);
        assert!(SearchQuery::default().fuzzy);
        assert!(SearchQuery::paged("hello", 1, 20).fuzzy);
    }

    #[test]
    fn with_fuzzy_disables_typo_tolerance() {
        let q = SearchQuery::new("hello").with_fuzzy(false);
        assert!(!q.fuzzy);
        assert_eq!(q.query, "hello");
    }

    #[test]
    fn fuzzy_missing_in_payload_defaults_true() {
        let restored: SearchQuery =
            serde_json::from_str(r#"{"query":"x","offset":0,"limit":20}"#).unwrap();
        assert!(restored.fuzzy, "payloads predating the field stay fuzzy");
    }
}
