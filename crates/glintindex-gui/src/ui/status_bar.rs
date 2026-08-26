//! Status bar UI component.
//!
//! Displays status messages and pagination controls at the bottom of the window.
//! The left side shows result stats (e.g. "Showing 1–20 of 150 results").
//! The right side contains Previous / Page X of Y / Next pagination controls.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Button, Label};

use crate::window::WindowState;

/// Build references returned by [`build`].
pub struct StatusBarWidgets {
    pub container: gtk::Box,
    pub status_label: Label,
    pub page_label: Label,
    pub prev_btn: Button,
    pub next_btn: Button,
}

/// Builds the integrated status bar with pagination controls.
///
/// Layout:
/// ```text
/// [ status_label (hexpand) ]                    [Prev] Page X of Y [Next]
/// ```
pub fn build(state: &Rc<RefCell<WindowState>>) -> StatusBarWidgets {
    let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    container.set_margin_start(8);
    container.set_margin_end(8);
    container.set_margin_top(4);
    container.set_margin_bottom(4);

    // ── Left: status label ────────────────────────────────────
    let st = state.borrow();
    let status_label = Label::builder()
        .label(&st.status)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();
    container.append(&status_label);

    // ── Right: pagination controls ────────────────────────────
    let prev_btn = Button::builder().label("Previous").build();
    let page_label = Label::new(Some("Page 1 of 1"));
    let next_btn = Button::builder().label("Next").build();

    prev_btn.set_sensitive(false);
    next_btn.set_sensitive(false);

    let pagination_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    pagination_box.set_halign(gtk::Align::End);
    pagination_box.append(&prev_btn);
    pagination_box.append(&page_label);
    pagination_box.append(&next_btn);

    container.append(&pagination_box);

    // ── Click handlers ────────────────────────────────────────
    let prev_btn_rc = Rc::new(prev_btn.clone());
    let next_btn_rc = Rc::new(next_btn.clone());

    // Previous
    {
        let state_clone = state.clone();
        let page_label = page_label.clone();
        let next_btn = next_btn_rc.clone();
        prev_btn.connect_clicked(move |btn| {
            let mut st = state_clone.borrow_mut();
            if st.current_page > 1 {
                st.current_page -= 1;
                let page = st.current_page;
                let total = st.total_pages;
                let query = st.query.clone();
                drop(st);
                page_label.set_text(&format!("Page {page} of {total}"));
                btn.set_sensitive(page > 1);
                next_btn.set_sensitive(page < total);
                super::toolbar::perform_search_from_state(&state_clone, &query);
            }
        });
    }

    // Next
    {
        let state_clone = state.clone();
        let page_label = page_label.clone();
        let prev_btn = prev_btn_rc.clone();
        next_btn.connect_clicked(move |btn| {
            let mut st = state_clone.borrow_mut();
            if st.current_page < st.total_pages {
                st.current_page += 1;
                let page = st.current_page;
                let total = st.total_pages;
                let query = st.query.clone();
                drop(st);
                page_label.set_text(&format!("Page {page} of {total}"));
                prev_btn.set_sensitive(page > 1);
                btn.set_sensitive(page < total);
                super::toolbar::perform_search_from_state(&state_clone, &query);
            }
        });
    }

    StatusBarWidgets {
        container,
        status_label,
        page_label,
        prev_btn,
        next_btn,
    }
}

/// Refreshes the status label text and pagination control states from `state`.
///
/// Call this from the GTK main thread after updating `WindowState` fields.
/// All widget references are optional so callers can guard on first paint.
pub fn refresh_status_bar(
    state: &WindowState,
    status_label: Option<&Label>,
    page_label: Option<&Label>,
    prev_btn: Option<&Button>,
    next_btn: Option<&Button>,
) {
    if let Some(label) = status_label {
        label.set_text(&state.status);
    }
    if let (Some(pl), Some(pb), Some(nb)) = (page_label, prev_btn, next_btn) {
        let page = state.current_page;
        let total = state.total_pages;
        pl.set_text(&format!("Page {page} of {total}"));
        pb.set_sensitive(page > 1);
        nb.set_sensitive(page < total);
    }
}
