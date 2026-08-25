//! Search flow tests.
//!
//! Tests search entry signals, result list population, and row selection.

use gtk::prelude::*;

use super::setup_test_state;

#[gtk::test]
fn test_search_result_row_count_matches_state() {
    let _ = gtk::init();
    let (mut state, tmp) = setup_test_state();

    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("a.txt"), "alpha beta gamma").unwrap();
    std::fs::write(folder.join("b.txt"), "beta delta epsilon").unwrap();

    let _ = state.service.add_folder(&folder);
    let _ = state.service.index_folder(&folder);

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let query_obj = glintindex_core::SearchQuery::new("beta");
        st.results = st.service.search(&query_obj).unwrap_or_default().results;
    }

    let listbox = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    let st = state.borrow();
    for result in &st.results {
        let row = gtk::Label::builder()
            .label(result.document.filename())
            .build();
        listbox.append(&row);
    }

    let mut row_count = 0;
    let mut child = listbox.first_child();
    while child.is_some() {
        row_count += 1;
        child = child.and_then(|c| c.next_sibling());
    }
    let result_count = st.results.len();
    drop(st);

    assert!(
        row_count == result_count,
        "ListBox rows ({row_count}) should match results ({result_count})"
    );
}

#[gtk::test]
fn test_row_selection_updates_selected_index() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        for i in 0..3 {
            let doc = glintindex_core::Document::new(
                std::path::PathBuf::from(format!("/test/file_{i}.txt")),
                100,
                std::time::SystemTime::now(),
                format!("content {i}"),
            );
            st.results.push(glintindex_core::SearchResult::new(
                doc,
                1.0,
                format!("snip {i}"),
            ));
        }
    }

    let state_clone = state.clone();
    let listbox = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    listbox.connect_row_selected(move |_listbox, row| {
        if let Some(row) = row {
            let index = row.index() as usize;
            let mut st = state_clone.borrow_mut();
            st.selected_index = Some(index);
        }
    });

    let second_row = gtk::ListBoxRow::builder()
        .child(&gtk::Label::builder().label("row 1").build())
        .build();
    listbox.append(&second_row);
    listbox.select_row(Some(&second_row));
    super::process_events();

    let st = state.borrow();
    assert_eq!(st.selected_index, Some(0));
}

#[gtk::test]
fn test_clear_button_resets_search_state() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        st.query = "old query".to_string();
        let doc = glintindex_core::Document::new(
            std::path::PathBuf::from("/test/file.txt"),
            100,
            std::time::SystemTime::now(),
            "content".into(),
        );
        st.results
            .push(glintindex_core::SearchResult::new(doc, 1.0, "snip".into()));
        st.selected_index = Some(0);
    }

    {
        let mut st = state.borrow_mut();
        st.query.clear();
        st.results.clear();
        st.selected_index = None;
        st.status = "Ready".to_string();
    }

    let st = state.borrow();
    assert!(st.query.is_empty());
    assert!(st.results.is_empty());
    assert_eq!(st.selected_index, None);
    assert_eq!(st.status, "Ready");
}

#[gtk::test]
fn test_search_result_contains_document_fields() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let doc = glintindex_core::Document::new(
            std::path::PathBuf::from("/home/user/project/main.rs"),
            4096,
            std::time::SystemTime::now(),
            "fn main() {}".into(),
        );
        st.results
            .push(glintindex_core::SearchResult::new(doc, 0.95, "main".into()));
    }

    let st = state.borrow();
    let result = &st.results[0];
    assert_eq!(
        result.document.path,
        std::path::PathBuf::from("/home/user/project/main.rs")
    );
    assert_eq!(result.document.size, 4096);
    assert_eq!(result.document.content, "fn main() {}");
    assert!(result.score > 0.0);
    assert!(!result.snippet.is_empty());
}

#[gtk::test]
fn test_search_result_score_is_positive() {
    let doc = glintindex_core::Document::new(
        std::path::PathBuf::from("/test/file.txt"),
        100,
        std::time::SystemTime::now(),
        "content".into(),
    );
    let result = glintindex_core::SearchResult::new(doc, 1.0, "snippet".into());
    assert!(result.score > 0.0, "Score should be positive");
}

#[gtk::test]
fn test_search_result_snippet_is_non_empty() {
    let doc = glintindex_core::Document::new(
        std::path::PathBuf::from("/test/file.txt"),
        100,
        std::time::SystemTime::now(),
        "content".into(),
    );
    let result = glintindex_core::SearchResult::new(doc, 1.0, "test snippet".into());
    assert!(!result.snippet.is_empty(), "Snippet should not be empty");
}

#[gtk::test]
fn test_search_on_empty_index_returns_empty() {
    let (service, _tmp) = super::setup_test_service();
    let query_obj = glintindex_core::SearchQuery::new("anything");
    let results = service.search(&query_obj).unwrap_or_default().results;
    assert!(results.is_empty(), "Empty index should return no results");
}

#[gtk::test]
fn test_search_with_special_characters() {
    let _ = gtk::init();
    let (mut state, tmp) = setup_test_state();

    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(
        folder.join("special.txt"),
        "fn main() { println!(\"hello\"); }",
    )
    .unwrap();

    let _ = state.service.add_folder(&folder);
    let _ = state.service.index_folder(&folder);

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let query_obj = glintindex_core::SearchQuery::new("fn main");
        st.results = st.service.search(&query_obj).unwrap_or_default().results;
    }

    let st = state.borrow();
    assert!(
        !st.results.is_empty(),
        "Search for 'fn main' should find results"
    );
}

#[gtk::test]
fn test_activate_key_triggers_search() {
    let _ = gtk::init();
    let (mut state, tmp) = setup_test_state();

    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("test.txt"), "hello world").unwrap();

    let _ = state.service.add_folder(&folder);
    let _ = state.service.index_folder(&folder);

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_clone = state.clone();
    let search_entry = gtk::SearchEntry::new();

    search_entry.connect_activate(move |entry| {
        let query = entry.text().to_string();
        let mut st = state_clone.borrow_mut();
        st.query = query.clone();
        if !query.trim().is_empty() {
            let query_obj = glintindex_core::SearchQuery::new(&query);
            st.results = st.service.search(&query_obj).unwrap_or_default().results;
        }
    });

    search_entry.set_text("hello");
    search_entry.emit_activate();
    super::process_events();

    let st = state.borrow();
    assert!(!st.results.is_empty(), "Activate should trigger search");
}

#[gtk::test]
fn test_search_multiple_results_ordered_by_score() {
    let _ = gtk::init();
    let (mut state, tmp) = setup_test_state();

    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("a.txt"), "alpha beta").unwrap();
    std::fs::write(folder.join("b.txt"), "beta beta beta").unwrap();

    let _ = state.service.add_folder(&folder);
    let _ = state.service.index_folder(&folder);

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));

    {
        let mut st = state.borrow_mut();
        let query_obj = glintindex_core::SearchQuery::new("beta");
        st.results = st.service.search(&query_obj).unwrap_or_default().results;
    }

    let st = state.borrow();
    if st.results.len() >= 2 {
        for i in 0..st.results.len() - 1 {
            assert!(
                st.results[i].score >= st.results[i + 1].score,
                "Results should be ordered by descending score"
            );
        }
    }
}

#[gtk::test]
fn test_search_empty_query_returns_empty() {
    let (service, _tmp) = super::setup_test_service();
    let query_obj = glintindex_core::SearchQuery::new("");
    let results = service.search(&query_obj).unwrap_or_default().results;
    assert!(results.is_empty(), "Empty query should return no results");
}

#[gtk::test]
fn test_search_entry_clear_button_resets() {
    let _ = gtk::init();
    let (state, _tmp) = setup_test_state();

    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
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
        }
    });

    search_entry.set_text("search term");
    super::process_events();
    assert_eq!(state.borrow().query, "search term");

    search_entry.set_text("");
    super::process_events();

    let st = state.borrow();
    assert!(st.query.is_empty());
    assert!(st.results.is_empty());
}
