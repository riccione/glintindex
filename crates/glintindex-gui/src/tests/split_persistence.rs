//! Split-ratio persistence tests.
//!
//! The results panel restores from `main_split_ratio` relative to the
//! real viewport and must never rewrite the config on startup —
//! writing `width / viewport` against a hardcoded 1000pt restore basis
//! used to decay the stored ratio to its clamp floor across launches.

use eframe::egui::{self, Pos2, Rect};

use crate::app::{GlintApp, View};

use super::setup_test_service;

/// Runs `n` headless frames on one context at the given viewport width.
///
/// `RawInput.screen_rect` drives `viewport_rect`, which both the
/// restore and the persist side measure against (the headless default
/// viewport is a 10000pt square).
fn run_frames(app: &mut GlintApp, ctx: &egui::Context, screen_width: f32, frames: usize) {
    for _ in 0..frames {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(screen_width, 700.0),
            )),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| app.ui(ui));
        out.drop_without_applying_deltas();
    }
}

/// Builds a main-view app with the given persisted split ratio.
fn app_with_ratio(ratio: f32) -> (GlintApp, tempfile::TempDir) {
    let (mut service, tmp) = setup_test_service();
    service.set_main_split_ratio(ratio).unwrap();
    let mut app = GlintApp::with_service(service);
    app.view = View::Main;
    (app, tmp)
}

#[test]
fn startup_at_wide_viewport_does_not_rewrite_ratio() {
    let (mut app, _tmp) = app_with_ratio(0.30);
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1920.0, 3);
    assert_eq!(
        app.service.config().main_split_ratio,
        30,
        "startup must not rewrite the ratio (the old fixed-1000pt \
         basis wrote 1000*ratio/window_width back on every launch)"
    );
}

#[test]
fn startup_at_reference_viewport_keeps_ratio() {
    let (mut app, _tmp) = app_with_ratio(0.57);
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1000.0, 3);
    assert_eq!(app.service.config().main_split_ratio, 57);
}

#[test]
fn panel_resize_updates_config() {
    let (mut app, _tmp) = app_with_ratio(0.50);
    let ctx = egui::Context::default();
    // First frame only records the baseline ratio.
    run_frames(&mut app, &ctx, 1000.0, 1);
    assert_eq!(app.service.config().main_split_ratio, 50);

    // Simulate a user drag: the panel state now says 400px wide.
    ctx.data_mut(|d| {
        d.insert_persisted(
            egui::Id::new("results_panel"),
            egui::PanelState {
                outer_rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 600.0)),
            },
        );
    });
    run_frames(&mut app, &ctx, 1000.0, 1);
    assert_eq!(
        app.service.config().main_split_ratio,
        40,
        "a resized panel must persist its new ratio"
    );
}
