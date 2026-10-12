//! Results list panel.
//!
//! Left resizable panel listing search-result file names (the full
//! path lives in the preview header). Row clicks select a result and
//! load its preview; the panel width feeds the persisted split ratio.

use eframe::egui::{self, Label, Sense};

use crate::app::GlintApp;

/// Width assumed for the main window when converting the persisted
/// split ratio to a panel width (matches the GTK default of 1000).
const REFERENCE_WIDTH: f32 = 1000.0;

/// Draws the results side panel.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    let ratio = app.service.config().main_split_ratio_f32();
    let mut new_selection = None;

    let panel = egui::Panel::left("results_panel")
        .resizable(true)
        .default_size((REFERENCE_WIDTH * ratio).clamp(150.0, 850.0))
        .size_range(egui::Rangef::new(120.0, 900.0));

    let response = panel.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("results_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, result) in app.results.iter().enumerate() {
                    let selected = app.selected_index == Some(index);
                    // selectable(false): rows are list items, not text —
                    // keeps egui's label text-selection from showing an
                    // I-beam cursor over them (and lets drag scroll).
                    let label = Label::new(result.document.filename())
                        .truncate()
                        .selectable(false);
                    let response = ui.add(label.sense(Sense::click()));
                    let response = if selected {
                        response.highlight()
                    } else {
                        response
                    };
                    if response.clicked() {
                        new_selection = Some(index);
                    }
                }
            });
    });

    if let Some(index) = new_selection {
        app.select_result(index);
    }

    persist_split_ratio(app, ui, response.response.rect.width());
}

/// Persists the panel width as a config split ratio when it settles.
fn persist_split_ratio(app: &mut GlintApp, ui: &egui::Ui, width: f32) {
    if width <= 0.0 {
        return;
    }
    let total = ui.ctx().viewport_rect().width();
    if total <= 0.0 {
        return;
    }
    let ratio = (width / total).clamp(0.15, 0.85);
    let rounded = (ratio * 100.0).round() / 100.0;
    if app.last_persisted_ratio != Some(rounded) {
        let _ = app.service.set_main_split_ratio(rounded);
        app.last_persisted_ratio = Some(rounded);
    }
}
