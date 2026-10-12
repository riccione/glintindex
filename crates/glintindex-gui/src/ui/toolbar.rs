//! Application toolbar.
//!
//! Top panel with the settings button and the debounced search entry.
//! The search `TextEdit` is bound to `app.search_text`; changes funnel
//! through [`GlintApp::on_query_changed`] (pagination reset + debounce)
//! and Enter dispatches immediately, bypassing the debounce. A cross
//! button overlaid on the field's right edge clears the input while
//! text is present.

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

            // Clear button overlaid on the right edge of the field,
            // shown only while there is something to clear.
            if !app.search_text.is_empty() {
                let side = ui.spacing().interact_size.y;
                let rect = egui::Rect::from_center_size(
                    response.rect.right_center()
                        - egui::vec2(side / 2.0 + ui.spacing().item_spacing.x, 0.0),
                    egui::vec2(side, side),
                );
                let clear = ui.put(
                    rect,
                    egui::Button::new(egui::RichText::new("✖")).min_size(egui::vec2(side, side)),
                );
                if clear.clicked() {
                    app.clear_search();
                    response.request_focus();
                }
                clear.on_hover_text("Clear search");
            }

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
