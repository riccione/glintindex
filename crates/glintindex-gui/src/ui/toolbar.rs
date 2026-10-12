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

            // Clear icon overlaid on the right edge of the field, shown
            // only while there is something to clear. Anchored inside the
            // field's inner margin (TextEdit default 4/2) and drawn with
            // the painter so fallback-font metrics (NotoEmoji ✖ is 1.27em
            // wide, 1.36em tall) cannot push it outside the field.
            if !app.search_text.is_empty() {
                let field = response.rect;
                let side = (field.height() - 6.0).max(14.0);
                let rect = egui::Rect::from_min_size(
                    egui::pos2(field.right() - 4.0 - side, field.center().y - side / 2.0),
                    egui::vec2(side, side),
                );
                let clear = ui
                    .allocate_rect(rect, egui::Sense::click())
                    .on_hover_text("Clear search")
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if clear.hovered() || clear.is_pointer_button_down_on() {
                    ui.painter()
                        .rect_filled(rect, 3.0, ui.visuals().faint_bg_color);
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "✖",
                    egui::FontId::proportional(side * 0.7),
                    ui.visuals().text_color(),
                );
                if clear.clicked() {
                    app.clear_search();
                    response.request_focus();
                }
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
