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

/// Settings pages that can be navigated to directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    Appearance,
    IndexedFolders,
    IgnoredFolders,
    Index,
    About,
}

impl SettingsPage {
    fn stack_name(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::IndexedFolders => "indexed_folders",
            Self::IgnoredFolders => "ignored_folders",
            Self::Index => "index",
            Self::About => "about",
        }
    }
}

/// Shows the settings window.
///
/// If the settings window is already open, brings it to the front.
/// Otherwise, creates a new settings window and stores a reference
/// in `WindowState` so it can be closed by clicking Settings again.
///
/// `initial_page` optionally selects a specific settings page on open.
/// `view_stack` is the main window's content stack, used by settings
/// pages to trigger view switching when folders are added or removed.
pub fn show_settings(
    parent: &impl IsA<Window>,
    state: &Rc<RefCell<WindowState>>,
    initial_page: Option<SettingsPage>,
    view_stack: gtk::Stack,
) {
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

    let page_names = [
        SettingsPage::General,
        SettingsPage::Appearance,
        SettingsPage::IndexedFolders,
        SettingsPage::IgnoredFolders,
        SettingsPage::Index,
        SettingsPage::About,
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
    let appearance_page = appearance::build(state);
    let indexed_folders_page = indexed_folders::build(state, &settings_window, view_stack);
    let ignored_folders_page = ignored_folders::build(state);
    let index_page_widget = index_page::build(state);
    let about_page = about::build();

    pages.add_named(&general_page, Some("general"));
    pages.add_named(&appearance_page, Some("appearance"));
    pages.add_named(&indexed_folders_page, Some("indexed_folders"));
    pages.add_named(&ignored_folders_page, Some("ignored_folders"));
    pages.add_named(&index_page_widget, Some("index"));
    pages.add_named(&about_page, Some("about"));

    for (i, page) in page_names.iter().enumerate() {
        let display = display_names[i];
        let row = ListBoxRow::builder()
            .child(&Label::builder().label(display).build())
            .build();
        sidebar.append(&row);

        // If this is the initial page, store a reference to select it later
        if Some(*page) == initial_page {
            if let Some(target_row) = sidebar.row_at_index(i as i32) {
                sidebar.select_row(Some(&target_row));
                pages.set_visible_child_name(page.stack_name());
            }
        }
    }

    // Connect sidebar selection to page switching
    let pages_clone = pages.clone();
    sidebar.connect_row_selected(move |_listbox, row| {
        if let Some(row) = row {
            let index = row.index() as usize;
            if let Some(page) = page_names.get(index) {
                pages_clone.set_visible_child_name(page.stack_name());
            }
        }
    });

    // Select first page by default if no initial page was requested
    if initial_page.is_none() {
        if let Some(row) = sidebar.row_at_index(0) {
            sidebar.select_row(Some(&row));
        }
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
