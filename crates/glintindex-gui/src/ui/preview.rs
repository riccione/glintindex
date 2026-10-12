//! Preview pane.
//!
//! Central panel with action buttons (Open / Open Folder / Copy Path)
//! above a scrollable monospace preview. The preview renders the
//! toolkit-agnostic [`crate::preview_model`] as an egui `LayoutJob`:
//! dim gray italics for `//` headers and `…` gaps, warm-yellow
//! background for search matches.

use std::ops::Range;

use eframe::egui::{
    self, FontId,
    text::{LayoutJob, TextFormat},
};

use crate::app::GlintApp;
use crate::preview_model::{LineStyle, PreviewModel};
use crate::theme::MATCH_BACKGROUND;

/// Placeholder shown when nothing is selected.
pub const PLACEHOLDER_TEXT: &str = "Select a file to preview";

/// Draws the preview panel (action bar + scrollable preview).
pub fn show(app: &mut GlintApp, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        let selected_path = app.selected_path.clone();
        let file_exists = selected_path.as_ref().is_some_and(|p| p.exists());

        ui.horizontal(|ui| {
            let open = ui.add_enabled(file_exists, egui::Button::new("Open"));
            if open.clicked() {
                if let Some(path) = &selected_path {
                    if let Err(e) = crate::file_actions::open_file(path) {
                        tracing::warn!(
                            target: "glintindex::gui",
                            path = %path.display(),
                            error = %e,
                            "open file failed"
                        );
                    }
                }
            }

            let reveal = ui.add_enabled(selected_path.is_some(), egui::Button::new("Open Folder"));
            if reveal.clicked() {
                if let Some(path) = &selected_path {
                    if let Err(e) = crate::file_actions::reveal_in_file_manager(path) {
                        tracing::warn!(
                            target: "glintindex::gui",
                            path = %path.display(),
                            error = %e,
                            "reveal in file manager failed"
                        );
                    }
                }
            }

            let copy = ui.add_enabled(selected_path.is_some(), egui::Button::new("Copy Path"));
            if copy.clicked() {
                if let Some(path) = &selected_path {
                    ui.ctx().copy_text(path.display().to_string());
                }
            }
        });

        ui.separator();

        let font_size = app.service.config().clamped_font_size() as f32;
        egui::ScrollArea::vertical()
            .id_salt("preview_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match &app.preview_model {
                Some(model) => {
                    let job = to_layout_job(model, font_size, ui.visuals().clone());
                    ui.add(egui::Label::new(job).selectable(true));
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.weak(PLACEHOLDER_TEXT);
                    });
                }
            });
    });
}

/// Converts a [`PreviewModel`] into a single `LayoutJob`.
///
/// Dim lines (headers, gaps) render gray and italic; content lines
/// render monospace with warm-yellow backgrounds over match ranges
/// (byte offsets clamped to char boundaries).
pub fn to_layout_job(model: &PreviewModel, font_size: f32, visuals: egui::Visuals) -> LayoutJob {
    let mono = FontId::monospace(font_size);
    let normal = TextFormat {
        font_id: mono.clone(),
        color: visuals.text_color(),
        ..Default::default()
    };
    let dim = TextFormat {
        font_id: mono.clone(),
        color: visuals.weak_text_color(),
        italics: true,
        ..Default::default()
    };
    let matched = TextFormat {
        font_id: mono.clone(),
        color: visuals.text_color(),
        background: MATCH_BACKGROUND,
        ..Default::default()
    };

    let mut job = LayoutJob::default();
    for line in &model.lines {
        match line.style {
            LineStyle::Dim => {
                job.append(&line.number_prefix, 0.0, dim.clone());
                job.append(&line.text, 0.0, dim.clone());
            }
            LineStyle::Normal => {
                job.append(&line.number_prefix, 0.0, normal.clone());
                append_with_matches(&mut job, &line.text, &line.match_ranges, &normal, &matched);
            }
        }
        job.append("\n", 0.0, normal.clone());
    }
    job
}

/// Appends line text with match ranges styled as highlights.
fn append_with_matches(
    job: &mut LayoutJob,
    text: &str,
    ranges: &[Range<usize>],
    normal: &TextFormat,
    matched: &TextFormat,
) {
    let mut last = 0usize;
    for range in ranges {
        let start = clamp_char_boundary(text, range.start).max(last);
        let end = clamp_char_boundary(text, range.end).max(start);
        if start < last {
            continue;
        }
        job.append(&text[last..start], 0.0, normal.clone());
        job.append(&text[start..end], 0.0, matched.clone());
        last = end;
    }
    job.append(&text[last..], 0.0, normal.clone());
}

/// Clamps a byte offset to the nearest valid char boundary at or below it.
fn clamp_char_boundary(text: &str, byte: usize) -> usize {
    let mut b = byte.min(text.len());
    while b > 0 && !text.is_char_boundary(b) {
        b -= 1;
    }
    b
}
