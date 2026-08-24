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

/// Builds the toolbar containing the settings button, spacer, and search entry.
pub fn build_toolbar(
    state: &Rc<RefCell<WindowState>>,
    results_listbox: &ListBox,
) -> (GtkBox, Button) {
    let settings_btn = Button::builder().label("Settings").build();

    let search_entry = SearchEntry::builder()
        .placeholder_text("Search files…")
        .hexpand(true)
        .build();

    // ── Background search channel ──────────────────────────────
    // Results are sent from a background thread via mpsc and
    // drained on the GTK main loop by a periodic poll timer.
    let (tx, rx) = mpsc::channel::<(u64, Vec<glintindex_core::SearchResult>)>();
    let rx = Rc::new(RefCell::new(rx));

    // Track the latest query ID applied to the UI so stale
    // out-of-order results are discarded.
    let latest_applied_id = Rc::new(RefCell::new(0u64));

    // Periodic poll: drain the mpsc receiver every 50ms on the
    // GTK main loop and apply the newest result batch.
    {
        let state_clone = state.clone();
        let listbox = results_listbox.clone();
        let rx = rx.clone();
        let latest_id = latest_applied_id.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            // Drain all pending messages, keeping only the newest
            let mut newest: Option<(u64, Vec<glintindex_core::SearchResult>)> = None;
            while let Ok(msg) = rx.borrow().try_recv() {
                match &newest {
                    Some((id, _)) if msg.0 <= *id => {}
                    _ => newest = Some(msg),
                }
            }

            if let Some((query_id, results)) = newest {
                if query_id >= *latest_id.borrow() {
                    *latest_id.borrow_mut() = query_id;
                    let mut st = state_clone.borrow_mut();
                    st.results = results;
                    st.selected_index = None;
                    let count = st.results.len();
                    st.status = format!(
                        "Found {} result{}",
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                    results::refresh_results_list(&listbox, &st.results);
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
        let tx = tx.clone();
        let debounce = debounce_id.clone();

        search_entry.connect_changed(move |entry| {
            let query = entry.text().to_string();

            // Update state immediately (for preview, status, etc.)
            {
                let mut st = state_clone.borrow_mut();
                st.query = query.clone();
            }

            // Cancel any pending debounce timeout
            if let Some(id) = debounce.borrow_mut().take() {
                id.remove();
            }

            if query.trim().is_empty() {
                let mut st = state_clone.borrow_mut();
                st.results.clear();
                st.selected_index = None;
                st.status = "Ready".to_string();
                results::refresh_results_list(&listbox, &st.results);
                return;
            }

            // Spawn a 150ms debounce timeout, then kick off the
            // actual search on a background thread.
            let tx_clone = tx.clone();
            let state_for_spawn = state_clone.clone();
            let debounce_ref = debounce.clone();

            let source_id =
                gtk::glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                    let index_handle = {
                        let st = state_for_spawn.borrow();
                        st.service.index_service_handle()
                    };

                    let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst);
                    let query_text = query.clone();
                    let tx = tx_clone.clone();

                    std::thread::spawn(move || {
                        use glintindex_core::traits::SearchEngine;
                        let query_obj = glintindex_core::SearchQuery::new(&query_text);
                        let result = {
                            let svc = index_handle.lock().map_err(|e| {
                                glintindex_core::GlintIndexError::Other(format!(
                                    "lock poisoned: {e}"
                                ))
                            });
                            svc.and_then(|svc| svc.search(&query_obj))
                        };
                        match result {
                            Ok(results) => {
                                let _ = tx.send((query_id, results));
                            }
                            Err(e) => {
                                tracing::warn!(
                                    target: "glintindex::gui",
                                    error = %e,
                                    "background search failed"
                                );
                                let _ = tx.send((query_id, Vec::new()));
                            }
                        }
                    });

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
        let tx = tx.clone();

        search_entry.connect_activate(move |entry| {
            let query = entry.text().to_string();
            if query.trim().is_empty() {
                return;
            }

            let index_handle = {
                let st = state_clone.borrow();
                st.service.index_service_handle()
            };

            let query_id = NEXT_QUERY_ID.fetch_add(1, Ordering::SeqCst);
            let query_text = query.clone();
            let tx = tx.clone();

            std::thread::spawn(move || {
                use glintindex_core::traits::SearchEngine;
                let query_obj = glintindex_core::SearchQuery::new(&query_text);
                let result = {
                    let svc = index_handle.lock().map_err(|e| {
                        glintindex_core::GlintIndexError::Other(format!("lock poisoned: {e}"))
                    });
                    svc.and_then(|svc| svc.search(&query_obj))
                };
                match result {
                    Ok(results) => {
                        let _ = tx.send((query_id, results));
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "glintindex::gui",
                            error = %e,
                            "background search failed"
                        );
                    }
                }
            });
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
