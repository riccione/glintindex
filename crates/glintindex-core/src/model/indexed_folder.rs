use std::path::PathBuf;

/// Represents a folder that has been configured for indexing.
///
/// Each indexed folder has a path and an enabled flag that allows
/// temporarily disabling indexing without removing the configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct IndexedFolder {
    /// Absolute path to the folder.
    pub path: PathBuf,
    /// Whether this folder is currently being indexed.
    pub enabled: bool,
}

impl IndexedFolder {
    /// Creates a new indexed folder configuration.
    pub fn new(path: PathBuf, enabled: bool) -> Self {
        Self { path, enabled }
    }

    /// Creates an enabled indexed folder.
    pub fn enabled(path: PathBuf) -> Self {
        Self::new(path, true)
    }

    /// Creates a disabled indexed folder.
    pub fn disabled(path: PathBuf) -> Self {
        Self::new(path, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_enabled() {
        let folder = IndexedFolder::new(PathBuf::from("/home/user/docs"), true);
        assert_eq!(folder.path, PathBuf::from("/home/user/docs"));
        assert!(folder.enabled);
    }

    #[test]
    fn new_disabled() {
        let folder = IndexedFolder::new(PathBuf::from("/tmp"), false);
        assert!(!folder.enabled);
    }

    #[test]
    fn convenience_constructors() {
        let enabled = IndexedFolder::enabled(PathBuf::from("/a"));
        assert!(enabled.enabled);
        let disabled = IndexedFolder::disabled(PathBuf::from("/b"));
        assert!(!disabled.enabled);
    }

    #[test]
    fn roundtrip_serde() {
        let folder = IndexedFolder::enabled(PathBuf::from("/home"));
        let json = serde_json::to_string(&folder).unwrap();
        let restored: IndexedFolder = serde_json::from_str(&json).unwrap();
        assert_eq!(folder, restored);
    }
}
