//! Settings window layout.
//!
//! Composes the navigation sidebar and the active settings page
//! into the settings window layout.

use iced::widget::{column, container, row, rule, text};

use crate::message::{Message, SettingsPage};
use crate::state::AppState;

use super::{about, general, ignored_folders, index_page, indexed_folders, navigation};

/// Renders the settings window layout.
///
/// Layout structure:
/// ```text
/// +--------------------------------------------------------------+
/// | Settings                                                     |
/// +----------------------+---------------------------------------+
/// | Navigation Sidebar   |  Active Page Content                  |
/// | - General            |                                       |
/// | - Indexed Folders    |                                       |
/// | - Ignored Folders    |                                       |
/// | - Index              |                                       |
/// | - About              |                                       |
/// +----------------------+---------------------------------------+
/// ```
pub fn view<'a>(state: &'a AppState) -> iced::Element<'a, Message> {
    let close_btn = iced::widget::button(text("Close".to_string()).size(14))
        .on_press(Message::CloseSettings)
        .padding(iced::Padding::new(6.0).horizontal(12.0));

    let title_bar = row![
        text("Settings".to_string()).size(20),
        iced::widget::Space::new().width(iced::Length::Fill),
        close_btn,
    ]
    .align_y(iced::Alignment::Center);

    let sidebar = navigation::view(state.settings_page);

    let page_content = match state.settings_page {
        SettingsPage::General => general::view(state),
        SettingsPage::IndexedFolders => indexed_folders::view(state),
        SettingsPage::IgnoredFolders => ignored_folders::view(state),
        SettingsPage::Index => index_page::view(state),
        SettingsPage::About => about::view(state),
    };

    let body = row![sidebar, rule::vertical(1), page_content].spacing(0);

    let layout = column![title_bar, rule::horizontal(1), body].spacing(8);

    container(layout)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(16)
        .into()
}
