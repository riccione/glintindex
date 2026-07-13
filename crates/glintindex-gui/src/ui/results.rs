//! Results list UI component.
//!
//! Displays search results as a scrollable list with file names and paths.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, ListBox, Orientation};

use crate::window::WindowState;

/// Builds the results list widget.
pub fn build(state: &Rc<RefCell<WindowState>>) -> ListBox {
    let listbox = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    // Connect row selection to state update
    let state_clone = state.clone();
    listbox.connect_row_selected(move |_listbox, row| {
        if let Some(row) = row {
            let index = row.index() as usize;
            let mut st = state_clone.borrow_mut();
            st.selected_index = Some(index);
            if index < st.results.len() {
                let path = &st.results[index].document.path;
                st.status = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
            }
        }
    });

    // Double-click to open file
    let state_clone = state.clone();
    listbox.connect_row_activated(move |_listbox, row| {
        let index = row.index() as usize;
        let st = state_clone.borrow();
        if index < st.results.len() {
            let path = &st.results[index].document.path;
            let _ = open::that(path);
        }
    });

    listbox
}

/// Creates a single result row widget.
#[allow(dead_code)]
pub fn create_result_row(result: &glintindex_core::SearchResult) -> GtkBox {
    let filename = result.document.filename().to_string();
    let path = result.document.path.display().to_string();

    let filename_label = Label::builder()
        .label(&filename)
        .halign(gtk::Align::Start)
        .build();

    let path_label = Label::builder()
        .label(&path)
        .halign(gtk::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();

    let row_content = GtkBox::new(Orientation::Vertical, 2);
    row_content.append(&filename_label);
    row_content.append(&path_label);

    row_content
}
