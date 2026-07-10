use std::path::PathBuf;

/// Application-level filesystem event abstraction.
///
/// Represents a detected change to a file in a watched directory.
/// This type is decoupled from any specific filesystem notification
/// library, providing a clean abstraction for the rest of the application.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use glintindex_core::watcher::WatchEvent;
///
/// let event = WatchEvent::Created(std::path::PathBuf::from("/home/user/docs/notes.txt"));
/// assert_eq!(event.path(), Path::new("/home/user/docs/notes.txt"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// A new file was created or appeared in a watched directory.
    Created(PathBuf),

    /// An existing file was modified.
    Modified(PathBuf),

    /// A file was deleted or moved out of a watched directory.
    Deleted(PathBuf),
}

impl WatchEvent {
    /// Returns the path of the affected file.
    pub fn path(&self) -> &std::path::Path {
        match self {
            WatchEvent::Created(p) | WatchEvent::Modified(p) | WatchEvent::Deleted(p) => p,
        }
    }

    /// Returns a human-readable description of the event type.
    pub fn kind(&self) -> &'static str {
        match self {
            WatchEvent::Created(_) => "created",
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Deleted(_) => "deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn created_event_path() {
        let event = WatchEvent::Created(PathBuf::from("/tmp/new.txt"));
        assert_eq!(event.path(), Path::new("/tmp/new.txt"));
        assert_eq!(event.kind(), "created");
    }

    #[test]
    fn modified_event_path() {
        let event = WatchEvent::Modified(PathBuf::from("/tmp/existing.txt"));
        assert_eq!(event.path(), Path::new("/tmp/existing.txt"));
        assert_eq!(event.kind(), "modified");
    }

    #[test]
    fn deleted_event_path() {
        let event = WatchEvent::Deleted(PathBuf::from("/tmp/removed.txt"));
        assert_eq!(event.path(), Path::new("/tmp/removed.txt"));
        assert_eq!(event.kind(), "deleted");
    }

    #[test]
    fn events_are_clonable() {
        let event = WatchEvent::Created(PathBuf::from("/tmp/test.txt"));
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn events_are_debuggable() {
        let event = WatchEvent::Modified(PathBuf::from("/tmp/test.txt"));
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Modified"));
    }
}
