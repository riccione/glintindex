use crate::index::schema::IndexFields;
use crate::model::{Document, SearchResult};
use tantivy::TantivyDocument;
use tantivy::schema::Value;

/// Converts a domain [`Document`] into a Tantivy document.
///
/// Filename and extension are derived from the document's path
/// rather than stored as separate fields on the domain model.
pub fn document_to_tantivy(doc: &Document, fields: &IndexFields) -> TantivyDocument {
    let mut tantivy_doc = TantivyDocument::default();

    let path_str = doc.path.to_string_lossy().to_string();
    tantivy_doc.add_text(fields.path, &path_str);

    let filename = doc.filename().to_string();
    tantivy_doc.add_text(fields.filename, &filename);

    let extension = doc.extension().unwrap_or("").to_string();
    tantivy_doc.add_text(fields.extension, &extension);

    tantivy_doc.add_text(fields.content, &doc.content);

    let modified_secs = doc
        .modified
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    tantivy_doc.add_u64(fields.modified, modified_secs);

    tantivy_doc.add_u64(fields.size, doc.size);

    tantivy_doc
}

/// Converts a Tantivy document and score into a domain [`SearchResult`].
///
/// Extracts stored fields from the Tantivy document and constructs
/// a fully populated domain model result.
pub fn tantivy_to_search_result(
    doc: &TantivyDocument,
    score: f32,
    snippet: String,
    fields: &IndexFields,
) -> Option<SearchResult> {
    let path_str = doc.get_first(fields.path)?.as_str()?;
    let path = std::path::PathBuf::from(path_str);

    let size = doc.get_first(fields.size)?.as_u64().unwrap_or(0);

    let modified_secs = doc.get_first(fields.modified)?.as_u64().unwrap_or(0);
    let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(modified_secs);

    let content = doc
        .get_first(fields.content)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let document = Document::new(path, size, modified, content);

    Some(SearchResult::new(document, score, snippet))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::schema::create_schema;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn roundtrip_document_conversion() {
        let (_schema, fields) = create_schema();

        let doc = Document::new(
            std::path::PathBuf::from("/home/user/report.pdf"),
            2048,
            UNIX_EPOCH + Duration::from_secs(1700000000),
            "Hello world content".to_string(),
        );

        let tantivy_doc = document_to_tantivy(&doc, &fields);
        let result =
            tantivy_to_search_result(&tantivy_doc, 1.0, "snippet".to_string(), &fields).unwrap();

        assert_eq!(result.document.path, doc.path);
        assert_eq!(result.document.size, doc.size);
        assert_eq!(result.document.content, doc.content);
        assert!((result.score - 1.0).abs() < f32::EPSILON);
        assert_eq!(result.snippet, "snippet");
    }

    #[test]
    fn document_to_tantivy_preserves_all_fields() {
        let (_schema, fields) = create_schema();
        let doc = Document::new(
            std::path::PathBuf::from("/tmp/test.rs"),
            512,
            UNIX_EPOCH + Duration::from_secs(12345),
            "fn main() {}".to_string(),
        );

        let tantivy_doc = document_to_tantivy(&doc, &fields);
        assert_eq!(
            tantivy_doc.get_first(fields.path).and_then(|v| v.as_str()),
            Some("/tmp/test.rs")
        );
        assert_eq!(
            tantivy_doc
                .get_first(fields.filename)
                .and_then(|v| v.as_str()),
            Some("test.rs")
        );
        assert_eq!(
            tantivy_doc
                .get_first(fields.extension)
                .and_then(|v| v.as_str()),
            Some("rs")
        );
        assert_eq!(
            tantivy_doc
                .get_first(fields.content)
                .and_then(|v| v.as_str()),
            Some("fn main() {}")
        );
        assert_eq!(
            tantivy_doc
                .get_first(fields.modified)
                .and_then(|v| v.as_u64()),
            Some(12345)
        );
        assert_eq!(
            tantivy_doc.get_first(fields.size).and_then(|v| v.as_u64()),
            Some(512)
        );
    }
}
