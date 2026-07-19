//! Application toolbar.
//!
//! Provides the settings button and search entry in a toolbar below the
//! native OS title bar. Replaces the previous GtkHeaderBar-based title bar.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, ListBox, SearchEntry};

use crate::ui::results;
use crate::window::WindowState;

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

    let listbox_for_search = results_listbox.clone();

    let state_clone = state.clone();
    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();
        let mut st = state_clone.borrow_mut();
        st.query = query.clone();

        if query.trim().is_empty() {
            st.results.clear();
            st.selected_index = None;
            st.status = "Ready".to_string();
            results::refresh_results_list(&listbox_for_search, &st.results);
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
                results::refresh_results_list(&listbox_for_search, &st.results);
            }
            Err(e) => {
                st.results.clear();
                st.selected_index = None;
                st.status = format!("Search error: {}", e);
                results::refresh_results_list(&listbox_for_search, &st.results);
            }
        }
    });

    let state_clone = state.clone();
    let listbox_for_activate = results_listbox.clone();
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
                results::refresh_results_list(&listbox_for_activate, &st.results);
            }
            Err(e) => {
                st.results.clear();
                st.selected_index = None;
                st.status = format!("Search error: {}", e);
                results::refresh_results_list(&listbox_for_activate, &st.results);
            }
        }
    });

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
