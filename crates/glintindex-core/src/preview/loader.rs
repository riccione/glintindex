//! File loading for previews with large-file handling.
//!
//! Loads file content with configurable size limits and encoding
//! detection. Large files are truncated to prevent memory issues.

use std::path::Path;

use crate::preview::encoding::{self, Encoding};

/// Default maximum file size for preview (1 MB).
const DEFAULT_MAX_SIZE: u64 = 1024 * 1024;

/// Configuration for file loading.
#[derive(Debug, Clone)]
pub struct LoadConfig {
    /// Maximum file size in bytes. Files larger than this are truncated.
    pub max_size: u64,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
        }
    }
}

/// Result of loading a file for preview.
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// The decoded text content.
    pub content: String,
    /// The detected encoding.
    pub encoding: Encoding,
    /// Whether the file was truncated due to size.
    pub truncated: bool,
    /// The original file size in bytes.
    pub original_size: u64,
    /// Whether the file is binary.
    pub is_binary: bool,
    /// Error message if loading failed.
    pub error: Option<String>,
}

impl LoadResult {
    /// Creates a successful load result.
    pub fn success(content: String, encoding: Encoding, original_size: u64) -> Self {
        Self {
            content,
            encoding,
            truncated: false,
            original_size,
            is_binary: false,
            error: None,
        }
    }

    /// Creates a truncated load result.
    pub fn truncated(content: String, encoding: Encoding, original_size: u64) -> Self {
        Self {
            content,
            encoding,
            truncated: true,
            original_size,
            is_binary: false,
            error: None,
        }
    }

    /// Creates a binary file result.
    pub fn binary(original_size: u64) -> Self {
        Self {
            content: String::new(),
            encoding: Encoding::Utf8,
            truncated: false,
            original_size,
            is_binary: true,
            error: Some("Binary file preview is not available.".to_string()),
        }
    }

    /// Creates an error result.
    pub fn error(message: String, original_size: u64) -> Self {
        Self {
            content: String::new(),
            encoding: Encoding::Utf8,
            truncated: false,
            original_size,
            is_binary: false,
            error: Some(message),
        }
    }
}

/// Loads a file for preview with the given configuration.
///
/// Handles encoding detection, large-file truncation, and binary
/// file detection.
pub fn load_file(path: &Path, config: &LoadConfig) -> LoadResult {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return LoadResult::error(format!("Cannot read file: {}", e), 0);
        }
    };

    let file_size = metadata.len();

    // Check if file is too large
    if file_size > config.max_size {
        match load_truncated(path, config.max_size) {
            Ok(result) => result,
            Err(e) => LoadResult::error(format!("Failed to load file: {}", e), file_size),
        }
    } else {
        match load_complete(path) {
            Ok(result) => result,
            Err(e) => LoadResult::error(format!("Failed to load file: {}", e), file_size),
        }
    }
}

/// Loads the complete file content.
fn load_complete(path: &Path) -> crate::error::Result<LoadResult> {
    let result = encoding::detect_from_file(path)?;
    let file_size = std::fs::metadata(path)?.len();

    if result.is_binary {
        Ok(LoadResult::binary(file_size))
    } else {
        Ok(LoadResult::success(result.text, result.encoding, file_size))
    }
}

/// Loads a truncated portion of the file.
fn load_truncated(path: &Path, max_bytes: u64) -> crate::error::Result<LoadResult> {
    let file_size = std::fs::metadata(path)?.len();

    // Read only the first max_bytes
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; max_bytes as usize];
    use std::io::Read;
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    let result = encoding::detect_and_decode(&buffer);

    if result.is_binary {
        Ok(LoadResult::binary(file_size))
    } else {
        let mut content = result.text;
        content.push_str("\n\nPreview truncated.\nOnly the first 1 MB is displayed.");
        Ok(LoadResult::truncated(content, result.encoding, file_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_text_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello, world!").unwrap();

        let result = load_file(&path, &LoadConfig::default());
        assert!(!result.is_binary);
        assert!(!result.truncated);
        assert_eq!(result.content, "Hello, world!");
    }

    #[test]
    fn load_binary_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.bin");
        std::fs::write(&path, vec![0x00, 0x01, 0x02, 0x03]).unwrap();

        let result = load_file(&path, &LoadConfig::default());
        assert!(result.is_binary);
        assert!(result.error.is_some());
    }

    #[test]
    fn load_large_file_truncated() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("large.txt");
        let content = "x".repeat(2000);
        std::fs::write(&path, &content).unwrap();

        let config = LoadConfig { max_size: 1024 };
        let result = load_file(&path, &config);
        assert!(result.truncated);
        assert!(result.content.contains("Preview truncated"));
    }

    #[test]
    fn load_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/file.txt");
        let result = load_file(&path, &LoadConfig::default());
        assert!(result.error.is_some());
    }

    #[test]
    fn load_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("empty.txt");
        std::fs::write(&path, "").unwrap();

        let result = load_file(&path, &LoadConfig::default());
        assert!(!result.is_binary);
        assert!(result.content.is_empty());
    }

    #[test]
    fn load_config_default() {
        let config = LoadConfig::default();
        assert_eq!(config.max_size, 1024 * 1024);
    }

    #[test]
    fn load_result_success() {
        let result = LoadResult::success("test".to_string(), Encoding::Utf8, 4);
        assert!(!result.is_binary);
        assert!(!result.truncated);
        assert!(result.error.is_none());
    }

    #[test]
    fn load_result_truncated() {
        let result = LoadResult::truncated("test".to_string(), Encoding::Utf8, 1000);
        assert!(result.truncated);
        assert_eq!(result.original_size, 1000);
    }

    #[test]
    fn load_result_binary() {
        let result = LoadResult::binary(512);
        assert!(result.is_binary);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Binary"));
    }

    #[test]
    fn load_result_error() {
        let result = LoadResult::error("test error".to_string(), 0);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "test error");
    }
}
