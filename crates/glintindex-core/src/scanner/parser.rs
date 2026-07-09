/// Returns `true` if the first few bytes of the buffer look like binary data.
///
/// Heuristic: scans the first 512 bytes for null bytes or a high
/// density of non-printable, non-whitespace bytes. This is fast and
/// avoids reading the entire file.
pub fn is_likely_binary(content: &[u8]) -> bool {
    let sample_len = content.len().min(512);
    let sample = &content[..sample_len];

    let null_count = sample.iter().filter(|&&b| b == 0).count();
    if null_count > 0 {
        return true;
    }

    let non_text = sample
        .iter()
        .filter(|&&b| {
            // Control characters (excluding common whitespace: tab, newline, carriage return)
            (b < 0x20 && b != 0x09 && b != 0x0A && b != 0x0D) || b == 0x7F
        })
        .count();

    // If more than 10% of sampled bytes are control characters, treat as binary
    let threshold = sample_len / 10;
    non_text > threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn read_file_as_string(path: &std::path::Path) -> std::result::Result<String, std::io::Error> {
        let bytes = std::fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes).into_owned();
        Ok(content)
    }

    #[test]
    fn read_valid_utf8() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();
        let content = read_file_as_string(file.path()).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn read_invalid_utf8_lossy() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0xFF, 0xFE, b'h', b'i']).unwrap();
        let content = read_file_as_string(file.path()).unwrap();
        assert!(content.contains("hi"));
    }

    #[test]
    fn binary_detection_null_byte() {
        let data = [0x00, 0x01, 0x02, 0x03];
        assert!(is_likely_binary(&data));
    }

    #[test]
    fn binary_detection_text() {
        let data = b"fn main() {\n    println!(\"hello\");\n}\n";
        assert!(!is_likely_binary(data));
    }

    #[test]
    fn binary_detection_empty() {
        assert!(!is_likely_binary(&[]));
    }

    #[test]
    fn binary_detection_control_chars() {
        let data: Vec<u8> = (0..100).map(|i| (i % 32) as u8).collect();
        assert!(is_likely_binary(&data));
    }
}
