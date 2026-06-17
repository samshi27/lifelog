use chrono::Utc;
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
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let commits = db.commits_for_day(&today)?;

    println!("\n{}  ({} commits)\n", today, commits.len());
    for commit in &commits {
        println!("{}", commit);
    }

    Ok(())
}
