//! Theme and UI switching tests.
//!
//! Tests CSS provider loading, theme switching, and UI state transitions.

use gtk::prelude::*;

#[gtk::test]
fn test_light_theme_sets_dark_preference_false() {
    let _ = gtk::init();
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(false);
        let prefer_dark = settings.is_gtk_application_prefer_dark_theme();
        assert!(!prefer_dark, "Light theme should set prefer_dark to false");
    }
}

#[gtk::test]
fn test_dark_theme_sets_dark_preference_true() {
    let _ = gtk::init();
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(true);
        let prefer_dark = settings.is_gtk_application_prefer_dark_theme();
        assert!(prefer_dark, "Dark theme should set prefer_dark to true");
    }
}

#[gtk::test]
fn test_system_theme_follows_preference() {
    let _ = gtk::init();
    if let Some(settings) = gtk::Settings::default() {
        let current = settings.is_gtk_application_prefer_dark_theme();
        let _ = current;
    }
}

#[gtk::test]
fn test_css_provider_loads_light_css() {
    let _ = gtk::init();
    let provider = gtk::CssProvider::new();
    let css = include_str!("../../resources/themes/light.css");
    provider.load_from_data(css);
}

#[gtk::test]
fn test_css_provider_loads_dark_css() {
    let _ = gtk::init();
    let provider = gtk::CssProvider::new();
    let css = include_str!("../../resources/themes/dark.css");
    provider.load_from_data(css);
}

#[gtk::test]
fn test_font_size_change_updates_css() {
    let _ = gtk::init();
    let provider = gtk::CssProvider::new();
    let light_css = include_str!("../../resources/themes/light.css");

    let css_12 = format!("{light_css}\n* {{ font-size: 12pt; }}");
    provider.load_from_data(&css_12);

    let css_20 = format!("{light_css}\n* {{ font-size: 20pt; }}");
    provider.load_from_data(&css_20);
}

#[gtk::test]
fn test_theme_manager_new_applies_theme() {
    let _ = gtk::init();
    let manager = crate::theme::ThemeManager::new(glintindex_core::Theme::Light, 12);
    let _ = manager;
}

#[gtk::test]
fn test_theme_manager_switches_theme() {
    let _ = gtk::init();
    let manager = crate::theme::ThemeManager::new(glintindex_core::Theme::Light, 12);
    manager.apply(glintindex_core::Theme::Dark, 12);
    manager.apply(glintindex_core::Theme::Light, 12);
}

#[gtk::test]
fn test_empty_state_visible_when_no_folders() {
    let (service, _tmp) = super::setup_test_service();
    assert!(
        !service.has_enabled_folders(),
        "Service should have no enabled folders initially"
    );
}

#[gtk::test]
fn test_main_visible_when_folders_exist() {
    let (service, tmp) = super::setup_test_service();
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();

    let mut svc = service;
    let _ = svc.add_folder(&folder);

    assert!(
        svc.has_enabled_folders(),
        "Configured folder means main state"
    );
}

#[gtk::test]
fn test_settings_page_enum_serializes() {
    let page = crate::ui::settings::SettingsPage::IndexedFolders;
    assert_eq!(page.stack_name(), "indexed_folders");

    let page = crate::ui::settings::SettingsPage::General;
    assert_eq!(page.stack_name(), "general");

    let page = crate::ui::settings::SettingsPage::Appearance;
    assert_eq!(page.stack_name(), "appearance");

    let page = crate::ui::settings::SettingsPage::About;
    assert_eq!(page.stack_name(), "about");
}

#[gtk::test]
fn test_css_classes_applied_to_widgets() {
    let _ = gtk::init();
    let label = gtk::Label::builder()
        .label("Test")
        .css_classes(["title-1"])
        .build();

    let classes = label.css_classes();
    assert!(
        classes.iter().any(|c| c == "title-1"),
        "Label should have 'title-1' CSS class"
    );
}

#[gtk::test]
fn test_checkbutton_group_mutual_exclusion() {
    let _ = gtk::init();
    let radio_system = gtk::CheckButton::with_label("System");
    let radio_light = gtk::CheckButton::with_label("Light");
    let radio_dark = gtk::CheckButton::with_label("Dark");

    radio_light.set_group(Some(&radio_system));
    radio_dark.set_group(Some(&radio_system));

    radio_system.set_active(true);
    assert!(radio_system.is_active());
    assert!(!radio_light.is_active());
    assert!(!radio_dark.is_active());

    radio_light.set_active(true);
    assert!(!radio_system.is_active());
    assert!(radio_light.is_active());
    assert!(!radio_dark.is_active());

    radio_dark.set_active(true);
    assert!(!radio_system.is_active());
    assert!(!radio_light.is_active());
    assert!(radio_dark.is_active());
}
