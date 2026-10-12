//! Status bar panel.
//!
//! Bottom panel: result stats on the left, pagination controls on the
//! right (Previous / Page X of Y / Next).

use eframe::egui::{self, Align, Layout};

use crate::app::GlintApp;

/// Draws the bottom status bar.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    egui::Panel::bottom("status_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.weak(&app.status);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let next = ui.add_enabled(
                    app.current_page < app.total_pages,
                    egui::Button::new("Next"),
                );
                if next.clicked() {
                    app.change_page(1);
                }

                ui.label(format!("Page {} of {}", app.current_page, app.total_pages));

                let prev = ui.add_enabled(app.current_page > 1, egui::Button::new("Previous"));
                if prev.clicked() {
                    app.change_page(-1);
                }
            });
        });
    });
}
