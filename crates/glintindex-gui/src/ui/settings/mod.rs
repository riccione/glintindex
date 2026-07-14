//! Settings window module.
//!
//! Provides the settings window with sidebar navigation and
//! multiple content pages for managing application configuration.

pub mod about;
pub mod appearance;
pub mod general;
pub mod ignored_folders;
pub mod index_page;
pub mod indexed_folders;

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Stack, Window};

use crate::window::WindowState;

/// Shows the settings window.
///
/// If the settings window is already open, brings it to the front.
/// Otherwise, creates a new settings window and stores a reference
/// in `WindowState` so it can be closed by clicking Settings again.
pub fn show_settings(parent: &impl IsA<Window>, state: &Rc<RefCell<WindowState>>) {
    // If settings window already exists, just present it
    {
        let st = state.borrow();
        if let Some(ref existing) = st.settings_window {
            existing.present();
            return;
        }
    }

    let settings_window = Window::builder()
        .title("Settings")
        .default_width(700)
        .default_height(500)
        .transient_for(parent)
        .build();

    // ── Sidebar navigation ──────────────────────────────────────
    let sidebar = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    let pages = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let page_names = vec![
        "general".to_string(),
        "appearance".to_string(),
        "indexed_folders".to_string(),
        "ignored_folders".to_string(),
        "index".to_string(),
        "about".to_string(),
    ];

    let display_names = [
        "General",
        "Appearance",
        "Indexed Folders",
        "Ignored Folders",
        "Index",
        "About",
    ];

    let general_page = general::build(state);
    let appearance_page = appearance::build(state, &settings_window);
    let indexed_folders_page = indexed_folders::build(state, &settings_window);
    let ignored_folders_page = ignored_folders::build(state);
    let index_page_widget = index_page::build(state);
    let about_page = about::build();

    pages.add_named(&general_page, Some("general"));
    pages.add_named(&appearance_page, Some("appearance"));
    pages.add_named(&indexed_folders_page, Some("indexed_folders"));
    pages.add_named(&ignored_folders_page, Some("ignored_folders"));
    pages.add_named(&index_page_widget, Some("index"));
    pages.add_named(&about_page, Some("about"));

    for (i, _name) in page_names.iter().enumerate() {
        let display = display_names[i];
        let row = ListBoxRow::builder()
            .child(&Label::builder().label(display).build())
            .build();
        sidebar.append(&row);
    }

    // Connect sidebar selection to page switching
    let pages_clone = pages.clone();
    let page_names_clone = page_names.clone();
    sidebar.connect_row_selected(move |_listbox, row| {
        if let Some(row) = row {
            let index = row.index() as usize;
            if let Some(name) = page_names_clone.get(index) {
                pages_clone.set_visible_child_name(name);
            }
        }
    });

    // Select first page by default
    if let Some(row) = sidebar.row_at_index(0) {
        sidebar.select_row(Some(&row));
    }

    // ── Layout ──────────────────────────────────────────────────
    let sidebar_scroll = ScrolledWindow::builder()
        .child(&sidebar)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(160)
        .build();

    let content_box = GtkBox::new(Orientation::Horizontal, 0);
    content_box.append(&sidebar_scroll);
    content_box.append(&gtk::Separator::new(Orientation::Vertical));
    content_box.append(&pages);

    settings_window.set_child(Some(&content_box));

    // Connect close-request to clear the stored reference
    let state_for_close = state.clone();
    settings_window.connect_close_request(move |_| {
        let mut st = state_for_close.borrow_mut();
        st.settings_window = None;
        gtk::glib::Propagation::Proceed
    });

    // Add Esc key handler to close the window
    let key_controller = gtk::EventControllerKey::new();
    let settings_window_for_key = settings_window.clone();
    let state_for_key = state.clone();
    key_controller.connect_key_pressed(move |_controller, key, _keycode, _modifiers| {
        if key == gtk::gdk::Key::Escape {
            settings_window_for_key.set_visible(false);
            let mut st = state_for_key.borrow_mut();
            st.settings_window = None;
        }
        gtk::glib::Propagation::Proceed
    });
    settings_window.add_controller(key_controller);

    // Store the window reference in state
    {
        let mut st = state.borrow_mut();
        st.settings_window = Some(settings_window.clone());
    }

    settings_window.present();
}
