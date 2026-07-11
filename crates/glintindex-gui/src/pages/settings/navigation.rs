//! Settings navigation sidebar widget.
//!
//! Renders a vertical list of navigation items for switching
//! between settings pages. The active page is visually highlighted.

use iced::widget::{button, column, container, text};

use crate::message::{Message, SettingsPage};

/// Renders the navigation sidebar for the settings window.
///
/// Each item is a button that sends `Message::SettingsPageSelected`
/// when clicked. The currently active page is highlighted.
pub fn view(active: SettingsPage) -> iced::Element<'static, Message> {
    let pages = [
        (SettingsPage::General, "General"),
        (SettingsPage::IndexedFolders, "Indexed Folders"),
        (SettingsPage::IgnoredFolders, "Ignored Folders"),
        (SettingsPage::Index, "Index"),
        (SettingsPage::About, "About"),
    ];

    let mut nav = column![].spacing(2);

    for (page, label) in pages {
        let is_active = active == page;
        let btn = button(text(label.to_string()).size(14))
            .on_press(Message::SettingsPageSelected(page))
            .width(iced::Length::Fill)
            .padding(iced::Padding::new(8.0).horizontal(12.0));

        let styled_btn = if is_active {
            btn.style(iced::widget::button::primary)
        } else {
            btn.style(iced::widget::button::text)
        };

        nav = nav.push(styled_btn);
    }

    container(nav)
        .width(iced::Length::Fixed(200.0))
        .height(iced::Length::Fill)
        .padding(8)
        .into()
}
