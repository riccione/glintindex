//! Search bar UI component.
//!
//! Provides the search input field and settings button in the header.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, HeaderBar, Label, SearchEntry};

use crate::window::WindowState;

/// Builds the header bar containing the settings button and search entry.
pub fn build(state: &Rc<RefCell<WindowState>>) -> (HeaderBar, Button) {
    let settings_btn = Button::builder().label("Settings").build();

    let search_entry = SearchEntry::builder()
        .hexpand(true)
        .placeholder_text("Search files…")
        .build();

    // Connect search entry changes
    let state_clone = state.clone();
    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();
        let mut st = state_clone.borrow_mut();
        st.query = query.clone();

        if query.trim().is_empty() {
            st.results.clear();
            st.selected_index = None;
            st.status = "Ready".to_string();
            return;
        }

        // Execute search synchronously (fast for local Tantivy index)
        let query_obj = glintindex_core::SearchQuery::new(&query);
        match st.service.search(&query_obj) {
            Ok(results) => {
                let count = results.len();
                st.results = results;
                st.selected_index = None;
                st.status = format!(
                    "Found {} result{}",
                    count,
                    if count == 1 { "" } else { "s" }
                );
            }
            Err(e) => {
                st.results.clear();
                st.selected_index = None;
                st.status = format!("Search error: {}", e);
            }
        }
    });

    // Connect search entry activation (Enter key)
    let state_clone = state.clone();
    search_entry.connect_activate(move |entry| {
        let query = entry.text().to_string();
        let mut st = state_clone.borrow_mut();
        if query.trim().is_empty() {
            return;
        }

        let query_obj = glintindex_core::SearchQuery::new(&query);
        match st.service.search(&query_obj) {
            Ok(results) => {
                let count = results.len();
                st.results = results;
                st.selected_index = None;
                st.status = format!(
                    "Found {} result{}",
                    count,
                    if count == 1 { "" } else { "s" }
                );
            }
            Err(e) => {
                st.results.clear();
                st.selected_index = None;
                st.status = format!("Search error: {}", e);
            }
        }
    });

    let title_widget = GtkBox::new(gtk::Orientation::Horizontal, 8);
    title_widget.append(&Label::builder().label("GlintIndex").build());
    title_widget.append(&search_entry);

    let header = HeaderBar::builder().title_widget(&title_widget).build();

    header.pack_start(&settings_btn);

    (header, settings_btn)
}
