use tantivy::schema::{STORED, STRING, Schema, SchemaBuilder, TEXT};

/// Field handles for the Tantivy schema.
///
/// These handles provide typed access to schema fields without
/// exposing raw field IDs to callers.
pub struct IndexFields {
    pub path: tantivy::schema::Field,
    pub filename: tantivy::schema::Field,
    pub extension: tantivy::schema::Field,
    pub content: tantivy::schema::Field,
    pub modified: tantivy::schema::Field,
    pub size: tantivy::schema::Field,
}

/// Creates the Tantivy schema used by GlintIndex.
///
/// Returns both the schema and a set of field handles for
/// convenient field access during indexing and querying.
pub fn create_schema() -> (Schema, IndexFields) {
    let mut builder = SchemaBuilder::new();

    let path = builder.add_text_field("path", STRING | STORED);
    let filename = builder.add_text_field("filename", TEXT | STORED);
    let extension = builder.add_text_field("extension", STRING | STORED);
    let content = builder.add_text_field("content", TEXT | STORED);
    let modified = builder.add_u64_field("modified", STORED);
    let size = builder.add_u64_field("size", STORED);

    let fields = IndexFields {
        path,
        filename,
        extension,
        content,
        modified,
        size,
    };

    (builder.build(), fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_all_fields() {
        let (schema, _fields) = create_schema();
        assert!(schema.get_field("path").is_ok());
        assert!(schema.get_field("filename").is_ok());
        assert!(schema.get_field("extension").is_ok());
        assert!(schema.get_field("content").is_ok());
        assert!(schema.get_field("modified").is_ok());
        assert!(schema.get_field("size").is_ok());
    }
}
