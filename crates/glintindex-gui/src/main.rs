mod app;
mod message;
mod pages;
mod state;
mod theme;
mod widgets;

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view).run()
}
