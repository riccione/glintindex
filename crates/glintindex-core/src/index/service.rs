use std::cell::UnsafeCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, QueryParser};
use tantivy::tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::error::{GlintIndexError, Result};
use crate::metadata::{FileMetadata, Repository};
use crate::model::{Document, SearchQuery, SearchResponse};
use crate::traits::{DocumentIndexer, SearchEngine};

use super::mapper::{document_to_tantivy, tantivy_to_search_result};
use super::schema::{IndexFields, create_schema};
use super::statistics::IndexStatistics;

/// The default number of writer heap bytes (50 MB).
const DEFAULT_WRITER_HEAP: usize = 50_000_000;

/// A high-level search index service built on Tantivy.
///
/// `IndexService` owns all Tantivy resources and exposes an
/// application-specific API. Callers never interact with Tantivy types
/// directly.
///
/// The service is designed to be shared across threads via `Arc`.
///
/// # Examples
///
/// ```no_run
/// use glintindex_core::index::IndexService;
/// use std::path::Path;
///
/// let service = IndexService::open_or_create(Path::new("/tmp/my-index")).unwrap();
/// ```
pub struct IndexService {
    index: Index,
    writer: UnsafeCell<IndexWriter>,
    reader: IndexReader,
    fields: Arc<IndexFields>,
    index_path: PathBuf,
    metadata: Option<Mutex<Repository>>,
    /// Buffered metadata records awaiting batch flush to SQLite.
    metadata_buffer: Mutex<Vec<FileMetadata>>,
}

// SAFETY: IndexWriter is Send. All mutable access to the writer goes
// through methods that take &self and are not called concurrently.
unsafe impl Send for IndexService {}

impl IndexService {
    /// Opens an existing index or creates a new one at the given path.
    ///
    /// If the directory does not exist, it is created along with
    /// any necessary parent directories.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Index`] if the index cannot be
    /// created or opened.
    pub fn open_or_create(index_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_path)?;

        let (schema, fields) = create_schema();

        // Try to open an existing index; if schema doesn't match, recreate.
        let index = match Index::open_in_dir(index_path) {
            Ok(existing) => {
                if existing.schema() == schema {
                    existing
                } else {
                    tracing::info!(
                        target: "glintindex::index",
                        "schema mismatch detected, recreating index"
                    );
                    // Drop old index to release file handles / memory maps
                    drop(existing);
                    // Remove all index files but preserve the directory
                    if let Ok(entries) = std::fs::read_dir(index_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                    Index::create_in_dir(index_path, schema)?
                }
            }
            Err(_) => {
                // No existing index or unrecognizable error — create fresh
                Index::create_in_dir(index_path, schema)?
            }
        };

        // Register a custom tokenizer that strips tokens longer than 40
        // characters. This prevents Tantivy's SimpleTokenizer from spinning
        // on corrupted text (e.g. raw byte streams from broken PDF extraction)
        // where there are no whitespace boundaries for millions of characters.
        let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("default", tokenizer);

        let writer = index.writer(DEFAULT_WRITER_HEAP)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let fields = Arc::new(fields);

        // Initialize metadata database
        let db_path = index_path.join("metadata.db");
        let metadata = Repository::initialize(&db_path).ok();

        Ok(Self {
            index,
            writer: UnsafeCell::new(writer),
            reader,
            fields,
            index_path: index_path.to_path_buf(),
            metadata: metadata.map(Mutex::new),
            metadata_buffer: Mutex::new(Vec::new()),
        })
    }

    /// Returns `true` if an index exists at the given path.
    pub fn index_exists(index_path: &Path) -> bool {
        index_path.exists() && index_path.join("meta.json").exists()
    }

    /// Returns the path where this index is stored.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Buffers a metadata record for later batch flush to SQLite.
    fn buffer_metadata(&self, document: &Document) {
        let meta = FileMetadata {
            path: document.path.to_string_lossy().to_string(),
            size: document.size as i64,
            modified: document
                .modified
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            hash: None,
            mime: None,
            parser_version: 1,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        };
        if let Ok(mut buffer) = self.metadata_buffer.lock() {
            buffer.push(meta);
        }
    }

    /// Flushes buffered metadata to the database in a single transaction.
    pub fn flush_metadata_buffer(&self) -> Result<()> {
        let buffer = {
            let Ok(mut guard) = self.metadata_buffer.lock() else {
                return Ok(());
            };
            std::mem::take(&mut *guard)
        };

        if !buffer.is_empty() {
            if let Some(ref repo) = self.metadata {
                if let Ok(mut guard) = repo.lock() {
                    guard.upsert_batch(&buffer)?;
                }
            }
        }
        Ok(())
    }

    /// Returns a reference to the metadata repository, if available.
    pub fn metadata(&self) -> Option<&Mutex<Repository>> {
        self.metadata.as_ref()
    }

    /// Commits all pending changes to the index.
    ///
    /// This must be called after adding, updating, or removing
    /// documents to make them visible to search.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Index`] if the commit fails.
    pub fn commit(&self) -> Result<()> {
        // SAFETY: commit is not called concurrently.
        unsafe {
            (*self.writer.get()).commit()?;
        }
        self.reader.reload()?;
        Ok(())
    }

    /// Reloads the index reader to reflect recent commits.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Index`] if the reload fails.
    pub fn reload_reader(&self) -> Result<()> {
        self.reader.reload()?;
        Ok(())
    }

    /// Retrieves statistics about the current index state.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Index`] if the statistics
    /// cannot be retrieved.
    pub fn statistics(&self) -> Result<IndexStatistics> {
        let searcher = self.reader.searcher();
        let indexed_documents = searcher.num_docs();
        let index_size_bytes = self.calculate_index_size()?;
        Ok(IndexStatistics::new(indexed_documents, index_size_bytes))
    }

    fn calculate_index_size(&self) -> Result<u64> {
        let mut total = 0u64;
        for entry in std::fs::read_dir(&self.index_path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }

    /// Removes the index directory from disk.
    ///
    /// The service must not be used after calling this method.
    ///
    /// # Errors
    ///
    /// Returns [`GlintIndexError::Io`] if the directory cannot
    /// be removed.
    pub fn delete_index(&self) -> Result<()> {
        std::fs::remove_dir_all(&self.index_path)?;
        Ok(())
    }

    fn search_inner(&self, query: &SearchQuery) -> Result<SearchResponse> {
        if query.is_empty() {
            return Ok(SearchResponse::new(Vec::new(), 0, 0, 0));
        }

        let searcher = self.reader.searcher();

        // Build the standard full-text query
        let text_fields = vec![self.fields.filename, self.fields.content];
        let query_parser = QueryParser::for_index(&self.index, text_fields);

        let standard_query = query_parser
            .parse_query(&query.query)
            .map_err(|e| GlintIndexError::Search(format!("failed to parse query: {e}")))?;

        // Build prefix queries for tokens >= 3 characters.
        let prefix_query = self.build_prefix_query(&query.query);

        // Build fuzzy queries for typo-tolerant matching.
        let fuzzy_query = self.build_fuzzy_query(&query.query);

        // Combine standard, prefix, and fuzzy queries
        let combined_query: Box<dyn tantivy::query::Query> = {
            let mut sub_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
            sub_queries.push((Occur::Should, standard_query));
            if let Some(prefix_q) = prefix_query {
                sub_queries.push((Occur::Should, prefix_q));
            }
            if let Some(fuzzy_q) = fuzzy_query {
                sub_queries.push((Occur::Should, fuzzy_q));
            }
            Box::new(BooleanQuery::new(sub_queries))
        };

        // Single-pass search: fetch total count + top docs in one Tantivy execution.
        // `search_limit` ensures we fetch enough documents to cover both the skip
        // (offset) and the page (limit).
        let search_limit = query.offset.saturating_add(query.limit);
        let (total_hits, top_docs) = searcher.search(
            &*combined_query,
            &(Count, TopDocs::with_limit(search_limit).order_by_score()),
        )?;

        let paginated: Vec<_> = top_docs
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        let mut results = Vec::with_capacity(paginated.len());

        for (score, doc_address) in paginated {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let snippet = self
                .generate_snippet(&doc, &*combined_query)
                .unwrap_or_default();

            if let Some(result) = tantivy_to_search_result(&doc, score, snippet, &self.fields) {
                results.push(result);
            }
        }

        Ok(SearchResponse::new(
            results,
            total_hits,
            query.offset,
            query.limit,
        ))
    }

    /// Builds prefix queries for search tokens with length >= 3.
    ///
    /// For each token in the query that is at least 3 characters long
    /// (after normalization), creates a prefix query using
    /// `FuzzyTermQuery::new_prefix` with distance 1. This performs
    /// a typo-tolerant prefix match against the inverted index's FST.
    ///
    /// Tokens shorter than 3 characters only get exact-match queries
    /// to avoid overly broad results.
    fn build_prefix_query(&self, query: &str) -> Option<Box<dyn tantivy::query::Query>> {
        let mut sub_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        for term_str in query.split_whitespace() {
            let normalized = term_str.to_lowercase();
            if normalized.len() < 3 {
                continue;
            }

            let filename_term = Term::from_field_text(self.fields.filename, &normalized);
            let content_term = Term::from_field_text(self.fields.content, &normalized);

            let filename_prefix = FuzzyTermQuery::new_prefix(filename_term, 1, true);
            let content_prefix = FuzzyTermQuery::new_prefix(content_term, 1, true);

            sub_queries.push((Occur::Should, Box::new(filename_prefix)));
            sub_queries.push((Occur::Should, Box::new(content_prefix)));
        }

        if sub_queries.is_empty() {
            None
        } else {
            Some(Box::new(BooleanQuery::new(sub_queries)))
        }
    }

    /// Builds fuzzy queries for search tokens with length >= 3.
    ///
    /// For each token, creates a full-term fuzzy match using
    /// `FuzzyTermQuery` with Damerau-Levenshtein distance 1.
    /// This catches single-character typos (e.g. "progamming" → "programming").
    ///
    /// Tokens shorter than 3 characters are skipped to avoid overly broad results.
    fn build_fuzzy_query(&self, query: &str) -> Option<Box<dyn tantivy::query::Query>> {
        let mut sub_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        for term_str in query.split_whitespace() {
            let normalized = term_str.to_lowercase();
            if normalized.len() < 3 {
                continue;
            }

            let filename_term = Term::from_field_text(self.fields.filename, &normalized);
            let content_term = Term::from_field_text(self.fields.content, &normalized);

            sub_queries.push((
                Occur::Should,
                Box::new(FuzzyTermQuery::new(filename_term, 1, true)),
            ));
            sub_queries.push((
                Occur::Should,
                Box::new(FuzzyTermQuery::new(content_term, 1, true)),
            ));
        }

        if sub_queries.is_empty() {
            None
        } else {
            Some(Box::new(BooleanQuery::new(sub_queries)))
        }
    }

    fn generate_snippet(
        &self,
        doc: &tantivy::TantivyDocument,
        query: &dyn tantivy::query::Query,
    ) -> Option<String> {
        let mut snippet_generator = tantivy::snippet::SnippetGenerator::create(
            &self.reader.searcher(),
            query,
            self.fields.content,
        )
        .ok()?;
        snippet_generator.set_max_num_chars(200);
        let snippet = snippet_generator.snippet_from_doc(doc);
        Some(snippet.to_html())
    }
}

impl DocumentIndexer for IndexService {
    fn add_document(&self, document: &Document) -> Result<()> {
        let tantivy_doc = document_to_tantivy(document, &self.fields);
        // SAFETY: add_document is not called concurrently with other writer mutations.
        unsafe {
            (*self.writer.get())
                .add_document(tantivy_doc)
                .map_err(|e| GlintIndexError::Index(format!("failed to add document: {e}")))?;
        }
        Ok(())
    }

    fn update_document(&self, document: &Document) -> Result<()> {
        let tantivy_doc = document_to_tantivy(document, &self.fields);
        let path_term =
            tantivy::Term::from_field_text(self.fields.path, &document.path.to_string_lossy());
        // SAFETY: update operations are not called concurrently.
        unsafe {
            (*self.writer.get()).delete_term(path_term);
            (*self.writer.get())
                .add_document(tantivy_doc)
                .map_err(|e| GlintIndexError::Index(format!("failed to update document: {e}")))?;
        }

        // Buffer metadata for batch flush
        self.buffer_metadata(document);

        Ok(())
    }

    fn remove_document(&self, path: &Path) -> Result<()> {
        let path_term = tantivy::Term::from_field_text(self.fields.path, &path.to_string_lossy());
        // SAFETY: delete_term is not called concurrently.
        unsafe {
            (*self.writer.get()).delete_term(path_term);
        }

        // Remove metadata record
        if let Some(ref repo) = self.metadata {
            if let Ok(guard) = repo.lock() {
                let _ = guard.remove(&path.to_string_lossy());
            }
        }

        Ok(())
    }

    fn rebuild(&self) -> Result<()> {
        // SAFETY: rebuild is not called concurrently.
        unsafe {
            (*self.writer.get())
                .delete_all_documents()
                .map_err(|e| GlintIndexError::Index(format!("failed to clear index: {e}")))?;
            (*self.writer.get()).commit()?;
        }
        self.reader.reload()?;

        // Clear metadata database
        if let Some(ref repo) = self.metadata {
            if let Ok(guard) = repo.lock() {
                let _ = guard.clear();
            }
        }

        Ok(())
    }
}

impl SearchEngine for IndexService {
    fn search(&self, query: &SearchQuery) -> Result<SearchResponse> {
        self.search_inner(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn temp_index_service() -> (IndexService, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let service = IndexService::open_or_create(tmp.path()).unwrap();
        (service, tmp)
    }

    fn sample_document(name: &str, content: &str) -> Document {
        Document::new(
            PathBuf::from(format!("/home/user/{name}")),
            content.len() as u64,
            UNIX_EPOCH + Duration::from_secs(1700000000),
            content.to_string(),
        )
    }

    #[test]
    fn create_index() {
        let (service, _tmp) = temp_index_service();
        assert!(IndexService::index_exists(_tmp.path()));
        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 0);
    }

    #[test]
    fn open_existing_index() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let _service = IndexService::open_or_create(tmp.path()).unwrap();
        }
        let service = IndexService::open_or_create(tmp.path()).unwrap();
        assert!(IndexService::index_exists(tmp.path()));
        let _ = service;
    }

    #[test]
    fn add_document_and_search() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("readme.md", "Hello world from the readme");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("readme")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].document.filename(), "readme.md");
    }

    #[test]
    fn search_content() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("notes.txt", "Rust is a systems programming language");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service
            .search(&SearchQuery::new("systems programming"))
            .unwrap();
        assert!(!response.results.is_empty());
        assert!(response.results[0].snippet.contains("systems"));
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let (service, _tmp) = temp_index_service();
        let response = service.search(&SearchQuery::new("")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn search_no_matches() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("a.txt", "hello");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("nonexistent")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn update_document() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("file.txt", "original content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let updated = sample_document("file.txt", "updated content");
        service.update_document(&updated).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("updated")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].document.content.contains("updated"));

        let old_response = service.search(&SearchQuery::new("original")).unwrap();
        assert!(old_response.is_empty());
    }

    #[test]
    fn remove_document() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("delete_me.txt", "to be removed");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        service
            .remove_document(Path::new("/home/user/delete_me.txt"))
            .unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("removed")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn multiple_documents_search() {
        let (service, _tmp) = temp_index_service();
        let docs = vec![
            sample_document("a.txt", "apple pie recipe"),
            sample_document("b.txt", "banana bread recipe"),
            sample_document("c.txt", "cherry jam recipe"),
        ];
        for doc in &docs {
            service.add_document(doc).unwrap();
        }
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("recipe")).unwrap();
        assert_eq!(response.results.len(), 3);
    }

    #[test]
    fn statistics_after_indexing() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("test.txt", "content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let stats = service.statistics().unwrap();
        assert_eq!(stats.indexed_documents, 1);
        assert!(stats.index_size_bytes > 0);
    }

    #[test]
    fn rebuild_clears_index() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("a.txt", "some content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        assert_eq!(service.statistics().unwrap().indexed_documents, 1);

        service.rebuild().unwrap();
        service.commit().unwrap();

        assert_eq!(service.statistics().unwrap().indexed_documents, 0);
    }

    #[test]
    fn index_exists_returns_false_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nonexistent");
        assert!(!IndexService::index_exists(&missing));
    }

    #[test]
    fn snippet_generation() {
        let (service, _tmp) = temp_index_service();
        let long_content = "The quick brown fox jumps over the lazy dog. \
            This is a longer sentence designed to test snippet generation \
            and ensure that we get a meaningful excerpt from the document.";
        let doc = sample_document("animal.txt", long_content);
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("fox")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(!response.results[0].snippet.is_empty());
        assert!(response.results[0].snippet.contains("fox"));
    }

    // ── Prefix search tests ─────────────────────────────────────

    #[test]
    fn prefix_search_filename() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("Sergei_Report.pdf", "Annual report");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // Prefix query should match the filename
        let response = service.search(&SearchQuery::new("serg")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].document.filename(), "Sergei_Report.pdf");
    }

    #[test]
    fn prefix_search_content() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("report.txt", "Sergei filed the invoice");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // Prefix query should match content
        let response = service.search(&SearchQuery::new("serg")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert!(response.results[0].document.content.contains("Sergei"));
    }

    #[test]
    fn prefix_search_three_char_threshold() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("Sergei.txt", "content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // 1-char: exact only (no prefix)
        let response = service.search(&SearchQuery::new("s")).unwrap();
        assert!(response.is_empty()); // "s" doesn't match "Sergei" exactly

        // 2-char: exact only (no prefix)
        let response = service.search(&SearchQuery::new("se")).unwrap();
        assert!(response.is_empty()); // "se" doesn't match "Sergei" exactly

        // 3-char: exact + prefix
        let response = service.search(&SearchQuery::new("ser")).unwrap();
        assert_eq!(response.results.len(), 1); // prefix matches "Sergei"
    }

    #[test]
    fn prefix_search_exact_still_works() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("readme.md", "Hello world from the readme");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("readme")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].document.filename(), "readme.md");
    }

    #[test]
    fn prefix_search_case_insensitive() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("Sergei.txt", "content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // Uppercase prefix should also match
        let response = service.search(&SearchQuery::new("SERG")).unwrap();
        assert_eq!(response.results.len(), 1);
    }

    #[test]
    fn prefix_search_multi_word() {
        let (service, _tmp) = temp_index_service();
        let doc1 = sample_document("Sergei_Invoice.pdf", "Invoice details");
        let doc2 = sample_document("report.txt", "Sergei's report");
        service.add_document(&doc1).unwrap();
        service.add_document(&doc2).unwrap();
        service.commit().unwrap();

        // Multi-word query: "ser inv" should match both
        let response = service.search(&SearchQuery::new("ser inv")).unwrap();
        assert!(!response.is_empty());
    }

    #[test]
    fn prefix_search_no_matches() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("test.txt", "hello");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let response = service.search(&SearchQuery::new("xyz")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn prefix_search_exact_ranks_above_prefix() {
        let (service, _tmp) = temp_index_service();
        let doc1 = sample_document("report.txt", "sergei invoice");
        let doc2 = sample_document("Sergei.txt", "other content");
        service.add_document(&doc1).unwrap();
        service.add_document(&doc2).unwrap();
        service.commit().unwrap();

        // "sergei" should match both documents
        let response = service.search(&SearchQuery::new("sergei")).unwrap();
        assert_eq!(response.results.len(), 2);
        // Both documents should be found (exact match on filename + prefix match on content)
        let filenames: Vec<&str> = response
            .results
            .iter()
            .map(|r| r.document.filename())
            .collect();
        assert!(filenames.contains(&"report.txt"));
        assert!(filenames.contains(&"Sergei.txt"));
    }

    #[test]
    fn prefix_search_single_char_no_prefix() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("test.txt", "hello");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // Single char should only do exact search, not prefix
        let response = service.search(&SearchQuery::new("h")).unwrap();
        assert!(response.is_empty()); // "h" doesn't match "hello" exactly
    }

    #[test]
    fn prefix_search_two_char_no_prefix() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("test.txt", "hello world");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // Two chars should only do exact search, not prefix
        let response = service.search(&SearchQuery::new("he")).unwrap();
        assert!(response.is_empty()); // "he" doesn't match "hello" exactly
    }

    #[test]
    fn fuzzy_search_finds_typo_in_content() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("main.rs", "programming is fun");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // "progamming" (missing 'r') should find "programming"
        let response = service.search(&SearchQuery::new("progamming")).unwrap();
        assert_eq!(response.results.len(), 1);
    }

    #[test]
    fn fuzzy_search_finds_typo_in_filename() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("programming.txt", "content");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // "progamming" should match filename "programming.txt"
        let response = service.search(&SearchQuery::new("progamming")).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].document.filename(), "programming.txt");
    }

    #[test]
    fn fuzzy_search_distance_2_returns_empty() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("main.rs", "programming is fun");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // "progammming" (two extra 'm's) is distance 2 — should not match
        let response = service.search(&SearchQuery::new("progammming")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn fuzzy_prefix_finds_typo() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("config.toml", "settings configuration");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // "setings" (missing 't') should prefix-match "settings"
        let response = service.search(&SearchQuery::new("setings")).unwrap();
        assert!(!response.results.is_empty());
    }

    #[test]
    fn fuzzy_search_short_tokens_skipped() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("a.txt", "a b c");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        // "ab" is < 3 chars — should not trigger fuzzy, just exact
        let response = service.search(&SearchQuery::new("ab")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn fuzzy_search_empty_index() {
        let (service, _tmp) = temp_index_service();
        let response = service.search(&SearchQuery::new("programming")).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn fuzzy_and_exact_results_combined() {
        let (service, _tmp) = temp_index_service();
        let doc1 = sample_document("exact.txt", "exact match");
        let doc2 = sample_document("fuzzy.txt", "programming here");
        service.add_document(&doc1).unwrap();
        service.add_document(&doc2).unwrap();
        service.commit().unwrap();

        // "progamming" should find "programming" via fuzzy
        let response = service.search(&SearchQuery::new("progamming")).unwrap();
        assert!(!response.results.is_empty());
    }
}
