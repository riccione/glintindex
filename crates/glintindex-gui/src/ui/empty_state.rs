use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, ProgressBar, Window};

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

    // ── Title and subtitle ─────────────────────────────────────
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

    // ── Primary action button ──────────────────────────────────
    let index_btn = Button::builder()
        .label("Index Folder…")
        .css_classes(["suggested-action"])
        .halign(gtk::Align::Center)
        .build();
    inner.append(&index_btn);

    // ── Separator ──────────────────────────────────────────────
    let or_label = Label::builder()
        .label("or")
        .css_classes(["dim-label"])
        .build();
    inner.append(&or_label);

    // ── Secondary action button ────────────────────────────────
    let settings_btn = Button::builder()
        .label("Open Settings")
        .halign(gtk::Align::Center)
        .build();
    inner.append(&settings_btn);

    // ── Progress bar (hidden initially) ────────────────────────
    let progress_bar = ProgressBar::builder()
        .show_text(true)
        .visible(false)
        .build();
    inner.append(&progress_bar);

    // ── Status label (hidden initially) ────────────────────────
    let status_label = Label::builder()
        .label("")
        .css_classes(["dim-label"])
        .build();
    inner.append(&status_label);

    container.append(&inner);

    // ── Index Folder button handler ────────────────────────────
    {
        let state = state.clone();
        let view_stack = view_stack.clone();
        let index_btn = index_btn.clone();
        let settings_btn = settings_btn.clone();
        let title = title.clone();
        let subtitle = subtitle.clone();
        let progress_bar = progress_bar.clone();
        let status_label = status_label.clone();
        let parent_window = parent_window.clone();

        index_btn.clone().connect_clicked(move |_| {
            let state = state.clone();
            let view_stack = view_stack.clone();
            let index_btn = index_btn.clone();
            let settings_btn = settings_btn.clone();
            let title = title.clone();
            let subtitle = subtitle.clone();
            let progress_bar = progress_bar.clone();
            let status_label = status_label.clone();
            let _parent_window = parent_window.clone();

            glib::spawn_future_local(async move {
                let dialog = rfd::AsyncFileDialog::new()
                    .set_title("Select Folder to Index")
                    .pick_folder()
                    .await;

                let Some(file_handle) = dialog else {
                    return;
                };
                let path = file_handle.path().to_path_buf();

                // 1. Add folder to config
                {
                    let mut st = state.borrow_mut();
                    if let Err(e) = st.service.add_folder(&path) {
                        st.status = format!("Failed to add folder: {e}");
                        return;
                    }
                }

                // 2. Enter indexing state
                title.set_text("Creating your search index…");
                subtitle.set_text("This may take several minutes.");
                index_btn.set_label("Indexing…");
                index_btn.set_sensitive(false);
                settings_btn.set_sensitive(false);
                progress_bar.set_visible(true);
                progress_bar.set_show_text(true);
                progress_bar.set_fraction(0.0);
                progress_bar.pulse();
                status_label.set_text("");

                // 3. Start background indexing
                {
                    let mut st = state.borrow_mut();
                    if let Err(e) = st.service.start_indexing() {
                        st.status = format!("Failed to start indexing: {e}");
                        reset_to_initial(
                            &title,
                            &subtitle,
                            &index_btn,
                            &settings_btn,
                            &progress_bar,
                            &status_label,
                        );
                        return;
                    }
                }

                // 4. Poll progress until indexing completes
                let state_poll = state.clone();
                let view_stack = view_stack.clone();
                let title_f = title.clone();
                let subtitle_f = subtitle.clone();
                let index_btn_f = index_btn.clone();
                let settings_btn_f = settings_btn.clone();
                let progress_f = progress_bar.clone();
                let status_f = status_label.clone();

                glib::timeout_add_local(Duration::from_millis(200), move || {
                    let st = state_poll.borrow();

                    if !st.service.is_indexing() {
                        // Indexing completed — switch to main view
                        drop(st);
                        let mut st = state_poll.borrow_mut();
                        st.last_job_progress = st.service.current_progress();
                        st.refresh_statistics();
                        drop(st);

                        reset_to_initial(
                            &title_f,
                            &subtitle_f,
                            &index_btn_f,
                            &settings_btn_f,
                            &progress_f,
                            &status_f,
                        );
                        view_stack.set_visible_child_name("main");
                        return glib::ControlFlow::Break;
                    }

                    // Show progress
                    if let Some(progress) = st.service.current_progress() {
                        progress_f.pulse();
                        status_f
                            .set_text(&format!("Processed: {} files", progress.files_processed));
                    }

                    glib::ControlFlow::Continue
                });
            });
        });
    }

    // ── Settings button handler ────────────────────────────────
    {
        let state = state.clone();
        let view_stack = view_stack.clone();
        let parent_window = parent_window.clone();

        settings_btn.connect_clicked(move |_| {
            crate::ui::settings::show_settings(
                &parent_window,
                &state,
                Some(crate::ui::settings::SettingsPage::IndexedFolders),
                view_stack.clone(),
            );
        });
    }

    container
}

/// Resets all widgets to the initial (non-indexing) state.
fn reset_to_initial(
    title: &Label,
    subtitle: &Label,
    index_btn: &Button,
    settings_btn: &Button,
    progress_bar: &ProgressBar,
    status_label: &Label,
) {
    title.set_text("Welcome to GlintIndex");
    subtitle.set_text("Choose a folder to index to start searching your files.");
    index_btn.set_label("Index Folder…");
    index_btn.set_sensitive(true);
    settings_btn.set_sensitive(true);
    progress_bar.set_visible(false);
    progress_bar.set_fraction(0.0);
    status_label.set_text("");
}
