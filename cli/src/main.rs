use chrono::{Datelike, Duration, Local};
use engine::{Commit, db::Database};

mod app;
mod input;
mod screens;
mod tui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = Database::open_default()?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        tui::run(&db)?;
        return Ok(());
    }

    match args[0].as_str() {
        "log" => {
            show_today(&db)?;
        }

        "push" => {
            let today = Local::now().format("%Y-%m-%d").to_string();
            match db.seal_day(&today)? {
                engine::db::PushResult::Sealed(n) => {
                    println!("sealed {}, {} commits", today, n);
                }
                engine::db::PushResult::Resealed(total, new) => {
                    println!("re-sealed {}, {} commits ({} new)", today, total, new);
                }
                engine::db::PushResult::NothingNew(_at) => {
                    println!("already sealed - nothing new to push");
                }
                engine::db::PushResult::Empty => {
                    println!("nothing to seal - log something first");
                }
            }
        }

        "status" => {
            let today_key = Local::now().format("%Y-%m-%d").to_string(); 
            let today_formatted = Local::now().format("%-d %B %Y").to_string(); 

            match db.day_status(&today_key)? {
                engine::db::DayStatus::Sealed(n) => {
                    println!("{} • sealed • {} commits", today_formatted, n);
                }
                engine::db::DayStatus::Draft(n) => {
                    println!(
                        "{} • draft • {} commits - not pushed yet",
                        today_formatted, n
                    );
                }
                engine::db::DayStatus::Empty => {
                    println!("{} • nothing logged yet", today_formatted);
                }
            }
        }
        "grid" => {
            show_grid(&db)?;
        }

        _ => {
            let line = args.join(" ");
            match Commit::parse(&line) {
                Ok(commit) => {
                    db.save(&commit)?;
                    println!("logged: {}", commit);
                }
                Err(e) => println!("couldn't parse that: {:?}", e),
            }
        }
    }

    Ok(())
}

fn show_today(db: &Database) -> rusqlite::Result<()> {
    let today_key = Local::now().format("%Y-%m-%d").to_string();
    let today_formatted = Local::now().format("%-d %B %Y").to_string();
    let commits = db.commits_for_day(&today_key)?;

    let n: usize = commits.len();
    let plural = if n == 1 { "commit" } else { "commits" };

    println!("\n{}  ({} {})\n", today_formatted, n, plural);

    for commit in &commits {
        println!("{}", commit);
    }

    Ok(())
}

fn shade_for(count: u32) -> (u8, u8, u8) {
    match count {
        0 => (22, 33, 28),
        1..=2 => (15, 61, 42),
        3..=4 => (28, 125, 82),
        5..=7 => (39, 163, 111),
        _ => (52, 209, 127),
    }
}

fn tile(count: u32) {
    let (r, g, b) = shade_for(count);

    print!("\x1b[48;2;{};{};{}m  \x1b[0m", r, g, b);
}

fn show_grid(db: &Database) -> rusqlite::Result<()> {
    let today = Local::now().date_naive();
    let start = today - Duration::days(29);
    let start_key = start.format("%Y-%m-%d").to_string();

    let counts = db.daily_counts(&start_key)?;

    println!("\nlast 30 days\n");

    let pad = start.weekday().num_days_from_monday();
    for _ in 0..pad {
        print!("   ");
    }

    let mut current = start;
    let mut column = pad;
    for _ in 0..30 {
        let key = current.format("%Y-%m-%d").to_string();
        let count = counts.get(&key).copied().unwrap_or(0);

        tile(count);
        print!(" ");

        column += 1;
        if column == 7 {
            println!();
            column = 0;
        }

        current = current + Duration::days(1);
    }
    println!("\n");
    Ok(())
}
