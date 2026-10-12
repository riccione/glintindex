//! Application toolbar.
//!
//! Top panel with the settings button and the debounced search entry.
//! The search `TextEdit` is bound to `app.search_text`; changes funnel
//! through [`GlintApp::on_query_changed`] (pagination reset + debounce)
//! and Enter dispatches immediately, bypassing the debounce.

use eframe::egui::{self, Key};

use crate::app::{GlintApp, SettingsPage, View};

/// Draws the top toolbar panel.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Settings").clicked() {
                app.view = View::Settings;
                app.settings_page = SettingsPage::General;
            }

            ui.add_space(8.0);

            let response = ui.add(
                egui::TextEdit::singleline(&mut app.search_text)
                    .hint_text("Search files…")
                    .desired_width(f32::INFINITY),
            );

            // Enter: immediate search, bypassing the debounce window.
            if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                app.on_query_changed();
                app.dispatch_search();
            }
        });

        // Detect edits made this frame (typing, paste, clear button).
        app.on_query_changed();
    });
}
