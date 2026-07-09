use std::path::PathBuf;

/// Returns the default index storage directory.
///
/// Uses platform-standard locations via the `dirs` crate:
/// - Linux: `~/.local/share/glintindex/index`
/// - macOS: `~/Library/Application Support/glintindex/index`
/// - Windows: `C:\Users\<user>\AppData\Local\glintindex\index`
pub fn default_index_directory() -> PathBuf {
    data_dir().join("index")
}

/// Returns the default set of ignored folder names.
pub fn default_ignored_folders() -> Vec<String> {
    vec![
        ".git".into(),
        ".svn".into(),
        ".hg".into(),
        "node_modules".into(),
        "__pycache__".into(),
        ".DS_Store".into(),
    ]
}

/// Returns the default maximum preview size in characters.
pub fn default_max_preview_size() -> usize {
    200
}

fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("glintindex")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_index_directory_ends_with_index() {
        let dir = default_index_directory();
        assert!(dir.ends_with("index"));
    }

    #[test]
    fn default_index_directory_is_absolute() {
        let dir = default_index_directory();
        assert!(dir.is_absolute(), "expected absolute path, got: {dir:?}");
    }

    #[test]
    fn default_index_directory_contains_glintindex() {
        let dir = default_index_directory();
        let s = dir.to_string_lossy();
        assert!(
            s.contains("glintindex"),
            "expected path to contain 'glintindex', got: {s}"
        );
    }

    #[test]
    fn default_ignored_folders_non_empty() {
        let folders = default_ignored_folders();
        assert!(folders.contains(&".git".to_string()));
        assert!(folders.contains(&"node_modules".to_string()));
    }

    #[test]
    fn default_max_preview_size_is_positive() {
        assert!(default_max_preview_size() > 0);
    }
}
