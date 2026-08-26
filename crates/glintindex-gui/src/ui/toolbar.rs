//! Application toolbar.
//!
//! Provides the settings button and search entry in a toolbar below the
//! native OS title bar. Replaces the previous GtkHeaderBar-based title bar.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, ListBox, SearchEntry};

use crate::ui::results;
use crate::window::WindowState;

/// Monotonically increasing query ID to discard stale search results.
static NEXT_QUERY_ID: AtomicU64 = AtomicU64::new(1);

/// Performs a search on a background thread and sends the result through `tx`.
fn spawn_search(
    index_handle: std::sync::Arc<std::sync::Mutex<glintindex_core::IndexService>>,
    query: glintindex_core::SearchQuery,
    tx: mpsc::Sender<(u64, glintindex_core::SearchResponse)>,
    query_id: u64,
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
    });
}

/// Performs a search using the current state and sends results through the channel.
///
/// Used by pagination controls to trigger a search when the page changes.
pub fn perform_search_from_state(state: &Rc<RefCell<WindowState>>, query: &str) {
    if query.trim().is_empty() {
        return;
    }

    let (index_handle, per_page, current_page, tx) = {
        let st = state.borrow();
        let tx = st.search_tx.clone();
        (
            st.service.index_service_handle(),
            st.per_page,
            st.current_page,
            tx,
        )
    };

    let tx = match tx {
        Some(tx) => tx,
        None => {
            tracing::warn!("search_tx not initialized in WindowState");
            return;
        }
    };

    let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst);
    let search_query = glintindex_core::SearchQuery::paged(query, current_page, per_page);

    spawn_search(index_handle, search_query, tx, query_id);
}

/// Builds the toolbar containing the settings button, spacer, and search entry.
///
/// `search_rx` is the receiving end of the search channel created by the caller.
pub fn build_toolbar(
    state: &Rc<RefCell<WindowState>>,
    results_listbox: &ListBox,
    search_rx: mpsc::Receiver<(u64, glintindex_core::SearchResponse)>,
) -> (GtkBox, Button) {
    let settings_btn = Button::builder().label("Settings").build();

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search files…")
        .hexpand(true)
        .build();

    // ── Background search channel ──────────────────────────────
    let search_rx = Rc::new(RefCell::new(search_rx));

    // Track the latest query ID applied to the UI so stale
    // out-of-order results are discarded.
    let latest_applied_id = Rc::new(RefCell::new(0u64));

    // ── Periodic poll: drain channel every 50ms on GTK main loop ──
    {
        let state_clone = state.clone();
        let listbox = results_listbox.clone();
        let rx = search_rx.clone();
        let latest_id = latest_applied_id.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let mut newest: Option<(u64, glintindex_core::SearchResponse)> = None;
            while let Ok(msg) = rx.borrow().try_recv() {
                match &newest {
                    Some((id, _)) if msg.0 <= *id => {}
                    _ => newest = Some(msg),
                }
            }

            if let Some((query_id, response)) = newest {
                if query_id >= *latest_id.borrow() {
                    *latest_id.borrow_mut() = query_id;
                    let mut st = state_clone.borrow_mut();

                    // Extract fields before moving results into state
                    let total = response.total;
                    let offset = response.offset;
                    let limit = response.limit;
                    let results = response.results;

                    // Preserve pagination metadata in state
                    st.results = results;
                    st.total_results = total;
                    st.per_page = limit;
                    st.total_pages = if limit == 0 || total == 0 {
                        1
                    } else {
                        total.div_ceil(limit)
                    };
                    st.current_page = offset.checked_div(limit).map(|q| q + 1).unwrap_or(1);
                    st.selected_index = None;

                    let count = st.results.len();
                    let start = if st.total_results == 0 { 0 } else { offset + 1 };
                    let end = (offset + count).min(st.total_results);
                    st.status = if st.total_results == 0 {
                        "No results found".to_string()
                    } else {
                        format!("Showing {}–{} of {} results", start, end, st.total_results)
                    };

                    results::refresh_results_list(&listbox, &st.results);

                    // Update integrated status bar (status text + pagination)
                    super::status_bar::refresh_status_bar(
                        &st,
                        st.status_label.as_ref(),
                        st.pagination_page_label.as_ref(),
                        st.pagination_prev_btn.as_ref(),
                        st.pagination_next_btn.as_ref(),
                    );
                }
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    // ── Debounce timer ─────────────────────────────────────────
    let debounce_id: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));

    // ── connect_changed (typing) ───────────────────────────────
    {
        let state_clone = state.clone();
        let listbox = results_listbox.clone();
        let debounce = debounce_id.clone();

        search_entry.connect_changed(move |entry| {
            let query = entry.text().to_string();

            // Update state immediately (for preview, status, etc.)
            {
                let mut st = state_clone.borrow_mut();
                st.query = query.clone();
                // Reset pagination when query text changes
                st.current_page = 1;
            }

            // Cancel any pending debounce timeout
            if let Some(id) = debounce.borrow_mut().take() {
                id.remove();
            }

            if query.trim().is_empty() {
                let mut st = state_clone.borrow_mut();
                st.results.clear();
                st.selected_index = None;
                st.total_results = 0;
                st.total_pages = 1;
                st.current_page = 1;
                st.status = "No results found".to_string();
                results::refresh_results_list(&listbox, &st.results);
                super::status_bar::refresh_status_bar(
                    &st,
                    st.status_label.as_ref(),
                    st.pagination_page_label.as_ref(),
                    st.pagination_prev_btn.as_ref(),
                    st.pagination_next_btn.as_ref(),
                );
                return;
            }

            // Spawn a 150ms debounce timeout, then kick off the
            // actual search on a background thread.
            let state_for_spawn = state_clone.clone();
            let debounce_ref = debounce.clone();

            let source_id =
                gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                    let (index_handle, per_page, current_page, tx) = {
                        let st = state_for_spawn.borrow();
                        let tx = st.search_tx.clone();
                        (
                            st.service.index_service_handle(),
                            st.per_page,
                            st.current_page,
                            tx,
                        )
                    };

                    if let Some(tx) = tx {
                        let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst);
                        let query =
                            glintindex_core::SearchQuery::paged(&query, current_page, per_page);
                        spawn_search(index_handle, query, tx, query_id);
                    }

                    // Clear reference once fired
                    *debounce_ref.borrow_mut() = None;

                    gtk::glib::ControlFlow::Break
                });

            *debounce.borrow_mut() = Some(source_id);
        });
    }

    // ── connect_activate (Enter key) ───────────────────────────
    {
        let state_clone = state.clone();

        search_entry.connect_activate(move |entry| {
            let query = entry.text().to_string();
            if query.trim().is_empty() {
                return;
            }

            let (index_handle, per_page, current_page, tx) = {
                let st = state_clone.borrow();
                let tx = st.search_tx.clone();
                (
                    st.service.index_service_handle(),
                    st.per_page,
                    st.current_page,
                    tx,
                )
            };

            if let Some(tx) = tx {
                let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst);
                let query = glintindex_core::SearchQuery::paged(&query, current_page, per_page);
                spawn_search(index_handle, query, tx, query_id);
            }
        });
    }

    let left_tools = GtkBox::new(gtk::Orientation::Horizontal, 4);
    left_tools.append(&settings_btn);

    let spacer = GtkBox::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    let toolbar = GtkBox::new(gtk::Orientation::Horizontal, 6);
    toolbar.add_css_class("toolbar");
    toolbar.append(&left_tools);
    toolbar.append(&spacer);
    toolbar.append(&search_entry);

    (toolbar, settings_btn)
}
