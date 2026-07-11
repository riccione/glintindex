//! Results list widget.
//!
//! Displays search results as a scrollable list of items.
//! Each item shows the filename and full path.
//! Supports mouse selection and highlights the active item.

use iced::widget::{Column, column, container, mouse_area, scrollable, text};

use crate::message::Message;
use glintindex_core::SearchResult;

/// The maximum number of visible results before scrolling kicks in.
const MAX_VISIBLE_HEIGHT: f32 = 500.0;

/// Creates the results list widget.
///
/// Each result is rendered as a clickable item showing the filename
/// and full path. The currently selected item is highlighted.
/// Clicking an item sends `Message::ResultSelected`.
pub fn view<'a>(results: &'a [SearchResult], selected: Option<usize>) -> Column<'a, Message> {
    if results.is_empty() {
        return column![
            container(
                text("No results")
                    .size(14)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5))
            )
            .center_x(iced::Length::Fill)
            .padding(20)
        ];
    }

    let items: Column<'a, Message> = results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let is_selected = selected == Some(index);
            let filename = result.document.filename().to_string();
            let path = result.document.path.display().to_string();

            let content = column![
                text(filename).size(14).width(iced::Length::Fill),
                text(path)
                    .size(11)
                    .color(iced::Color::from_rgb(0.45, 0.45, 0.45))
                    .width(iced::Length::Fill),
            ]
            .spacing(2);

            let item = container(content)
                .width(iced::Length::Fill)
                .padding(iced::Padding::new(10.0).horizontal(12.0));

            let styled_item = if is_selected {
                item.style(iced::widget::container::bordered_box)
            } else {
                item
            };

            mouse_area(styled_item)
                .on_press(Message::ResultSelected(index))
                .into()
        })
        .collect();

    column![scrollable(items).height(iced::Length::Fixed(MAX_VISIBLE_HEIGHT))]
}
