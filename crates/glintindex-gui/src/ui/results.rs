//! Results list panel.
//!
//! Left resizable panel listing search-result file names (the full
//! path lives in the preview header). Row clicks select a result and
//! load its preview; the panel width feeds the persisted split ratio.

use eframe::egui::{self, Label, Sense};

use crate::app::GlintApp;

/// Fallback basis for restoring the split ratio when the viewport
/// width is unknown (0). Normally the real viewport width is used so
/// the stored ratio round-trips against the same denominator that
/// persisting uses.
const REFERENCE_WIDTH: f32 = 1000.0;

/// Draws the results side panel.
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    let ratio = app.service.config().main_split_ratio_f32();
    let viewport_w = ui.ctx().viewport_rect().width();
    let basis_w = if viewport_w > 0.0 {
        viewport_w
    } else {
        REFERENCE_WIDTH
    };
    let mut new_selection = None;

    let panel = egui::Panel::left("results_panel")
        .resizable(true)
        .default_size(basis_w * ratio)
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

    persist_split_ratio(app, ui, response.response.rect.width());
}

/// Persists the panel width as a config split ratio when it settles.
///
/// The first frame only records the ratio: startup must never rewrite
/// `main_split_ratio`. Writing back `width / viewport` on launch used
/// to corrupt the stored value whenever the window was not
/// `REFERENCE_WIDTH` wide (the restore side hardcoded 1000 while this
/// measured the real viewport), decaying the ratio to its floor.
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
    match app.last_measured_ratio {
        None => app.last_measured_ratio = Some(rounded),
        Some(prev) if prev != rounded => {
            let _ = app.service.set_main_split_ratio(rounded);
            app.last_measured_ratio = Some(rounded);
        }
        Some(_) => {}
    }
}
