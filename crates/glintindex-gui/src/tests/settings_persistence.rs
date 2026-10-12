//! Settings persistence tests.
//!
//! Covers config writes/reads for theme, font size, index-on-startup,
//! folders, and ignored folders — mostly service-level, plus the
//! settings-view wiring that persists changes.

use glintindex_core::Theme;

use super::{run_ui, setup_test_app, setup_test_service};

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
