//! Ignored Folders settings page.
//!
//! Displays the list of ignored folder names that are excluded
//! from indexing. Users can add and remove entries.

use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};

use crate::message::Message;
use crate::state::AppState;

/// Renders the Ignored Folders settings page.
///
/// Shows a list of ignored folder names with remove buttons and
/// a text input to add new entries.
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    let header = column![
        text("Ignored Folders".to_string()).size(20),
        rule::horizontal(1),
        text("Folder names excluded from indexing. Changes are saved immediately.".to_string())
            .size(13)
            .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),
    ]
    .spacing(8);

    let add_input = text_input("New folder name...", &state.settings_status)
        .on_input(Message::AddIgnoredFolderRequested)
        .on_submit(Message::AddIgnoredFolderRequested(state.settings_status.clone()))
        .padding(8)
        .width(iced::Length::Fill);

    let mut list = column![].spacing(4);

    if state.ignored_folders.is_empty() {
        list = list.push(
            text("No ignored folders configured.".to_string())
                .size(14)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        );
    } else {
        for name in &state.ignored_folders {
            let row_content = row![
                text(name.clone()).size(14).width(iced::Length::Fill),
                button(text("Remove".to_string()).size(12))
                    .on_press(Message::RemoveIgnoredFolderRequested(name.clone()))
                    .padding(iced::Padding::new(4.0).horizontal(8.0))
                    .style(iced::widget::button::danger),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            list = list.push(
                container(row_content)
                    .width(iced::Length::Fill)
                    .padding(iced::Padding::new(6.0).horizontal(12.0))
                    .style(iced::widget::container::bordered_box),
            );
        }
    }

    let content = column![
        header,
        add_input,
        scrollable(list).height(iced::Length::Fill)
    ]
    .spacing(12)
    .width(iced::Length::Fill);

    container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(20)
        .into()
}
