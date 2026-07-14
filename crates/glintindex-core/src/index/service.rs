use std::cell::UnsafeCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::error::{GlintIndexError, Result};
use crate::metadata::{FileMetadata, Repository};
use crate::model::{Document, SearchQuery, SearchResult};
use crate::traits::{DocumentIndexer, SearchEngine};

use super::mapper::{document_to_tantivy, tantivy_to_search_result};
use super::schema::{IndexFields, create_schema};
use super::statistics::IndexStatistics;

/// The default maximum number of search results returned per query.
const DEFAULT_SEARCH_LIMIT: usize = 20;

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
        let dir = MmapDirectory::open(index_path)?;
        let index = Index::open_or_create(dir, schema)?;

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

    fn search_inner(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();

        let text_fields = vec![self.fields.filename, self.fields.content];
        let query_parser = QueryParser::for_index(&self.index, text_fields);

        let tantivy_query = query_parser
            .parse_query(&query.query)
            .map_err(|e| GlintIndexError::Search(format!("failed to parse query: {e}")))?;

        let collector = TopDocs::with_limit(DEFAULT_SEARCH_LIMIT).order_by_score();
        let top_docs = searcher.search(&tantivy_query, &collector)?;

        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let snippet = self
                .generate_snippet(&doc, &tantivy_query)
                .unwrap_or_default();

            if let Some(result) = tantivy_to_search_result(&doc, score, snippet, &self.fields) {
                results.push(result);
            }
        }

        Ok(results)
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

        // Update metadata record
        if let Some(ref repo) = self.metadata {
            let meta = FileMetadata {
                path: document.path.to_string_lossy().to_string(),
                size: document.size,
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
            if let Ok(guard) = repo.lock() {
                let _ = guard.upsert(&meta);
            }
        }

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
    fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
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

        let results = service.search(&SearchQuery::new("readme")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.filename(), "readme.md");
    }

    #[test]
    fn search_content() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("notes.txt", "Rust is a systems programming language");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let results = service
            .search(&SearchQuery::new("systems programming"))
            .unwrap();
        assert!(!results.is_empty());
        assert!(results[0].snippet.contains("systems"));
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let (service, _tmp) = temp_index_service();
        let results = service.search(&SearchQuery::new("")).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_matches() {
        let (service, _tmp) = temp_index_service();
        let doc = sample_document("a.txt", "hello");
        service.add_document(&doc).unwrap();
        service.commit().unwrap();

        let results = service.search(&SearchQuery::new("nonexistent")).unwrap();
        assert!(results.is_empty());
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

        let results = service.search(&SearchQuery::new("updated")).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].document.content.contains("updated"));

        let old_results = service.search(&SearchQuery::new("original")).unwrap();
        assert!(old_results.is_empty());
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

        let results = service.search(&SearchQuery::new("removed")).unwrap();
        assert!(results.is_empty());
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

        let results = service.search(&SearchQuery::new("recipe")).unwrap();
        assert_eq!(results.len(), 3);
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

        let results = service.search(&SearchQuery::new("fox")).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].snippet.is_empty());
        assert!(results[0].snippet.contains("fox"));
    }
}
