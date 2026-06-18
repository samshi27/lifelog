use crossterm::event::KeyCode;

use crate::{app::App, screens::Screen};

pub fn handle_key(code: KeyCode, app: &mut App) {
    match app.screen {
        Screen::Dashboard => match code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('t') => app.screen = Screen::Today,
            _ => {}
        },

        Screen::Today => match code {
            KeyCode::Esc => app.screen = Screen::Dashboard,
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        },
    }
}
