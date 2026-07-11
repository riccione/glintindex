//! Indexed Folders settings page.
//!
//! Displays every configured folder with its enabled/disabled state.
//! Provides actions to add, remove, enable, and disable folders.

use iced::widget::{button, column, container, row, rule, scrollable, text};

use crate::message::Message;
use crate::state::AppState;

/// Renders the Indexed Folders settings page.
///
/// Shows a list of all configured folders with toggle and remove
/// buttons. An "Add Folder" button opens the native folder picker.
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    let add_button = button(text("Add Folder".to_string()).size(14))
        .on_press(Message::AddFolderRequested)
        .padding(iced::Padding::new(8.0).horizontal(16.0));

    let header = column![
        text("Indexed Folders".to_string()).size(20),
        rule::horizontal(1),
        row![text("Manage folders that are indexed for search.".to_string()).size(13)
            .color(iced::Color::from_rgb(0.4, 0.4, 0.4)),]
            .push(add_button),
    ]
    .spacing(8);

    let mut list = column![].spacing(4);

    if state.indexed_folders.is_empty() {
        list = list.push(
            text("No folders configured. Click \"Add Folder\" to get started.".to_string())
                .size(14)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        );
    } else {
        for folder in &state.indexed_folders {
            let path_display = folder.path.display().to_string();
            let status_label = if folder.enabled { "Enabled" } else { "Disabled" };
            let status_color = if folder.enabled {
                iced::Color::from_rgb(0.2, 0.6, 0.2)
            } else {
                iced::Color::from_rgb(0.6, 0.2, 0.2)
            };

            let toggle_label = if folder.enabled { "Disable" } else { "Enable" };
            let toggle_msg = if folder.enabled {
                Message::DisableFolderRequested(path_display.clone())
            } else {
                Message::EnableFolderRequested(path_display.clone())
            };

            let row_content = row![
                column![
                    text(path_display.clone()).size(14).width(iced::Length::Fill),
                    text(status_label.to_string())
                        .size(11)
                        .color(status_color),
                ]
                .spacing(2)
                .width(iced::Length::Fill),
                button(text(toggle_label.to_string()).size(12))
                    .on_press(toggle_msg)
                    .padding(iced::Padding::new(4.0).horizontal(8.0)),
                button(text("Remove".to_string()).size(12))
                    .on_press(Message::RemoveFolderRequested(path_display))
                    .padding(iced::Padding::new(4.0).horizontal(8.0))
                    .style(iced::widget::button::danger),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center);

            list = list.push(
                container(row_content)
                    .width(iced::Length::Fill)
                    .padding(iced::Padding::new(8.0).horizontal(12.0))
                    .style(iced::widget::container::bordered_box),
            );
        }
    }

    let content = column![header, scrollable(list).height(iced::Length::Fill)]
        .spacing(12)
        .width(iced::Length::Fill);

    container(content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(20)
        .into()
}
