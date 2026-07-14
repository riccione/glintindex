//! Ignored Folders settings page.
//!
//! Displays the list of ignored folder names with add/remove actions.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, ListBox, Orientation, ScrolledWindow, Separator};

use crate::window::WindowState;

/// Builds the Ignored Folders settings page.
pub fn build(state: &Rc<RefCell<WindowState>>) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("Ignored Folders")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    let description = Label::builder()
        .label("Folder names excluded from indexing. Changes are saved immediately.")
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .wrap(true)
        .build();
    content.append(&description);

    // Add folder entry + button
    let add_box = GtkBox::new(Orientation::Horizontal, 8);
    let entry = Entry::builder()
        .placeholder_text("New folder name...")
        .hexpand(true)
        .build();

    let add_btn = Button::builder().label("Add").build();

    add_box.append(&entry);
    add_box.append(&add_btn);
    content.append(&add_box);

    // Ignored folders list
    let listbox = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();

    let state_clone = state.clone();
    let listbox_clone = listbox.clone();
    gtk::glib::idle_add_local(move || {
        refresh_ignored_list(&state_clone, &listbox_clone);
        gtk::glib::ControlFlow::Break
    });

    let scrolled = ScrolledWindow::builder()
        .child(&listbox)
        .vexpand(true)
        .build();
    content.append(&scrolled);

    // Connect Add button
    let state_clone = state.clone();
    let entry_clone = entry.clone();
    let listbox_clone = listbox.clone();
    add_btn.connect_clicked(move |_| {
        let name = entry_clone.text().to_string();
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let mut st = state_clone.borrow_mut();
        match st.service.add_ignored_folder(trimmed.clone()) {
            Ok(()) => {
                st.status = format!("Added: {trimmed}");
            }
            Err(e) => {
                st.status = format!("Failed to add: {e}");
            }
        }
        entry_clone.set_text("");
        drop(st);
        refresh_ignored_list(&state_clone, &listbox_clone);
    });

    // Connect Enter key in entry
    let state_clone2 = state.clone();
    let _entry_clone2 = entry.clone();
    let listbox_clone2 = listbox.clone();
    entry.connect_activate(move |entry| {
        let name = entry.text().to_string();
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let mut st = state_clone2.borrow_mut();
        match st.service.add_ignored_folder(trimmed.clone()) {
            Ok(()) => {
                st.status = format!("Added: {trimmed}");
            }
            Err(e) => {
                st.status = format!("Failed to add: {e}");
            }
        }
        entry.set_text("");
        drop(st);
        refresh_ignored_list(&state_clone2, &listbox_clone2);
    });

    // Status label
    let status_label = Label::builder()
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .build();

    let state_clone = state.clone();
    let status_clone = status_label.clone();
    gtk::glib::idle_add_local(move || {
        let st = state_clone.borrow();
        status_clone.set_text(&st.status);
        gtk::glib::ControlFlow::Break
    });

    content.append(&status_label);
    content
}

/// Refreshes the ignored folders list from the current state.
pub fn refresh_ignored_list(state: &Rc<RefCell<WindowState>>, listbox: &ListBox) {
    // Clear existing rows
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

    let st = state.borrow();
    let names: Vec<String> = st.service.ignored_folders().to_vec();
    drop(st);

    for name in &names {
        let row_box = GtkBox::new(Orientation::Horizontal, 8);

        let name_label = Label::builder()
            .label(name)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .selectable(true)
            .build();
        row_box.append(&name_label);

        let remove_btn = Button::builder()
            .label("Remove")
            .css_classes(["destructive-action"])
            .build();

        let state_clone = state.clone();
        let name_clone = name.clone();
        let listbox_clone = listbox.clone();
        remove_btn.connect_clicked(move |_| {
            let mut st = state_clone.borrow_mut();
            match st.service.remove_ignored_folder(&name_clone) {
                Ok(()) => {
                    st.status = format!("Removed: {name_clone}");
                }
                Err(e) => {
                    st.status = format!("Failed to remove: {e}");
                }
            }
            drop(st);
            refresh_ignored_list(&state_clone, &listbox_clone);
        });
        row_box.append(&remove_btn);

        listbox.append(&row_box);
    }
}
