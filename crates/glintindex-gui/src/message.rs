use glintindex_core::SearchResult;

/// Messages that drive the application state transitions.
///
/// Each message represents a user interaction or system event
/// that may cause the application state to change.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
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
}
