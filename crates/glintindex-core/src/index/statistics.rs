/// Statistics about the current state of the search index.
///
/// Provides an overview of index health and resource usage.
/// Designed for extensibility — additional metrics can be added
/// without breaking the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStatistics {
    /// Total number of documents currently indexed.
    pub indexed_documents: u64,
    /// Total size of the index on disk in bytes.
    pub index_size_bytes: u64,
}

impl IndexStatistics {
    /// Creates a new `IndexStatistics` with the given values.
    pub fn new(indexed_documents: u64, index_size_bytes: u64) -> Self {
        Self {
            indexed_documents,
            index_size_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_statistics() {
        let stats = IndexStatistics::new(42, 1024);
        assert_eq!(stats.indexed_documents, 42);
        assert_eq!(stats.index_size_bytes, 1024);
    }

    #[test]
    fn statistics_equality() {
        let a = IndexStatistics::new(10, 500);
        let b = IndexStatistics::new(10, 500);
        assert_eq!(a, b);
    }
}
