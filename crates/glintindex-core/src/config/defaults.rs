use std::path::PathBuf;

use super::paths::AppPaths;

/// Returns the default index storage directory.
///
/// Delegates to [`AppPaths`] for platform-correct resolution:
/// - Linux: `~/.local/share/glintindex/index`
/// - macOS: `~/Library/Application Support/GlintIndex/index`
/// - Windows: `%LOCALAPPDATA%/GlintIndex/index`
pub fn default_index_directory() -> PathBuf {
    AppPaths::new().index_dir()
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
            s.to_lowercase().contains("glintindex"),
            "expected path to contain 'glintindex', got: {s}"
        );
    }

    #[test]
    fn default_index_directory_matches_app_paths() {
        let from_defaults = default_index_directory();
        let from_app_paths = AppPaths::new().index_dir();
        assert_eq!(from_defaults, from_app_paths);
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
