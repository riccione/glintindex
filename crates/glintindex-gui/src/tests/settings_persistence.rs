//! Settings persistence tests.
//!
//! Covers config writes/reads for theme, font size, index-on-startup,
//! folders, and ignored folders — mostly service-level, plus the
//! settings-view wiring that persists changes and the font-scaled
//! sidebar width.

use eframe::egui::{self, Color32, TextStyle};
use glintindex_core::Theme;

use super::{run_ui, setup_test_app, setup_test_service};

/// Runs `frames` headless frames on a fresh context and returns the
/// persisted settings-nav panel width together with the context (for
/// follow-up font/text measurements against the same fonts).
fn settings_nav_width(app: &mut crate::app::GlintApp, frames: usize) -> (f32, egui::Context) {
    let ctx = egui::Context::default();
    for _ in 0..frames {
        let out = ctx.run_ui(egui::RawInput::default(), |ui| app.ui(ui));
        out.drop_without_applying_deltas();
    }
    let width = egui::PanelState::load(&ctx, egui::Id::new("settings_nav"))
        .expect("settings nav panel state persisted")
        .size()
        .x;
    (width, ctx)
}

/// No-wrap width of the longest settings page label at the context's
/// active Body font.
fn longest_label_width(ctx: &egui::Context) -> f32 {
    let theme = match ctx.system_theme() {
        Some(egui::Theme::Dark) => egui::Theme::Dark,
        _ => egui::Theme::Light,
    };
    let font_id = ctx
        .style_of(theme)
        .text_styles
        .get(&TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(14.0));
    crate::app::SettingsPage::ALL
        .iter()
        .map(|page| {
            ctx.fonts_mut(|fonts| {
                fonts.layout_no_wrap(page.label().to_owned(), font_id.clone(), Color32::WHITE)
            })
            .size()
            .x
        })
        .fold(0.0, f32::max)
}

#[test]
fn test_theme_persists() {
    let (mut service, _tmp) = setup_test_service();
    service.set_theme(Theme::Dark).unwrap();
    assert_eq!(service.config().theme, Theme::Dark);
    service.set_theme(Theme::Light).unwrap();
    assert_eq!(service.config().theme, Theme::Light);
}

#[test]
fn test_font_size_persists_and_roundtrips() {
    let (mut service, _tmp) = setup_test_service();
    service.set_font_size(18).unwrap();
    assert_eq!(service.config().clamped_font_size(), 18);
}

#[test]
fn test_index_on_startup_roundtrip() {
    let (mut service, _tmp) = setup_test_service();
    service.set_index_on_startup(true).unwrap();
    assert!(service.config().index_on_startup);
    service.set_index_on_startup(false).unwrap();
    assert!(!service.config().index_on_startup);
}

#[test]
fn test_folder_add_enable_disable_remove() {
    let (mut service, tmp) = setup_test_service();
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();

    service.add_folder(&folder).unwrap();
    assert_eq!(service.indexed_folders().len(), 1);
    assert!(service.has_enabled_folders());

    service.disable_folder(&folder).unwrap();
    assert!(!service.enabled_folders().iter().any(|f| f.path == folder));

    service.enable_folder(&folder).unwrap();
    assert!(service.enabled_folders().iter().any(|f| f.path == folder));

    service.remove_folder(&folder).unwrap();
    assert!(service.indexed_folders().is_empty());
    assert!(!service.has_enabled_folders());
}

#[test]
fn test_ignored_folder_add_remove() {
    let (mut service, _tmp) = setup_test_service();
    service.add_ignored_folder("node_modules".into()).unwrap();
    assert!(
        service
            .ignored_folders()
            .contains(&"node_modules".to_string())
    );
    service.remove_ignored_folder("node_modules").unwrap();
    assert!(
        !service
            .ignored_folders()
            .contains(&"node_modules".to_string())
    );
}

#[test]
fn test_config_survives_reload() {
    let (mut service, tmp) = setup_test_service();
    service.set_theme(Theme::Dark).unwrap();
    service.set_font_size(20).unwrap();
    let config_path = tmp.path().join("config.toml");

    let reloaded = glintindex_core::config::loader::load(&config_path).unwrap();
    assert_eq!(reloaded.theme, Theme::Dark);
    assert_eq!(reloaded.clamped_font_size(), 20);
}

#[test]
fn test_settings_appearance_click_persists_theme() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;
    app.settings_page = crate::app::SettingsPage::Appearance;

    // Simulate what the Appearance page does on Dark click.
    app.service.set_theme(Theme::Dark).unwrap();
    app.applied_theme = None;

    run_ui(&mut app, 2);
    assert_eq!(app.service.config().theme, Theme::Dark);
}

#[test]
fn test_settings_view_navigates_pages() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;
    for page in crate::app::SettingsPage::ALL {
        app.settings_page = page;
        run_ui(&mut app, 1);
        assert_eq!(app.settings_page, page);
    }
}

#[test]
fn test_leave_settings_returns_to_main_or_empty() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;

    // No folders → empty view.
    app.leave_settings();
    assert_eq!(app.view, crate::app::View::Empty);

    // With folders → main view.
    let folder = std::env::temp_dir().join("glintindex-test-folders");
    std::fs::create_dir_all(&folder).unwrap();
    app.service.add_folder(&folder).unwrap();
    app.view = crate::app::View::Settings;
    app.leave_settings();
    assert_eq!(app.view, crate::app::View::Main);
    let _ = app.service.remove_folder(&folder);
}

#[test]
fn test_settings_nav_fits_longest_label_at_max_font() {
    let (mut app, _tmp) = setup_test_app();
    app.service.set_font_size(32).unwrap();
    app.applied_theme = None;
    app.view = crate::app::View::Settings;

    // Several frames so the theme applied in frame 1 is visible to the
    // root Ui style from frame 2 onward.
    let (width, ctx) = settings_nav_width(&mut app, 3);

    let longest = longest_label_width(&ctx);
    assert!(
        longest > 100.0,
        "measurement sanity: longest label at font 32 is substantial (got {longest})"
    );
    let row_padding = ctx.style_of(egui::Theme::Light).spacing.button_padding.x * 2.0;
    assert!(
        width >= longest + row_padding,
        "nav width {width} must fit longest label {longest} + row padding {row_padding} at font 32"
    );
    assert!(width <= 420.0, "nav width stays within the clamp");
}

#[test]
fn test_settings_nav_width_scales_with_font_size() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;

    app.service.set_font_size(8).unwrap();
    app.applied_theme = None;
    let (small, _) = settings_nav_width(&mut app, 3);

    app.service.set_font_size(32).unwrap();
    app.applied_theme = None;
    let (large, _) = settings_nav_width(&mut app, 3);

    assert!(
        (140.0..=420.0).contains(&small),
        "font 8 width in bounds (got {small})"
    );
    assert!(
        (140.0..=420.0).contains(&large),
        "font 32 width in bounds (got {large})"
    );
    assert!(
        large > small,
        "nav width grows with font size ({small} -> {large})"
    );
}

#[test]
fn test_font_size_staged_until_commit() {
    let (mut app, _tmp) = setup_test_app();
    // Render one frame so sync_theme marks the current font as applied.
    run_ui(&mut app, 1);
    let original = app.service.config().clamped_font_size();
    assert!(app.applied_theme.is_some());

    app.stage_font_size(20);
    assert_eq!(app.font_size_draft, Some(20), "draft is staged");
    assert_eq!(
        app.service.config().clamped_font_size(),
        original,
        "staging must not write config"
    );
    assert!(
        app.applied_theme.is_some(),
        "staging must not invalidate the applied theme"
    );

    app.commit_font_size();
    assert_eq!(app.service.config().clamped_font_size(), 20);
    assert_eq!(app.font_size_draft, None, "commit consumes the draft");
    assert!(
        app.applied_theme.is_none(),
        "commit invalidates applied theme so fonts re-apply"
    );
}

#[test]
fn test_font_size_commit_unchanged_value_is_noop() {
    let (mut app, _tmp) = setup_test_app();
    run_ui(&mut app, 1);
    let current = app.service.config().clamped_font_size();
    assert!(app.applied_theme.is_some());

    // Clicking the value box and clicking away without editing.
    app.stage_font_size(current as i32);
    app.commit_font_size();

    assert_eq!(app.font_size_draft, None);
    assert!(
        app.applied_theme.is_some(),
        "committing an unchanged value must not force a font re-apply"
    );
}

#[test]
fn test_font_size_discard_reverts_staged_value() {
    let (mut app, _tmp) = setup_test_app();
    let original = app.service.config().clamped_font_size();

    app.stage_font_size(20);
    app.discard_font_size(); // Escape path: drop before leave_settings.
    assert_eq!(app.font_size_draft, None);

    app.leave_settings();
    assert_eq!(
        app.service.config().clamped_font_size(),
        original,
        "discarded draft must never reach config"
    );
}

#[test]
fn test_leave_settings_commits_pending_font_size() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;

    // Back button / nav click: leaving counts as "clicked outside".
    app.stage_font_size(20);
    app.leave_settings();

    assert_eq!(app.service.config().clamped_font_size(), 20);
    assert_eq!(app.font_size_draft, None);
}

#[test]
fn test_font_size_draft_self_heals_on_appearance_render() {
    let (mut app, _tmp) = setup_test_app();
    app.view = crate::app::View::Settings;
    app.settings_page = crate::app::SettingsPage::Appearance;

    // A draft left over from an interaction that ended without a
    // release/blur event applies on the next appearance render.
    app.stage_font_size(20);
    run_ui(&mut app, 1);

    assert_eq!(app.service.config().clamped_font_size(), 20);
    assert_eq!(app.font_size_draft, None);
    assert!(app.applied_theme.is_none());
}
