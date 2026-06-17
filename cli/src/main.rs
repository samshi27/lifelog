use chrono::{Datelike, Duration, Local};
use engine::{Commit, db::Database};

// main returns a Result so the `?` operator can propagate database errors out
// rusqlite::Result<()> = "succeeds with nothing, or fails with a db error"
fn main() -> rusqlite::Result<()> {
    // open the database (creates the file + tables on first run)
    // `mut` because save() needs a mutable borrow (it opens a transaction)
    let mut db = Database::open("lifelog.db")?;

    // collect what the user typed. The FIRST arg is always the program's own
    // name, so we skip it with skip(1) and keep the rest
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Nothing typed? Show today and get out
    if args.is_empty() {
        show_today(&db)?;
        return Ok(());
    }

    // the first real word decides the command
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
            let today_key = Local::now().format("%Y-%m-%d").to_string(); // query string
            let today_formatted = Local::now().format("%-d %B %Y").to_string(); // display string

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
        // anything else is treated as a commit to log
        _ => {
            // re-join all the words back into one line for the parser
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

    println!("\n{}  ({} commits)\n", today_formatted, commits.len());
    for commit in &commits {
        println!("{}", commit);
    }
    Ok(())
}

// map a day's commit count to an emerald RGB
// dark (quiet) -> bright (busy)
// returns (r, g, b)
fn shade_for(count: u32) -> (u8, u8, u8) {
    match count {
        0 => (22, 33, 28),       // empty: faint, just above black
        1..=2 => (15, 61, 42),   // dark emerald
        3..=4 => (28, 125, 82),  // mid
        5..=7 => (39, 163, 111), // bright
        _ => (52, 209, 127),     // 8+: brightest
    }
}

// print tiles
fn tile(count: u32) {
    let (r, g, b) = shade_for(count);

    print!("\x1b[48;2;{};{};{}m  \x1b[0m", r, g, b);
}

fn show_grid(db: &Database) -> rusqlite::Result<()> {
    // 30 days ago, as the query's lower bound
    let today = Local::now().date_naive();
    let start = today - Duration::days(29); // 29 + today = 30 days
    let start_key = start.format("%Y-%m-%d").to_string();

    let counts = db.daily_counts(&start_key)?;

    println!("\nlast 30 days\n");

    // padding: figure out which weekday `start` is, and print blank cells
    // so the first real tile lands in the correct column
    // we'll treat monday as column 0
    let pad = start.weekday().num_days_from_monday();
    for _ in 0..pad {
        print!("   "); // blank cell: 3 spaces (matches tile width + gap)
    }

    // walk all 30 days, drawing a tile for each
    let mut current = start;
    let mut column = pad;
    for _ in 0..30 {
        let key = current.format("%Y-%m-%d").to_string();
        // map.get returns Option<&u32>; copied() turns it into Option<u32>;
        // unwrap_or(0) means "0 if this day isn't in the map"
        let count = counts.get(&key).copied().unwrap_or(0);

        tile(count);
        print!(" "); // gap between tiles

        column += 1;
        if column == 7 {
            // end of a week: newline, reset column
            println!();
            column = 0;
        }

        current = current + Duration::days(1);
    }
    println!("\n");
    Ok(())
}
