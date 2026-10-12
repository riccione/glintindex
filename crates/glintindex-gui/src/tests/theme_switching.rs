//! Theme switching tests.
//!
//! Verifies egui `Visuals`/font mapping for Light/Dark/System and the
//! settings page enumeration.

use eframe::egui::{self, FontFamily, TextStyle};
use glintindex_core::Theme;

use crate::app::{GlintApp, SettingsPage, View};
use crate::theme::apply_theme;

use super::{run_ui, setup_test_app, test_context};

#[test]
fn test_apply_theme_light_sets_light_visuals() {
    let ctx = test_context();
    apply_theme(&ctx, Theme::Light, 14);
    assert!(
        !ctx.style_of(egui::Theme::Light).visuals.dark_mode,
        "light visuals applied"
    );
}

#[test]
fn test_apply_theme_dark_sets_dark_visuals() {
    let ctx = test_context();
    apply_theme(&ctx, Theme::Dark, 14);
    assert!(
        ctx.style_of(egui::Theme::Dark).visuals.dark_mode,
        "dark visuals applied"
    );
}

#[test]
fn test_apply_theme_system_resolves_without_panic() {
    let ctx = test_context();
    // Headless context has no system theme — resolves to light.
    apply_theme(&ctx, Theme::System, 14);
    let dark = ctx.style_of(egui::Theme::Light).visuals.dark_mode;
    assert!(!dark, "unknown system theme falls back to light");
}

#[test]
fn test_font_size_applied_to_text_styles() {
    let ctx = test_context();
    apply_theme(&ctx, Theme::Light, 20);
    let style = ctx.style_of(egui::Theme::Light);
    let body = style.text_styles.get(&TextStyle::Body).unwrap();
    assert_eq!(body.size, 20.0);
    assert_eq!(body.family, FontFamily::Proportional);
    let mono = style.text_styles.get(&TextStyle::Monospace).unwrap();
    assert_eq!(mono.family, FontFamily::Monospace);
}

#[test]
fn test_settings_page_labels() {
    let labels: Vec<&str> = SettingsPage::ALL.iter().map(|p| p.label()).collect();
    assert_eq!(
        labels,
        vec![
            "General",
            "Appearance",
            "Indexed Folders",
            "Ignored Folders",
            "Index",
            "About"
        ]
    );
}

#[test]
fn test_initial_view_empty_without_folders() {
    let (app, _tmp) = setup_test_app();
    assert_eq!(app.view, View::Empty, "no folders → empty view");
}

#[test]
fn test_initial_view_main_with_folders() {
    let (mut app, tmp) = setup_test_app();
    let folder = tmp.path().join("docs");
    std::fs::create_dir_all(&folder).unwrap();
    app.service.add_folder(&folder).unwrap();

    let app = GlintApp::with_service(app.service);
    assert_eq!(app.view, View::Main, "configured folders → main view");
}

#[test]
fn test_theme_change_via_settings_marks_reapply() {
    let (mut app, _tmp) = setup_test_app();
    run_ui(&mut app, 1);
    assert!(
        app.applied_theme.is_some(),
        "theme applied during first frame"
    );

    // Simulate the Appearance page clicking Dark.
    app.service.set_theme(Theme::Dark).unwrap();
    app.applied_theme = None;
    run_ui(&mut app, 1);
    assert_eq!(
        app.applied_theme,
        Some((Theme::Dark, app.service.config().clamped_font_size()))
    );
}
