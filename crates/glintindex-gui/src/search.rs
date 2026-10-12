//! Background search plumbing.
//!
//! [`spawn_search`] runs a query on a detached thread against the shared
//! index handle and delivers the response over an mpsc channel;
//! [`apply_search_response`] folds a response into [`GlintApp`] state.
//! Both are toolkit-agnostic and unit-tested directly.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use crate::app::GlintApp;

/// Minimum trimmed characters required before a search is dispatched.
pub(crate) const MIN_QUERY_CHARS: usize = 4;

/// Returns whether `query` is eligible to be searched.
///
/// Trailing/leading whitespace does not count toward
/// [`MIN_QUERY_CHARS`].
pub(crate) fn query_eligible(query: &str) -> bool {
    query.trim().chars().count() >= MIN_QUERY_CHARS
}

/// Monotonically increasing query ID to discard stale search results.
static NEXT_QUERY_ID: AtomicU64 = AtomicU64::new(1);

/// Performs a search on a background thread and sends the result through `tx`.
///
/// `ctx` (when present) is used to wake the UI after the response is
/// sent — egui does not repaint on its own while idle, so without the
/// wake-up a response can sit unread until the next input event.
fn spawn_search(
    index_handle: std::sync::Arc<std::sync::Mutex<glintindex_core::IndexService>>,
    query: glintindex_core::SearchQuery,
    tx: mpsc::Sender<(u64, glintindex_core::SearchResponse)>,
    query_id: u64,
    ctx: Option<eframe::egui::Context>,
) {
    std::thread::spawn(move || {
        use glintindex_core::traits::SearchEngine;
        let result = {
            let svc = index_handle.lock().map_err(|e| {
                glintindex_core::GlintIndexError::Other(format!("lock poisoned: {e}"))
            });
            svc.and_then(|svc| svc.search(&query))
        };
        match result {
            Ok(response) => {
                let _ = tx.send((query_id, response));
            }
            Err(e) => {
                tracing::warn!(
                    target: "glintindex::gui",
                    error = %e,
                    "background search failed"
                );
                let _ = tx.send((
                    query_id,
                    glintindex_core::SearchResponse::new(Vec::new(), 0, 0, 0),
                ));
            }
        }
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    });
}

/// Applies a search response to app state (state mutation only).
///
/// `limit == 0` is the error sentinel sent by [`spawn_search`] when the
/// background search fails; it renders as "Search failed". `per_page` is
/// deliberately never overwritten from responses — it is a UI/config
/// preference, and echoing a zeroed error limit back would poison every
/// subsequent query (leading to a `TopDocs::with_limit(0)` panic).
pub(crate) fn apply_search_response(app: &mut GlintApp, response: glintindex_core::SearchResponse) {
    let total = response.total;
    let offset = response.offset;
    let limit = response.limit;

    app.results = response.results;
    app.total_results = total;
    app.invalidate_preview();
    app.selected_path = None;
    app.total_pages = if limit == 0 || total == 0 {
        1
    } else {
        total.div_ceil(limit)
    };
    app.current_page = offset.checked_div(limit).map(|q| q + 1).unwrap_or(1);
    app.selected_index = None;

    let count = app.results.len();
    let start = if app.total_results == 0 {
        0
    } else {
        offset + 1
    };
    let end = (offset + count).min(app.total_results);
    app.status = if limit == 0 {
        "Search failed".to_string()
    } else if app.total_results == 0 {
        "No results found".to_string()
    } else {
        format!("Showing {}–{} of {} results", start, end, app.total_results)
    };
}

impl GlintApp {
    /// Returns the next monotonically increasing query ID.
    pub(crate) fn next_query_id() -> u64 {
        NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// Dispatches a background search for the current query/page.
    ///
    /// Marks the query as dispatched so the debounce path does not
    /// re-fire. No-op for queries under [`MIN_QUERY_CHARS`] (including
    /// empty) or when the channel is gone.
    pub(crate) fn dispatch_search(&mut self) {
        let query = self.query.clone();
        if !query_eligible(&query) {
            return;
        }
        let Some(tx) = self.search_tx.clone() else {
            tracing::warn!("search_tx not initialized");
            return;
        };
        let index_handle = self.service.index_service_handle();
        let search_query =
            glintindex_core::SearchQuery::paged(&query, self.current_page, self.per_page)
                .with_fuzzy(self.service.config().fuzzy_search);
        let query_id = Self::next_query_id();
        spawn_search(
            index_handle,
            search_query,
            tx,
            query_id,
            self.frame_ctx.clone(),
        );
        self.last_dispatched = query;
    }

    /// Handles a change to the search text (typing or programmatic).
    ///
    /// Resets pagination, arms the 150ms debounce, and clears results
    /// when the input is not yet eligible (under [`MIN_QUERY_CHARS`]),
    /// with a status hint while the query is non-empty.
    pub(crate) fn on_query_changed(&mut self) {
        if self.search_text == self.query {
            return;
        }
        self.query = self.search_text.clone();
        self.current_page = 1;
        self.last_query_edit = Some(std::time::Instant::now());
        if !query_eligible(&self.query) {
            self.clear_results();
            if !self.query.is_empty() {
                self.status = format!("Type at least {MIN_QUERY_CHARS} characters to search");
            }
        }
    }

    /// Clears the search input and resets query/results (clear button).
    ///
    /// `on_query_changed` collapses the empty input into a query reset
    /// plus an immediate `clear_results`; the armed debounce is inert
    /// for an ineligible query, and in-flight responses are dropped by
    /// [`GlintApp::drain_search_channel`].
    pub(crate) fn clear_search(&mut self) {
        self.search_text.clear();
        self.on_query_changed();
    }

    /// Clears results, selection, preview, and status for an empty query.
    pub(crate) fn clear_results(&mut self) {
        self.results.clear();
        self.selected_index = None;
        self.selected_path = None;
        self.invalidate_preview();
        self.total_results = 0;
        self.total_pages = 1;
        self.current_page = 1;
        self.status = "No results found".to_string();
        self.last_dispatched = String::new();
    }

    /// Moves one page forward/backward and re-searches.
    pub(crate) fn change_page(&mut self, delta: i32) {
        let new_page = self.current_page as i32 + delta;
        if new_page < 1 || new_page as usize > self.total_pages {
            return;
        }
        self.current_page = new_page as usize;
        self.dispatch_search();
    }

    /// Fires the debounced search once the 150ms window has elapsed.
    ///
    /// Ineligible queries return before any repaint is armed, so a
    /// short query never spins the frame loop.
    pub(crate) fn handle_debounce(&mut self, ctx: &eframe::egui::Context) {
        if !query_eligible(&self.query) || self.last_dispatched == self.query {
            return;
        }
        let Some(edited_at) = self.last_query_edit else {
            return;
        };
        let debounce = std::time::Duration::from_millis(150);
        let elapsed = edited_at.elapsed();
        if elapsed >= debounce {
            self.dispatch_search();
        } else {
            ctx.request_repaint_after(debounce - elapsed);
        }
    }
}
