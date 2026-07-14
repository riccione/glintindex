//! Indexed Folders settings page.
//!
//! Displays configured folders with enable/disable and remove actions.
//! Provides an Add Folder button that opens the native folder picker.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, ListBox, Orientation, Separator, Window};

use crate::window::WindowState;

/// Builds the Indexed Folders settings page.
pub fn build(state: &Rc<RefCell<WindowState>>, parent: &Window) -> GtkBox {
    let content = GtkBox::new(Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title = Label::builder()
        .label("Indexed Folders")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        .build();
    content.append(&title);
    content.append(&Separator::new(gtk::Orientation::Horizontal));

    // Add Folder button
    let add_btn = Button::builder().label("Add Folder").build();

    let parent_clone = parent.clone();
    let state_clone = state.clone();
    add_btn.connect_clicked(move |_| {
        let dialog = gtk::FileChooserNative::new(
            Some("Select Folder to Index"),
            Some(&parent_clone),
            gtk::FileChooserAction::SelectFolder,
            Some("Select"),
            Some("Cancel"),
        );

        let state_clone = state_clone.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let mut st = state_clone.borrow_mut();
                        match st.service.add_folder(&path) {
                            Ok(()) => {
                                st.status = format!("Added: {}", path.display());
                            }
                            Err(e) => {
                                st.status = format!("Failed to add folder: {e}");
                            }
                        }
                    }
                }
            }
        });

        dialog.show();
    });

    content.append(&add_btn);

    // Folders list
    let listbox = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();

    let state_clone = state.clone();
    let listbox_clone = listbox.clone();
    gtk::glib::idle_add_local(move || {
        refresh_folder_list(&state_clone, &listbox_clone);
        gtk::glib::ControlFlow::Continue
    });

    content.append(&listbox);

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
        gtk::glib::ControlFlow::Continue
    });

    content.append(&status_label);
    content
}

/// Refreshes the folder list from the current state.
pub fn refresh_folder_list(state: &Rc<RefCell<WindowState>>, listbox: &ListBox) {
    // Clear existing rows
    while let Some(child) = listbox.first_child() {
        listbox.remove(&child);
    }

    let st = state.borrow();
    let folders: Vec<_> = st.service.indexed_folders().into_iter().cloned().collect();
    drop(st);

    for folder in &folders {
        let path_display = folder.path.display().to_string();
        let enabled = folder.enabled;

        let row_box = GtkBox::new(Orientation::Horizontal, 8);

        let path_label = Label::builder()
            .label(&path_display)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .selectable(true)
            .build();
        row_box.append(&path_label);

        let status_label = Label::builder()
            .label(if enabled { "Enabled" } else { "Disabled" })
            .css_classes([if enabled { "success" } else { "error" }])
            .build();
        row_box.append(&status_label);

        // Toggle button
        let toggle_label = if enabled { "Disable" } else { "Enable" };
        let toggle_btn = Button::builder().label(toggle_label).build();

        let state_clone = state.clone();
        let path_clone = path_display.clone();
        let listbox_clone = listbox.clone();
        toggle_btn.connect_clicked(move |_| {
            let mut st = state_clone.borrow_mut();
            let path = std::path::PathBuf::from(&path_clone);
            let result = if enabled {
                st.service.disable_folder(&path)
            } else {
                st.service.enable_folder(&path)
            };
            match result {
                Ok(()) => {
                    st.status = format!(
                        "{}: {}",
                        if enabled { "Disabled" } else { "Enabled" },
                        path_clone
                    );
                }
                Err(e) => {
                    st.status = format!("Failed: {e}");
                }
            }
            drop(st);
            refresh_folder_list(&state_clone, &listbox_clone);
        });
        row_box.append(&toggle_btn);

        // Remove button
        let remove_btn = Button::builder()
            .label("Remove")
            .css_classes(["destructive-action"])
            .build();

        let state_clone = state.clone();
        let path_clone = path_display.clone();
        let listbox_clone = listbox.clone();
        remove_btn.connect_clicked(move |_| {
            let mut st = state_clone.borrow_mut();
            let path = std::path::PathBuf::from(&path_clone);
            match st.service.remove_folder(&path) {
                Ok(()) => {
                    st.status = format!("Removed: {path_clone}");
                }
                Err(e) => {
                    st.status = format!("Failed to remove: {e}");
                }
            }
            drop(st);
            refresh_folder_list(&state_clone, &listbox_clone);
        });
        row_box.append(&remove_btn);

        listbox.append(&row_box);
    }
}
