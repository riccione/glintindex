//! Application messages for the GlintIndex GUI.
//!
//! Messages drive the state machine of the application. Each message
//! represents a user interaction or system event that may cause the
//! application state to change.

use glintindex_core::{PreviewOutput, SearchResult};

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
    /// A debounced search should execute after the timer fires.
    SearchDebounced(String),
    /// Search results were received from the index.
    SearchCompleted(Vec<SearchResult>),
    /// A search result was selected by the user.
    ResultSelected(usize),
    /// A search result was activated (double-click or Enter).
    ResultActivated(usize),
    /// The search input was submitted (Enter key pressed).
    SearchSubmitted,
    /// An error occurred during a search operation.
    SearchError(String),

    // ── File Operations ─────────────────────────────────────────
    /// Request to open the selected file with the default application.
    OpenFileRequested(usize),
    /// Request to open the containing folder of the selected file.
    OpenFolderRequested(usize),
    /// Request to copy the full path of the selected file to clipboard.
    CopyPathRequested(usize),
    /// Clipboard operation completed with a status message.
    ClipboardCompleted(String),

    // ── Recent Searches ─────────────────────────────────────────
    /// Request to select a recent search query.
    RecentSearchSelected(String),
    /// Request to clear all recent searches.
    ClearRecentSearches,

    // ── Keyboard Navigation ─────────────────────────────────────
    /// Navigate to the previous result in the list.
    NavigateUp,
    /// Navigate to the next result in the list.
    NavigateDown,
    /// Activate the currently selected result.
    ActivateSelected,

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

    // ── Preview ─────────────────────────────────────────────────
    /// Request to preview a file at the given path.
    PreviewRequested(String),
    /// Preview loaded successfully with syntax-highlighted content.
    PreviewLoaded(PreviewOutput),
    /// Preview failed with an error message.
    PreviewFailed(String),
    /// Search highlights were updated in the preview.
    SearchHighlightsUpdated(String),

    // ── Background Indexing ─────────────────────────────────────
    /// Request to start indexing all enabled folders in background.
    StartIndexing,
    /// Request to start rebuilding the index in background.
    StartRebuild,
    /// Periodic progress update from background job.
    ProgressTick,
    /// Background indexing completed successfully.
    IndexingCompleted(String),
    /// Background indexing failed with an error message.
    IndexingFailed(String),
    /// Background rebuild completed successfully.
    RebuildCompleted(String),
    /// Background rebuild failed with an error message.
    RebuildFailed(String),

    // ── Legacy Index Management (kept for compatibility) ────────
    /// Request to index all enabled folders (deprecated, use StartIndexing).
    IndexRequested,
    /// Indexing completed with a status message.
    IndexCompleted(String),
    /// Request to rebuild the index from scratch (deprecated, use StartRebuild).
    RebuildRequestedLegacy,
    /// Rebuild completed with a status message (legacy).
    RebuildCompletedLegacy(String),
    /// Request to clear all indexed documents.
    ClearRequested,
    /// Clear completed with a status message.
    ClearCompleted(String),
    /// Statistics have been refreshed.
    StatisticsUpdated,
}
