//! Empty state view.
//!
//! Shown when no indexed folders are configured: welcome text, a
//! primary "Index Folder…" action (native picker + background job),
//! and a shortcut into settings. While the first index runs, the
//! panel shows progress and polls via [`GlintApp`]'s `PollKind::EmptyIndex`.

use eframe::egui;

use crate::app::{GlintApp, SettingsPage, View};
use crate::ui::settings;

/// Draws the empty/welcome view.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.25);

            if app.empty_indexing {
                ui.heading("Creating your search index…");
                ui.weak("This may take several minutes.");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    if let Some(progress) = app.service.current_progress() {
                        ui.weak(format!("Processed: {} files", progress.files_processed));
                    }
                });
            } else {
                ui.heading("Welcome to GlintIndex");
                ui.weak("Choose a folder to index to start searching your files.");
                ui.add_space(16.0);

                if ui.add(egui::Button::new("Index Folder…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Folder to Index")
                        .pick_folder()
                    {
                        app.add_folder_and_index(&path);
                    }
                }

                ui.add_space(8.0);
                ui.weak("or");
                ui.add_space(8.0);

                if ui.button("Open Settings").clicked() {
                    settings::open_at(app, SettingsPage::IndexedFolders);
                }
            }

            if let Some(error) = &app.empty_error {
                ui.add_space(12.0);
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
    });

    // A folder may have been added via settings while this view was up.
    if !app.empty_indexing && app.view == View::Empty && app.service.has_enabled_folders() {
        app.view = View::Main;
    }
}
