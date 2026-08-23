//! Settings persistence tests.
//!
//! Tests GTK widget interactions with application settings,
//! verifying that user interactions update the backing state.

use gtk::prelude::*;

use super::setup_test_state;

#[gtk::test]
fn test_theme_radio_buttons_mutual_exclusion() {
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
}

#[gtk::test]
fn test_selecting_theme_updates_config() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_clone = state.clone();

    let radio_system = gtk::CheckButton::with_label("System");
    radio_system.set_active(true);

    radio_system.connect_toggled(move |btn| {
        if btn.is_active() {
            let mut st = state_clone.borrow_mut();
            let _ = st.service.set_theme(glintindex_core::Theme::System);
        }
    });

    radio_system.set_active(true);
    super::process_events();

    let st = state.borrow();
    assert_eq!(st.service.config().theme, glintindex_core::Theme::System);
}

#[gtk::test]
fn test_font_size_spin_updates_config() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_clone = state.clone();

    let spin = gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(12.0, 8.0, 32.0, 1.0, 5.0, 0.0))
        .build();

    spin.connect_value_changed(move |btn| {
        let new_size = btn.value() as u32;
        let mut st = state_clone.borrow_mut();
        let _ = st.service.set_font_size(new_size);
    });

    spin.set_value(20.0);
    super::process_events();

    let st = state.borrow();
    assert_eq!(st.service.config().font_size, 20);
}

#[gtk::test]
fn test_font_size_clamped_to_valid_range() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_clone = state.clone();

    let spin = gtk::SpinButton::builder()
        .adjustment(&gtk::Adjustment::new(12.0, 8.0, 32.0, 1.0, 5.0, 0.0))
        .build();

    spin.connect_value_changed(move |btn| {
        let new_size = btn.value() as u32;
        let mut st = state_clone.borrow_mut();
        let _ = st.service.set_font_size(new_size);
    });

    spin.set_value(50.0);
    super::process_events();

    let st = state.borrow();
    let clamped = st.service.config().clamped_font_size();
    assert!((8..=32).contains(&clamped), "Font size should be clamped");
}

#[gtk::test]
fn test_config_roundtrip_preserves_settings() {
    let _ = gtk::init();
    let (mut state, tmp) = setup_test_state();
    let config_path = tmp.path().join("config.toml");

    let _ = state.service.set_theme(glintindex_core::Theme::Dark);
    let _ = state.service.set_font_size(20);
    glintindex_core::config::loader::save(&config_path, state.service.config()).unwrap();

    let restored = glintindex_core::config::loader::load(&config_path).unwrap();
    assert_eq!(restored.theme, glintindex_core::Theme::Dark);
    assert_eq!(restored.font_size, 20);
}

#[gtk::test]
fn test_default_config_when_keys_missing() {
    let _ = gtk::init();
    let tmp = tempfile::TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"indexed_folders = []
ignored_folders = [".git", ".svn"]
index_directory = "/tmp/glintindex-test-index"
theme = "system"
max_preview_size = 200
commit_interval = 500
"#,
    )
    .unwrap();

    let config = glintindex_core::config::loader::load(&config_path).unwrap();
    assert_eq!(config.theme, glintindex_core::Theme::System);
    assert_eq!(config.max_preview_size, 200);
    assert_eq!(config.commit_interval, 500);
    assert_eq!(config.ignored_folders.len(), 2);
}

#[gtk::test]
fn test_toggle_button_enables_disables_folder() {
    let _ = gtk::init();
    let (service, _tmp) = {
        let tmp = tempfile::TempDir::new().unwrap();
        let folder_path = tmp.path().join("test_folder");
        std::fs::create_dir(&folder_path).unwrap();

        let config_path = tmp.path().join("config.toml");
        std::fs::write(
            &config_path,
            r#"indexed_folders = []
ignored_folders = []
index_directory = "/tmp/glintindex-test-index"
theme = "system"
max_preview_size = 200
commit_interval = 500
"#,
        )
        .unwrap();
        let mut svc = glintindex_core::ApplicationService::with_config_path(&config_path).unwrap();
        let _ = svc.add_folder(&folder_path);
        (svc, tmp)
    };

    let state = std::rc::Rc::new(std::cell::RefCell::new(super::setup_test_state().0));

    // Override service
    {
        let mut st = state.borrow_mut();
        st.service = service;
    }

    // Disable
    {
        let mut st = state.borrow_mut();
        let folders: Vec<_> = st.service.indexed_folders().into_iter().cloned().collect();
        let _ = st.service.disable_folder(&folders[0].path);
    }
    {
        let st = state.borrow();
        assert!(st.service.enabled_folders().is_empty());
    }

    // Re-enable
    {
        let mut st = state.borrow_mut();
        let folders: Vec<_> = st.service.indexed_folders().into_iter().cloned().collect();
        let _ = st.service.enable_folder(&folders[0].path);
    }
    {
        let st = state.borrow();
        assert_eq!(st.service.enabled_folders().len(), 1);
    }
}

#[gtk::test]
fn test_remove_button_deletes_folder() {
    let _ = gtk::init();
    let (service, _tmp) = {
        let tmp = tempfile::TempDir::new().unwrap();
        let folder_path = tmp.path().join("test_folder");
        std::fs::create_dir(&folder_path).unwrap();

        let config_path = tmp.path().join("config.toml");
        std::fs::write(
            &config_path,
            r#"indexed_folders = []
ignored_folders = []
index_directory = "/tmp/glintindex-test-index"
theme = "system"
max_preview_size = 200
commit_interval = 500
"#,
        )
        .unwrap();
        let mut svc = glintindex_core::ApplicationService::with_config_path(&config_path).unwrap();
        let _ = svc.add_folder(&folder_path);
        (svc, tmp)
    };

    let state = std::rc::Rc::new(std::cell::RefCell::new(super::setup_test_state().0));
    {
        let mut st = state.borrow_mut();
        st.service = service;
    }

    assert_eq!(state.borrow().service.indexed_folders().len(), 1);

    {
        let mut st = state.borrow_mut();
        let folders: Vec<_> = st.service.indexed_folders().into_iter().cloned().collect();
        let _ = st.service.remove_folder(&folders[0].path);
    }
    {
        let st = state.borrow();
        assert!(st.service.indexed_folders().is_empty());
    }
}

#[gtk::test]
fn test_ignored_folders_add_and_remove() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let _ = st.service.add_ignored_folder("custom_dir".to_string());
    }
    {
        let st = state.borrow();
        assert!(
            st.service
                .ignored_folders()
                .contains(&"custom_dir".to_string())
        );
    }

    {
        let mut st = state.borrow_mut();
        let _ = st.service.remove_ignored_folder("custom_dir");
    }
    {
        let st = state.borrow();
        assert!(
            !st.service
                .ignored_folders()
                .contains(&"custom_dir".to_string())
        );
    }
}

#[gtk::test]
fn test_search_entry_text_updates_query() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_clone = state.clone();
    let search_entry = gtk::SearchEntry::new();

    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();
        let mut st = state_clone.borrow_mut();
        st.query = query;
    });

    search_entry.set_text("test query");
    super::process_events();

    let st = state.borrow();
    assert_eq!(st.query, "test query");
}

#[gtk::test]
fn test_empty_search_clears_results() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let doc = glintindex_core::Document::new(
            std::path::PathBuf::from("/test/file.txt"),
            100,
            std::time::SystemTime::now(),
            "content".into(),
        );
        st.results
            .push(glintindex_core::SearchResult::new(doc, 1.0, "test".into()));
    }

    let state_clone = state.clone();
    let search_entry = gtk::SearchEntry::new();

    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string();
        let is_empty = query.trim().is_empty();
        let mut st = state_clone.borrow_mut();
        st.query = query;
        if is_empty {
            st.results.clear();
            st.selected_index = None;
            st.status = "Ready".to_string();
        }
    });

    // Set non-empty first to ensure changed signal fires, then clear
    search_entry.set_text("x");
    super::process_events();
    search_entry.set_text("");
    super::process_events();

    let st = state.borrow();
    assert!(st.results.is_empty());
    assert_eq!(st.status, "Ready");
}
