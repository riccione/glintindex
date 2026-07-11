//! Application messages for the GlintIndex GUI.
//!
//! Messages drive the state machine of the application. Each message
//! represents a user interaction or system event that may cause the
//! application state to change.

use glintindex_core::SearchResult;

/// The active page within the Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    /// General application information.
    General,
    /// Indexed folders management.
    IndexedFolders,
    /// Ignored folder names management.
    IgnoredFolders,
    /// Index management and statistics.
    Index,
    /// About page with version and license info.
    About,
}

/// Messages that drive the application state transitions.
///
/// Each message represents a user interaction or system event
/// that may cause the application state to change.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // ── Search ──────────────────────────────────────────────────
    /// The search input text changed.
    SearchChanged(String),
    /// Search results were received from the index.
    SearchCompleted(Vec<SearchResult>),
    /// A search result was selected by the user.
    ResultSelected(usize),
    /// The search input was submitted (Enter key pressed).
    SearchSubmitted,
    /// An error occurred during a search operation.
    SearchError(String),

    // ── Settings Navigation ─────────────────────────────────────
    /// Open the settings window (overlay on main view).
    OpenSettings,
    /// Close the settings window and return to main view.
    CloseSettings,
    /// Select a specific settings page.
    SettingsPageSelected(SettingsPage),

    // ── Indexed Folders ─────────────────────────────────────────
    /// A folder was successfully added to the configuration.
    FolderAdded(String),
    /// A folder was successfully removed from the configuration.
    FolderRemoved(String),
    /// A folder was enabled in the configuration.
    FolderEnabled(String),
    /// A folder was disabled in the configuration.
    FolderDisabled(String),
    /// Request to add a folder via native folder picker.
    AddFolderRequested,
    /// Request to remove a folder by path.
    RemoveFolderRequested(String),
    /// Request to enable a folder by path.
    EnableFolderRequested(String),
    /// Request to disable a folder by path.
    DisableFolderRequested(String),

    // ── Ignored Folders ─────────────────────────────────────────
    /// An ignored folder name was added.
    IgnoredFolderAdded(String),
    /// An ignored folder name was removed.
    IgnoredFolderRemoved(String),
    /// Request to add an ignored folder name.
    AddIgnoredFolderRequested(String),
    /// Request to remove an ignored folder name.
    RemoveIgnoredFolderRequested(String),

    // ── Index Management ────────────────────────────────────────
    /// Request to index all enabled folders.
    IndexRequested,
    /// Indexing completed with a status message.
    IndexCompleted(String),
    /// Request to rebuild the index from scratch.
    RebuildRequested,
    /// Rebuild completed with a status message.
    RebuildCompleted(String),
    /// Request to clear all indexed documents.
    ClearRequested,
    /// Clear completed with a status message.
    ClearCompleted(String),
    /// Statistics have been refreshed.
    StatisticsUpdated,
}
