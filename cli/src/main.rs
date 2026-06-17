use chrono::Utc;
use engine::{Commit, db::Database};

// main returns a Result so the `?` operator can propagate database errors out
// rusqlite::Result<()> = "succeeds with nothing, or fails with a db error"
fn main() -> rusqlite::Result<()> {
    // open the database (creates the file + tables on first run)
    // `mut` because save() needs a mutable borrow (it opens a transaction)
    let mut db = Database::open("lifelog.db")?;

    // TEMPORARY: hardcoded line so there's something to save each run
    // next step replaces this with real typed input
    let line = "work:report finished the Q2 draft -t 120 -s 1450";
    if let Ok(commit) = Commit::parse(line) {
        db.save(&commit)?;
    }

    // today's date as "YYYY-MM-DD" to match the `day` column
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let commits = db.commits_for_day(&today)?;

    // print a header, then each commit via its Display impl ({} not {:?})
    println!("\n{}  ({} commits)\n", today, commits.len());
    for commit in &commits {
        println!("{}", commit);
    }

    Ok(())
}
