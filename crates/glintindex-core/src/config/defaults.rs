use std::path::PathBuf;

/// Returns the default index storage directory.
///
/// On Linux this is `~/.local/share/glintindex/index`.
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
    PathBuf::from(std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.local/share")
    }))
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
