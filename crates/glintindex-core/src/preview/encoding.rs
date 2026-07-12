//! Encoding detection for file previews.
//!
//! Automatically detects common text encodings including UTF-8,
//! UTF-16 LE/BE, and UTF-8 with BOM. Falls back to raw bytes
//! if decoding fails.

use std::path::Path;

/// Supported text encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 encoding (most common).
    Utf8,
    /// UTF-8 with Byte Order Mark.
    Utf8Bom,
    /// UTF-16 Little Endian.
    Utf16Le,
    /// UTF-16 Big Endian.
    Utf16Be,
}

/// Result of encoding detection.
#[derive(Debug, Clone)]
pub struct EncodingResult {
    /// The detected encoding.
    pub encoding: Encoding,
    /// The decoded text content.
    pub text: String,
    /// Whether binary content was detected.
    pub is_binary: bool,
}

/// Detects the encoding of raw bytes and decodes them to text.
///
/// Checks for BOM markers first, then attempts UTF-8 validation,
/// and falls back to UTF-16 detection. Returns a result indicating
/// whether the content is text or binary.
pub fn detect_and_decode(bytes: &[u8]) -> EncodingResult {
    if bytes.is_empty() {
        return EncodingResult {
            encoding: Encoding::Utf8,
            text: String::new(),
            is_binary: false,
        };
    }

    // Check for BOM markers first (before binary detection)
    // UTF-16 LE BOM (FF FE)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let text = decode_utf16_le(&bytes[2..]);
        return EncodingResult {
            encoding: Encoding::Utf16Le,
            text,
            is_binary: false,
        };
    }

    // UTF-16 BE BOM (FE FF)
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let text = decode_utf16_be(&bytes[2..]);
        return EncodingResult {
            encoding: Encoding::Utf16Be,
            text,
            is_binary: false,
        };
    }

    // UTF-8 BOM (EF BB BF)
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        let text = String::from_utf8_lossy(&bytes[3..]).to_string();
        return EncodingResult {
            encoding: Encoding::Utf8Bom,
            text,
            is_binary: false,
        };
    }

    // Check for binary content (null bytes in first 8KB)
    let check_len = bytes.len().min(8192);
    if bytes[..check_len].contains(&0) {
        return EncodingResult {
            encoding: Encoding::Utf8,
            text: String::new(),
            is_binary: true,
        };
    }

    // Try UTF-8
    if let Ok(text) = std::str::from_utf8(bytes) {
        return EncodingResult {
            encoding: Encoding::Utf8,
            text: text.to_string(),
            is_binary: false,
        };
    }

    // Try UTF-16 LE without BOM (common in Windows)
    if bytes.len() >= 2 && bytes[1] == 0 && is_likely_utf16_le(bytes) {
        let text = decode_utf16_le(bytes);
        return EncodingResult {
            encoding: Encoding::Utf16Le,
            text,
            is_binary: false,
        };
    }

    // Try UTF-16 BE without BOM
    if bytes.len() >= 2 && bytes[0] == 0 && is_likely_utf16_be(bytes) {
        let text = decode_utf16_be(bytes);
        return EncodingResult {
            encoding: Encoding::Utf16Be,
            text,
            is_binary: false,
        };
    }

    // Fall back to lossy UTF-8
    EncodingResult {
        encoding: Encoding::Utf8,
        text: String::from_utf8_lossy(bytes).to_string(),
        is_binary: false,
    }
}

/// Detects encoding from a file path by reading its contents.
///
/// Returns an error if the file cannot be read.
pub fn detect_from_file(path: &Path) -> crate::error::Result<EncodingResult> {
    let bytes = std::fs::read(path)?;
    Ok(detect_and_decode(&bytes))
}

/// Decodes UTF-16 Little Endian bytes to a String.
fn decode_utf16_le(bytes: &[u8]) -> String {
    let chunks = bytes.chunks_exact(2);
    let decoded: Vec<u16> = chunks.map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&decoded)
}

/// Decodes UTF-16 Big Endian bytes to a String.
fn decode_utf16_be(bytes: &[u8]) -> String {
    let chunks = bytes.chunks_exact(2);
    let decoded: Vec<u16> = chunks.map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&decoded)
}

/// Heuristic check for UTF-16 LE content without BOM.
fn is_likely_utf16_le(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(1024);
    let mut null_count = 0;
    for chunk in bytes[..sample_len].chunks_exact(2) {
        if chunk[1] == 0 && chunk[0] != 0 {
            null_count += 1;
        }
    }
    null_count > sample_len / 8
}

/// Heuristic check for UTF-16 BE content without BOM.
fn is_likely_utf16_be(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(1024);
    let mut null_count = 0;
    for chunk in bytes[..sample_len].chunks_exact(2) {
        if chunk[0] == 0 && chunk[1] != 0 {
            null_count += 1;
        }
    }
    null_count > sample_len / 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_utf8() {
        let text = "Hello, world!";
        let result = detect_and_decode(text.as_bytes());
        assert_eq!(result.encoding, Encoding::Utf8);
        assert_eq!(result.text, text);
        assert!(!result.is_binary);
    }

    #[test]
    fn detect_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("Hello".as_bytes());
        let result = detect_and_decode(&bytes);
        assert_eq!(result.encoding, Encoding::Utf8Bom);
        assert_eq!(result.text, "Hello");
    }

    #[test]
    fn detect_utf16_le_bom() {
        let text = "Hello";
        let mut bytes = vec![0xFF, 0xFE];
        for c in text.encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        let result = detect_and_decode(&bytes);
        assert_eq!(result.encoding, Encoding::Utf16Le);
        assert_eq!(result.text, text);
    }

    #[test]
    fn detect_utf16_be_bom() {
        let text = "Hello";
        let mut bytes = vec![0xFE, 0xFF];
        for c in text.encode_utf16() {
            bytes.extend_from_slice(&c.to_be_bytes());
        }
        let result = detect_and_decode(&bytes);
        assert_eq!(result.encoding, Encoding::Utf16Be);
        assert_eq!(result.text, text);
    }

    #[test]
    fn detect_binary() {
        let bytes = vec![0x00, 0x01, 0x02, 0x03];
        let result = detect_and_decode(&bytes);
        assert!(result.is_binary);
    }

    #[test]
    fn detect_empty() {
        let result = detect_and_decode(&[]);
        assert_eq!(result.encoding, Encoding::Utf8);
        assert!(result.text.is_empty());
        assert!(!result.is_binary);
    }

    #[test]
    fn detect_file_utf8() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.txt");
        std::fs::write(&path, "Hello, world!").unwrap();
        let result = detect_from_file(&path).unwrap();
        assert_eq!(result.encoding, Encoding::Utf8);
        assert_eq!(result.text, "Hello, world!");
    }

    #[test]
    fn detect_file_not_found() {
        let path = Path::new("/nonexistent/file.txt");
        assert!(detect_from_file(path).is_err());
    }

    #[test]
    fn utf16_le_heuristic() {
        // Simulate UTF-16 LE text: "Hi" = 0x48 0x00 0x69 0x00
        let bytes = vec![0x48, 0x00, 0x69, 0x00];
        assert!(is_likely_utf16_le(&bytes));
    }

    #[test]
    fn utf16_be_heuristic() {
        // Simulate UTF-16 BE text: "Hi" = 0x00 0x48 0x00 0x69
        let bytes = vec![0x00, 0x48, 0x00, 0x69];
        assert!(is_likely_utf16_be(&bytes));
    }
}
