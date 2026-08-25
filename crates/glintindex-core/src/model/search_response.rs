use crate::model::SearchResult;

/// A paginated search response that encapsulates results along with
/// pagination metadata. This is the return type for all search operations.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchResponse {
    /// The search results for the current page.
    pub results: Vec<SearchResult>,
    /// Total number of matching documents across all pages.
    pub total: usize,
    /// The 0-based offset of the first result in this page.
    pub offset: usize,
    /// The maximum number of results requested per page.
    pub limit: usize,
}

impl Default for SearchResponse {
    fn default() -> Self {
        Self::new(Vec::new(), 0, 0, 20)
    }
}

impl SearchResponse {
    /// Creates a new search response.
    pub fn new(results: Vec<SearchResult>, total: usize, offset: usize, limit: usize) -> Self {
        Self {
            results,
            total,
            offset,
            limit,
        }
    }

    /// Calculates the current 1-based page number.
    ///
    /// Returns `1` if `limit` is zero to avoid division by zero.
    pub fn current_page(&self) -> usize {
        if self.limit == 0 {
            return 1;
        }
        (self.offset / self.limit) + 1
    }

    /// Calculates total number of pages based on total count and page limit.
    ///
    /// Returns `1` if `limit` is zero or `total` is zero.
    pub fn total_pages(&self) -> usize {
        if self.limit == 0 || self.total == 0 {
            return 1;
        }
        self.total.div_ceil(self.limit)
    }

    /// Returns `true` if the result set is empty.
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Document, SearchResult};
    use std::path::PathBuf;
    use std::time::UNIX_EPOCH;

    fn sample_result(score: f32) -> SearchResult {
        SearchResult::new(
            Document::new(
                PathBuf::from("/tmp/test.txt"),
                100,
                UNIX_EPOCH,
                "content".into(),
            ),
            score,
            "snippet".into(),
        )
    }

    #[test]
    fn new_response() {
        let results = vec![sample_result(1.0), sample_result(0.5)];
        let resp = SearchResponse::new(results.clone(), 10, 0, 20);
        assert_eq!(resp.results, results);
        assert_eq!(resp.total, 10);
        assert_eq!(resp.offset, 0);
        assert_eq!(resp.limit, 20);
    }

    #[test]
    fn current_page_first_page() {
        let resp = SearchResponse::new(vec![], 100, 0, 20);
        assert_eq!(resp.current_page(), 1);
    }

    #[test]
    fn current_page_second_page() {
        let resp = SearchResponse::new(vec![], 100, 20, 20);
        assert_eq!(resp.current_page(), 2);
    }

    #[test]
    fn current_page_zero_limit() {
        let resp = SearchResponse::new(vec![], 100, 0, 0);
        assert_eq!(resp.current_page(), 1);
    }

    #[test]
    fn total_pages_exact_division() {
        let resp = SearchResponse::new(vec![], 40, 0, 20);
        assert_eq!(resp.total_pages(), 2);
    }

    #[test]
    fn total_pages_with_remainder() {
        let resp = SearchResponse::new(vec![], 45, 0, 20);
        assert_eq!(resp.total_pages(), 3);
    }

    #[test]
    fn total_pages_zero_total() {
        let resp = SearchResponse::new(vec![], 0, 0, 20);
        assert_eq!(resp.total_pages(), 1);
    }

    #[test]
    fn total_pages_zero_limit() {
        let resp = SearchResponse::new(vec![], 100, 0, 0);
        assert_eq!(resp.total_pages(), 1);
    }

    #[test]
    fn is_empty_true() {
        let resp = SearchResponse::new(vec![], 0, 0, 20);
        assert!(resp.is_empty());
    }

    #[test]
    fn is_empty_false() {
        let resp = SearchResponse::new(vec![sample_result(1.0)], 1, 0, 20);
        assert!(!resp.is_empty());
    }

    #[test]
    fn roundtrip_serde() {
        let resp = SearchResponse::new(vec![sample_result(0.9)], 5, 0, 10);
        let json = serde_json::to_string(&resp).unwrap();
        let restored: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, restored);
    }
}
