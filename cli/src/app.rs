use crate::screens::Screen;

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Dashboard,
            should_quit: false,
        }
    }
}
