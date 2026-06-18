use crate::{app::App, input::handle_key, screens::Screen};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use engine::db::Database;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::io;

pub fn run(db: &Database) -> io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let result = run_app(&mut terminal, &mut app, db);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    db: &Database,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| match app.screen {
            Screen::Dashboard => draw_dashboard(frame),
            Screen::Today => draw_today(frame, db),
        })?;

        if let Event::Key(key) = event::read()? {
            handle_key(key.code, app);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn draw_dashboard(frame: &mut ratatui::Frame) {
    let area = frame.area();

    let emerald = Color::Rgb(52, 209, 127);
    let dim = Color::Rgb(120, 130, 124);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(7),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new("lifelog")
        .style(Style::default().fg(emerald))
        .alignment(Alignment::Center);

    frame.render_widget(title, chunks[1]);

    let menu_items = [
        ("today", "t"),
        ("grid", "g"),
        ("status", "s"),
        ("push", "p"),
        ("log a commit", "c"),
        ("quit", "q"),
    ];

    let label_width = menu_items
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(0);

    let menu_lines: Vec<Line> = menu_items
        .iter()
        .map(|(label, key)| {
            Line::from(vec![
                Span::styled(
                    format!("{:<width$}   ", label, width = label_width),
                    Style::default().fg(Color::White),
                ),
                Span::styled(*key, Style::default().fg(emerald)),
            ])
        })
        .collect();

    let menu = Paragraph::new(menu_lines).alignment(Alignment::Center);

    frame.render_widget(menu, chunks[3]);

    let status = Paragraph::new("press a key, or q to quit")
        .style(Style::default().fg(dim))
        .alignment(Alignment::Center);

    frame.render_widget(status, chunks[5]);
}

fn draw_today(frame: &mut ratatui::Frame, db: &Database) {
    let area = frame.area();

    let emerald = Color::Rgb(52, 209, 127);
    let dim = Color::Rgb(120, 130, 124);

    let today_key = chrono::Local::now().format("%Y-%m-%d").to_string();

    let commits = db.commits_for_day(&today_key).unwrap_or_default();

    let mut lines = Vec::new();

    if commits.is_empty() {
        lines.push(Line::from(Span::styled(
            "nothing logged yet today",
            Style::default().fg(dim),
        )));
    } else {
        for commit in commits {
            lines.push(Line::from(commit.to_string()));
        }
    }

    let block = Block::default()
        .title(" today ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(emerald));

    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, area);
}
