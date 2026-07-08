use iced::widget::{column, text};
use iced::{Element, Task};

use crate::message::Message;
use crate::state::AppState;

pub struct App {
    state: AppState,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let state = AppState::default();
        (Self { state }, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => Task::none(),
            Message::SearchQueryChanged(query) => {
                self.state.query = query;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![text("GlintIndex Search")].into()
    }
}
