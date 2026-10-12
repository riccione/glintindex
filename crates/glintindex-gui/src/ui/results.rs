//! Results list panel.
//!
//! Left resizable panel listing search-result file names (the full
//! path lives in the preview header). Row clicks select a result and
//! load its preview; the panel width (pixels) feeds the persisted
//! config setting.

use eframe::egui::{self, Label, Sense};

use crate::app::GlintApp;

/// Draws the results side panel.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    let saved_width = app.service.config().main_split_width();
    let mut new_selection = None;

    let panel = egui::Panel::left("results_panel")
        .resizable(true)
        .default_size(saved_width as f32)
        .size_range(egui::Rangef::new(120.0, 900.0));

    let response = panel.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("results_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, result) in app.results.iter().enumerate() {
                    let selected = app.selected_index == Some(index);
                    // Full-row selection background, painted before the
                    // label so the text draws on top. Response::highlight
                    // only underlines Labels, which reads as "no change".
                    if selected {
                        let rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(
                                ui.available_width(),
                                ui.text_style_height(&egui::TextStyle::Body),
                            ),
                        );
                        ui.painter()
                            .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
                    }
                    // selectable(false): rows are list items, not text —
                    // keeps egui's label text-selection from showing an
                    // I-beam cursor over them (and lets drag scroll).
                    let label = Label::new(result.document.filename())
                        .truncate()
                        .selectable(false);
                    let response = ui.add(label.sense(Sense::click()));
                    if response.clicked() {
                        new_selection = Some(index);
                    }
                }
            });
    });

    if let Some(index) = new_selection {
        app.select_result(index);
    }

    persist_sidebar_width(app, ui, response.response.rect.width());
}

/// Persists the panel width (pixels) when the user actually drags.
///
/// The first frame only records (width, viewport): startup must never
/// rewrite the config. After that, a write happens only when the panel
/// width changed while the viewport stayed the same — window resizes
/// and startup viewport settling are denominator changes, not user
/// drags, and used to corrupt the stored value.
fn persist_sidebar_width(app: &mut GlintApp, ui: &egui::Ui, width: f32) {
    if width <= 0.0 {
        return;
    }
    let viewport = ui.ctx().viewport_rect().width();
    if viewport <= 0.0 {
        return;
    }
    let viewport_unchanged = app.last_measured_viewport == Some(viewport);
    if viewport_unchanged && app.last_measured_width != Some(width) {
        let _ = app
            .service
            .set_main_split_width(width.round().clamp(120.0, 900.0) as u32);
    }
    app.last_measured_width = Some(width);
    app.last_measured_viewport = Some(viewport);
}
