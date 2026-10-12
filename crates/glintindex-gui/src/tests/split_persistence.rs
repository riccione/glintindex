//! Sidebar-width persistence tests.
//!
//! The results panel restores its width in pixels from config and only
//! persists when the user actually drags — window resizes and startup
//! viewport settling must never rewrite the saved width.

use eframe::egui::{self, Pos2, Rect};

use crate::app::{GlintApp, View};

use super::setup_test_service;

/// Runs `n` frames on one context, switching viewport width per call.
///
/// `RawInput.screen_rect` drives `viewport_rect` (the headless default
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

/// Builds a main-view app with the given persisted sidebar width.
fn app_with_width(width: u32) -> (GlintApp, tempfile::TempDir) {
    let (mut service, tmp) = setup_test_service();
    service.set_main_split_width(width).unwrap();
    let mut app = GlintApp::with_service(service);
    app.view = View::Main;
    (app, tmp)
}

/// Builds a main-view app with only the legacy ratio set (width unset).
fn app_with_legacy_ratio(ratio: f32) -> (GlintApp, tempfile::TempDir) {
    let (mut service, tmp) = setup_test_service();
    service.set_main_split_ratio(ratio).unwrap();
    let mut app = GlintApp::with_service(service);
    app.view = View::Main;
    (app, tmp)
}

/// Simulates a user drag by injecting a panel state of `px` width.
fn inject_panel_width(ctx: &egui::Context, px: f32) {
    ctx.data_mut(|d| {
        d.insert_persisted(
            egui::Id::new("results_panel"),
            egui::PanelState {
                outer_rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(px, 600.0)),
            },
        );
    });
}

#[test]
fn startup_at_wide_viewport_does_not_rewrite_legacy_ratio() {
    let (mut app, _tmp) = app_with_legacy_ratio(0.30);
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1920.0, 3);
    assert_eq!(
        app.service.config().main_split_width,
        0,
        "startup must not write a pixel width"
    );
    assert_eq!(
        app.service.config().main_split_ratio,
        30,
        "startup must not rewrite the legacy ratio"
    );
}

#[test]
fn viewport_change_does_not_rewrite_width() {
    // Regression: a window-size change between the first frames used to
    // write a corrupted ratio (config went 15 -> 18 on one launch).
    let (mut app, _tmp) = app_with_width(400);
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1200.0, 1);
    run_frames(&mut app, &ctx, 1000.0, 2);
    assert_eq!(
        app.service.config().main_split_width,
        400,
        "viewport settling must not rewrite the saved width"
    );
}

#[test]
fn drag_updates_stored_width() {
    let (mut app, _tmp) = app_with_width(400);
    let ctx = egui::Context::default();
    // First frame only records the baseline (viewport 1000, width 400).
    run_frames(&mut app, &ctx, 1000.0, 1);
    assert_eq!(app.service.config().main_split_width(), 400);

    // Simulate a user drag: the panel state now says 560px wide.
    inject_panel_width(&ctx, 560.0);
    run_frames(&mut app, &ctx, 1000.0, 1);
    assert_eq!(
        app.service.config().main_split_width,
        560,
        "a dragged panel must persist its new pixel width"
    );
    assert_eq!(
        app.service.config().main_split_ratio,
        56,
        "legacy ratio stays in sync for older builds"
    );
}

#[test]
fn window_shrink_does_not_clobber_saved_width() {
    let (mut app, _tmp) = app_with_width(700);
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1000.0, 1);
    // The window narrows below the saved width: the panel displays
    // clamped to the viewport, but the saved width must survive.
    run_frames(&mut app, &ctx, 650.0, 2);
    assert_eq!(
        app.service.config().main_split_width,
        700,
        "window resizes must not clobber the saved width"
    );
}

#[test]
fn legacy_ratio_restores_as_pixels() {
    let (mut app, _tmp) = app_with_legacy_ratio(0.57);
    assert_eq!(
        app.service.config().main_split_width(),
        570,
        "GTK-era ratio 57 restores as 570px"
    );
    let ctx = egui::Context::default();
    run_frames(&mut app, &ctx, 1000.0, 3);
    assert_eq!(app.service.config().main_split_ratio, 57);
    assert_eq!(app.service.config().main_split_width, 0);
}
