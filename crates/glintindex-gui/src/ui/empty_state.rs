use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, Window};

use crate::window::WindowState;

/// Builds the empty state widget shown when no indexed folders are configured.
///
/// Displays a vertically and horizontally centered welcome panel with a
/// primary action to index a folder and a secondary action to open settings.
pub fn build(
    state: &Rc<RefCell<WindowState>>,
    parent_window: &impl IsA<Window>,
    view_stack: gtk::Stack,
) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.set_vexpand(true);
    container.set_hexpand(true);
    container.set_valign(gtk::Align::Center);
    container.set_halign(gtk::Align::Center);

    let inner = GtkBox::new(Orientation::Vertical, 16);
    inner.set_valign(gtk::Align::Center);
    inner.set_halign(gtk::Align::Center);

    let title = Label::builder()
        .label("Welcome to GlintIndex")
        .css_classes(["title-1"])
        .build();
    inner.append(&title);

    let subtitle = Label::builder()
        .label("Choose a folder to index to start searching your files.")
        .css_classes(["dim-label"])
        .build();
    inner.append(&subtitle);

    let index_btn = Button::builder()
        .label("Index Folder…")
        .css_classes(["suggested-action"])
        .halign(gtk::Align::Center)
        .build();

    let state_for_index = state.clone();
    let stack_for_index = view_stack.clone();
    index_btn.connect_clicked(move |_| {
        let state = state_for_index.clone();
        let stack = stack_for_index.clone();
        glib::spawn_future_local(async move {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title("Select Folder to Index")
                .pick_folder()
                .await;

            if let Some(file_handle) = dialog {
                let path = file_handle.path().to_path_buf();
                let mut st = state.borrow_mut();
                if st.service.add_folder(&path).is_ok() {
                    st.status = format!("Added: {}", path.display());
                    let _ = st.service.index_folder(&path);
                    drop(st);
                    stack.set_visible_child_name("main");
                }
            }
        });
    });
    inner.append(&index_btn);

    let or_label = Label::builder()
        .label("or")
        .css_classes(["dim-label"])
        .build();
    inner.append(&or_label);

    let settings_btn = Button::builder()
        .label("Open Settings")
        .halign(gtk::Align::Center)
        .build();

    let state_for_settings = state.clone();
    let parent_for_settings = parent_window.clone();
    let stack_for_settings = view_stack.clone();
    settings_btn.connect_clicked(move |_| {
        crate::ui::settings::show_settings(
            &parent_for_settings,
            &state_for_settings,
            Some(crate::ui::settings::SettingsPage::IndexedFolders),
            stack_for_settings.clone(),
        );
    });
    inner.append(&settings_btn);

    container.append(&inner);
    container
}
