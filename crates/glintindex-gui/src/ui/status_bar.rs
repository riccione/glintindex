//! Status bar UI component.
//!
//! Displays status messages at the bottom of the window.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Label, Separator};

use crate::window::WindowState;

/// Builds the status bar widget.
pub fn build(state: &Rc<RefCell<WindowState>>) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let separator = Separator::new(gtk::Orientation::Horizontal);
    container.append(&separator);

    let st = state.borrow();
    let status_label = Label::builder()
        .label(&st.status)
        .halign(gtk::Align::Start)
        .css_classes(["dim-label", "caption"])
        .build();
    container.append(&status_label);

    container
}
