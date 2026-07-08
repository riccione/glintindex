use std::path::PathBuf;

/// Represents a file known to the application.
///
/// A `Document` captures the essential metadata of a file without storing
/// derived values like filename or extension, which are computed on demand
/// from the path.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Document {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub size: u64,
    /// Last modification time of the file.
    pub modified: std::time::SystemTime,
    /// Extracted text content of the file.
    pub content: String,
}

impl Document {
    /// Creates a new document with the given metadata.
    pub fn new(path: PathBuf, size: u64, modified: std::time::SystemTime, content: String) -> Self {
        Self {
            path,
            size,
            modified,
            content,
        }
    }

    /// Returns the filename component of the path.
    ///
    /// If the path does not end with a filename, returns an empty string.
    pub fn filename(&self) -> &str {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or("")
    }

    /// Returns the file extension (without the leading dot).
    ///
    /// Returns `None` if the file has no extension.
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn filename_trailing_slash() {
        let doc = Document::new(PathBuf::from("/home/user/"), 0, UNIX_EPOCH, String::new());
        assert_eq!(doc.filename(), "user");
    }

    #[test]
    fn extension_from_path() {
        let doc = Document::new(
            PathBuf::from("/home/user/notes.md"),
            512,
            UNIX_EPOCH,
            String::new(),
        );
        assert_eq!(doc.extension(), Some("md"));
    }

    #[test]
    fn extension_no_extension() {
        let doc = Document::new(
            PathBuf::from("/home/user/Makefile"),
            256,
            UNIX_EPOCH,
            String::new(),
        );
        assert_eq!(doc.extension(), None);
    }

    #[test]
    fn extension_dotfile() {
        let doc = Document::new(
            PathBuf::from("/home/user/.gitignore"),
            64,
            UNIX_EPOCH,
            String::new(),
        );
        assert_eq!(doc.extension(), None);
    }

    #[test]
    fn roundtrip_serde() {
        let doc = Document::new(
            PathBuf::from("/tmp/test.rs"),
            2048,
            UNIX_EPOCH,
            "fn main() {}".to_string(),
        );
        let json = serde_json::to_string(&doc).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, restored);
    }
}
