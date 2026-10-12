//! Search flow tests.
//!
//! Covers query changes, background dispatch, result application,
//! selection → preview, and pagination math — headless, no widgets.

use std::time::{Duration, Instant};

use glintindex_core::{Document, SearchQuery, SearchResult};

use super::{run_ui, setup_test_app, setup_test_service, wait_for_preview};

fn sample_doc(i: usize) -> Document {
    Document::new(
        std::path::PathBuf::from(format!("/test/file_{i}.txt")),
        100,
        std::time::SystemTime::now(),
        format!("content {i}"),
    )
}

fn push_results(app: &mut crate::app::GlintApp, count: usize) {
    for i in 0..count {
        app.results
            .push(SearchResult::new(sample_doc(i), 1.0, format!("snip {i}")));
    }
}

#[test]
fn test_results_list_smoke_renders() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 3);
    run_ui(&mut app, 2);
    assert_eq!(app.results.len(), 3);
}

#[test]
fn test_row_selection_updates_selected_index_and_preview() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 3);

    app.select_result(1);
    assert_eq!(app.selected_index, Some(1));
    assert_eq!(
        app.selected_path.as_deref(),
        Some(std::path::Path::new("/test/file_1.txt"))
    );
    assert_eq!(app.status, "file_1.txt");
    assert!(app.preview_loading, "generation starts in the background");

    wait_for_preview(&mut app);
    assert!(
        app.preview_model.is_some(),
        "selection loads a preview model"
    );
    assert!(!app.preview_loading);
}

#[test]
fn test_selection_out_of_range_is_ignored() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 1);
    app.select_result(5);
    assert_eq!(app.selected_index, None);
    assert!(app.preview_model.is_none());
}

#[test]
fn test_select_result_shows_loading_state() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 1);

    app.select_result(0);
    assert_eq!(app.status, "file_0.txt", "status feedback is immediate");
    assert!(app.preview_loading, "generation runs in the background");
    assert!(app.preview_model.is_none(), "model arrives via the channel");

    wait_for_preview(&mut app);
    assert!(app.preview_model.is_some(), "worker response applied");
    assert!(!app.preview_loading);
}

#[test]
fn test_stale_preview_response_is_dropped() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 1);
    app.select_result(0);
    let stale_id = app.preview_request_id;

    // Supersede the selection (new click / clear / new search results).
    app.invalidate_preview();

    // Forge a late response for the old selection.
    if let Some(tx) = app.preview_tx.clone() {
        let marker = crate::preview_model::PreviewLine {
            number_prefix: String::new(),
            text: "STALE_MARKER".into(),
            style: crate::preview_model::LineStyle::Normal,
            match_ranges: Vec::new(),
        };
        let mut model = crate::preview_model::PreviewModel::default();
        model.lines.push(marker);
        tx.send((stale_id, model)).unwrap();
    }
    app.drain_preview_channel();

    assert!(
        app.preview_model.is_none(),
        "superseded response must not apply"
    );
    assert!(!app.preview_loading);
}

#[test]
fn test_search_response_invalidates_pending_preview() {
    let (mut app, _tmp) = setup_test_app();
    push_results(&mut app, 1);
    app.select_result(0);
    let pending_id = app.preview_request_id;
    assert!(app.preview_loading);

    let response = glintindex_core::SearchResponse::new(Vec::new(), 0, 0, 0);
    crate::search::apply_search_response(&mut app, response);

    assert!(app.preview_model.is_none());
    assert!(!app.preview_loading, "pending preview is invalidated");
    assert_ne!(app.preview_request_id, pending_id);

    // A late worker response for the old id must stay dropped.
    app.drain_preview_channel();
    assert!(app.preview_model.is_none());
}

#[test]
fn test_metadata_only_preview_is_synchronous() {
    let (mut app, _tmp) = setup_test_app();
    let mut doc = sample_doc(0);
    doc.is_metadata_only = true;
    app.results.push(SearchResult::new(doc, 1.0, "snip".into()));

    app.select_result(0);

    assert!(!app.preview_loading, "no worker for metadata-only files");
    let model = app.preview_model.as_ref().expect("synchronous model");
    assert!(
        model
            .lines
            .iter()
            .any(|l| l.text.contains("indexed by file name"))
    );
}

#[test]
fn test_clear_results_resets_search_state() {
    let (mut app, _tmp) = setup_test_app();
    app.query = "old query".to_string();
    app.search_text = "old query".to_string();
    push_results(&mut app, 1);
    app.select_result(0);

    app.clear_results();

    assert!(app.results.is_empty());
    assert_eq!(app.selected_index, None);
    assert!(app.preview_model.is_none());
    assert!(app.selected_path.is_none());
    assert_eq!(app.status, "No results found");
}

#[test]
fn test_on_query_changed_empty_clears_results() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "salsa".to_string();
    app.on_query_changed();
    assert_eq!(app.query, "salsa");

    push_results(&mut app, 2);
    app.search_text = String::new();
    app.on_query_changed();

    assert!(app.query.is_empty());
    assert!(app.results.is_empty());
    assert_eq!(app.selected_index, None);
    assert!(app.preview_model.is_none());
    assert_eq!(app.status, "No results found");
}

#[test]
fn test_on_query_changed_resets_pagination() {
    let (mut app, _tmp) = setup_test_app();
    app.current_page = 3;
    app.search_text = "new".to_string();
    app.on_query_changed();
    assert_eq!(app.current_page, 1);
    assert!(app.last_query_edit.is_some());
}

#[test]
fn test_clear_search_resets_input_and_results() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "old query".to_string();
    app.on_query_changed();
    assert_eq!(app.query, "old query");
    push_results(&mut app, 2);
    app.select_result(0);
    app.current_page = 2;
    app.last_dispatched = "old query".to_string();

    app.clear_search();

    assert!(app.search_text.is_empty(), "input is cleared");
    assert!(app.query.is_empty(), "query collapses to empty");
    assert!(app.results.is_empty(), "results are cleared");
    assert_eq!(app.selected_index, None);
    assert!(app.selected_path.is_none());
    assert!(app.preview_model.is_none());
    assert_eq!(app.total_results, 0);
    assert_eq!(app.current_page, 1);
    assert_eq!(app.last_dispatched, String::new());
    assert_eq!(app.status, "No results found");
}

#[test]
fn test_toolbar_renders_clear_button_without_altering_input() {
    let (mut app, tmp) = setup_test_app();
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    app.service.add_folder(&folder).unwrap();
    app.view = crate::app::View::Main;

    // Non-empty input exercises the overlaid clear button; rendering
    // must not clear or otherwise mutate the typed text.
    app.search_text = "hello".to_string();
    run_ui(&mut app, 2);
    assert_eq!(app.search_text, "hello");

    // Empty input hides the button; still renders cleanly.
    app.search_text.clear();
    run_ui(&mut app, 2);
    assert!(app.search_text.is_empty());
    let _ = app.service.remove_folder(&folder);
}

#[test]
fn test_apply_search_response_guards() {
    let (mut app, _tmp) = setup_test_app();
    app.per_page = 20;

    // Error sentinel: limit == 0 must not poison per_page.
    crate::search::apply_search_response(
        &mut app,
        glintindex_core::SearchResponse::new(Vec::new(), 0, 0, 0),
    );
    assert_eq!(app.per_page, 20);
    assert_eq!(app.status, "Search failed");
    assert_eq!(app.total_pages, 1);

    // Normal response: pagination math (total=100, offset=40, limit=20).
    crate::search::apply_search_response(
        &mut app,
        glintindex_core::SearchResponse::new(Vec::new(), 100, 40, 20),
    );
    assert_eq!(
        app.per_page, 20,
        "per_page never overwritten from responses"
    );
    assert_eq!(app.total_pages, 5);
    assert_eq!(app.current_page, 3);
    assert_eq!(app.status, "Showing 41–40 of 100 results");

    // Empty result set (total=0, offset=0, limit=20).
    crate::search::apply_search_response(
        &mut app,
        glintindex_core::SearchResponse::new(Vec::new(), 0, 0, 20),
    );
    assert_eq!(app.status, "No results found");
    assert_eq!(app.total_pages, 1);
}

#[test]
fn test_change_page_clamps_and_dispatches() {
    let (mut app, _tmp) = setup_test_app();
    app.query = "beta".to_string();
    app.total_pages = 2;

    app.change_page(-1);
    assert_eq!(app.current_page, 1, "cannot go below page 1");

    app.change_page(1);
    assert_eq!(app.current_page, 2);
    assert_eq!(
        app.last_dispatched, "beta",
        "page change dispatches a search"
    );

    app.change_page(1);
    assert_eq!(app.current_page, 2, "cannot exceed last page");
}

#[test]
fn test_dispatch_search_marks_query_dispatched() {
    let (mut app, _tmp) = setup_test_app();
    app.query = "hello".to_string();
    app.dispatch_search();
    assert_eq!(app.last_dispatched, "hello");
}

#[test]
fn test_dispatch_search_ignores_empty_query() {
    let (mut app, _tmp) = setup_test_app();
    app.dispatch_search();
    assert!(app.last_dispatched.is_empty());
}

#[test]
fn test_debounce_fires_after_window() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "hello".to_string();
    app.on_query_changed();
    // Age the edit past the 150ms window.
    app.last_query_edit = Some(Instant::now() - Duration::from_millis(200));

    let ctx = super::test_context();
    app.handle_debounce(&ctx);
    assert_eq!(
        app.last_dispatched, "hello",
        "debounce dispatches after window"
    );
}

#[test]
fn test_debounce_not_fired_before_window() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "hello".to_string();
    app.on_query_changed(); // fresh timestamp

    let ctx = super::test_context();
    app.handle_debounce(&ctx);
    assert!(app.last_dispatched.is_empty(), "debounce waits 150ms");
}

#[test]
fn test_dispatch_search_ignores_short_query() {
    let (mut app, _tmp) = setup_test_app();
    app.query = "hel".to_string();
    app.dispatch_search();
    assert!(
        app.last_dispatched.is_empty(),
        "3-char query is not dispatched"
    );

    // Whitespace does not count toward the minimum.
    app.query = "  ab  ".to_string();
    app.dispatch_search();
    assert!(
        app.last_dispatched.is_empty(),
        "trimmed query under 4 not dispatched"
    );
}

#[test]
fn test_debounce_not_fired_for_short_query() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "hel".to_string();
    app.on_query_changed();
    // Age the edit far past the 150ms window — still ineligible.
    app.last_query_edit = Some(Instant::now() - Duration::from_millis(200));

    let ctx = super::test_context();
    app.handle_debounce(&ctx);
    assert!(
        app.last_dispatched.is_empty(),
        "debounce never dispatches short queries"
    );
}

#[test]
fn test_on_query_changed_short_query_clears_results() {
    let (mut app, _tmp) = setup_test_app();
    app.search_text = "hello".to_string();
    app.on_query_changed();
    push_results(&mut app, 2);
    app.select_result(0);
    assert!(!app.results.is_empty());

    app.search_text = "hel".to_string();
    app.on_query_changed();

    assert_eq!(app.query, "hel");
    assert!(app.results.is_empty(), "results cleared below the minimum");
    assert_eq!(app.selected_index, None);
    assert!(app.preview_model.is_none());
    assert_eq!(app.status, "Type at least 4 characters to search");
}

#[test]
fn test_search_result_contains_document_fields() {
    let doc = Document::new(
        std::path::PathBuf::from("/home/user/project/main.rs"),
        4096,
        std::time::SystemTime::now(),
        "fn main() {}".into(),
    );
    let result = SearchResult::new(doc, 0.95, "main".into());
    assert_eq!(
        result.document.path,
        std::path::PathBuf::from("/home/user/project/main.rs")
    );
    assert_eq!(result.document.size, 4096);
    assert_eq!(result.document.content, "fn main() {}");
    assert!(result.score > 0.0);
    assert!(!result.snippet.is_empty());
}

#[test]
fn test_search_on_empty_index_returns_empty() {
    let (service, _tmp) = setup_test_service();
    let results = service
        .search(&SearchQuery::new("anything"))
        .unwrap_or_default()
        .results;
    assert!(results.is_empty(), "Empty index should return no results");
}

#[test]
fn test_search_with_special_characters() {
    let (mut app, tmp) = setup_test_app();
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(
        folder.join("special.txt"),
        "fn main() { println!(\"hello\"); }",
    )
    .unwrap();

    app.service.add_folder(&folder).unwrap();
    app.service.index_folder(&folder).unwrap();

    let results = app
        .service
        .search(&SearchQuery::new("fn main"))
        .unwrap_or_default()
        .results;
    assert!(
        !results.is_empty(),
        "Search for 'fn main' should find results"
    );
}

#[test]
fn test_dispatched_search_delivers_results() {
    let (mut app, tmp) = setup_test_app();
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("test.txt"), "hello world").unwrap();
    app.service.add_folder(&folder).unwrap();
    app.service.index_folder(&folder).unwrap();

    app.query = "hello".to_string();
    app.dispatch_search();

    // Drain the background channel until results arrive (or time out).
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.results.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        run_ui(&mut app, 1);
    }
    assert!(
        !app.results.is_empty(),
        "background search delivers results"
    );
    assert!(
        app.results[0]
            .document
            .path
            .to_string_lossy()
            .contains("test.txt")
    );
}

#[test]
fn test_search_multiple_results_ordered_by_score() {
    let (service, tmp) = setup_test_service();
    let mut app_service = service;
    let folder = tmp.path().join("docs");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("a.txt"), "alpha beta").unwrap();
    std::fs::write(folder.join("b.txt"), "beta beta beta").unwrap();

    app_service.add_folder(&folder).unwrap();
    app_service.index_folder(&folder).unwrap();

    let results = app_service
        .search(&SearchQuery::new("beta"))
        .unwrap_or_default()
        .results;
    if results.len() >= 2 {
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "Results should be ordered by descending score"
            );
        }
    }
}

#[test]
fn test_search_empty_query_returns_empty() {
    let (service, _tmp) = setup_test_service();
    let results = service
        .search(&SearchQuery::new(""))
        .unwrap_or_default()
        .results;
    assert!(results.is_empty(), "Empty query should return no results");
}

#[test]
fn test_zero_per_page_paged_query_clamps() {
    // SearchQuery::paged clamps limit >= 1; verify no panic path.
    let q = SearchQuery::paged("x", 1, 0);
    assert!(q.limit >= 1);
}
