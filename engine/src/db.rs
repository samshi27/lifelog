use std::collections::HashMap;

use crate::{Commit, Pillar, Trailer};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::path::PathBuf;
use uuid::Uuid;

// wraps the live sqlite connection; all database access goes through this
pub struct Database {
    conn: Connection,
}

// the outcome of trying to push (seal) a day. Each variant is an EVENT
// the cli can turn into the right message
pub enum PushResult {
    Sealed(u32),        // first seal; carries the commit count
    Resealed(u32, u32), // (total commits, how many are new)
    NothingNew(String), // already sealed; carries the pushed_at time
    Empty,              // no commits to seal
}

// the current state of a day, for `status` to report
pub enum DayStatus {
    Sealed(u32), // sealed; carries commit count
    Draft(u32),  // has commits but not sealed; carries count
    Empty,       // nothing logged
}

impl Database {
    // work out the proper per-OS location for our database file,
    // creating the app's data folder if it doesn't exist yet
    // macOS: ~/Library/Application Support/lifelog/lifelog.db
    fn default_path() -> PathBuf {
        // ("", "", "lifelog") = qualifier, organisation, app name
        // we only care about the app name for a personal tool
        let proj =
            ProjectDirs::from("", "", "lifelog").expect("could not determine a home directory");

        let data_dir = proj.data_dir();

        // make sure the folder exists (e.g. first ever run)
        std::fs::create_dir_all(data_dir).expect("could not create the lifelog data folder");

        // db file lives inside that folder
        data_dir.join("lifelog.db")
    }

    // open the database at the proper OS location
    pub fn open_default() -> rusqlite::Result<Database> {
        let path = Self::default_path();
        Self::open(path.to_str().expect("db path was not valid UTF-8"))
    }

    // open or create the database file; ensure all tables exist
    // returns the ready-to-use Database, or a database error
    pub fn open(path: &str) -> rusqlite::Result<Database> {
        let conn = Connection::open(path)?;

        // run all 3 create statements at once
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS commits (
                id            TEXT PRIMARY KEY,
                pillar        TEXT NOT NULL,
                scope         TEXT,
                subject       TEXT NOT NULL,
                body          TEXT,
                is_highlight  INTEGER NOT NULL,
                created_at    TEXT NOT NULL,
                day           TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS trailers (
                id        INTEGER PRIMARY KEY,
                commit_id TEXT NOT NULL,
                key       TEXT NOT NULL,
                value     TEXT NOT NULL,
                FOREIGN KEY (commit_id) REFERENCES commits(id)
            );

            CREATE TABLE IF NOT EXISTS days (
                day       TEXT PRIMARY KEY,
                sealed    INTEGER NOT NULL,
                pushed_at TEXT
            );
            ",
        )?;

        Ok(Database { conn })
    }

    // read every commit for a given day (e.g. "2026-06-17"), oldest first,
    // rebuilding each database row back into a Commit struct.
    pub fn commits_for_day(&self, day: &str) -> rusqlite::Result<Vec<Commit>> {
        // Prepare the query once. ?1 is a placeholder filled by the `day` argument.
        let mut stmt = self.conn.prepare(
            "SELECT id, pillar, scope, subject, body, is_highlight, created_at
             FROM commits WHERE day = ?1 ORDER BY created_at",
        )?;

        // query_map runs the query and, for EACH row, runs this closure to
        // turn the raw columns back into a Commit
        let rows = stmt.query_map([day], |row| {
            // Pull text columns out; the type annotations tell rusqlite what to read
            let id_str: String = row.get(0)?;
            let pillar_str: String = row.get(1)?;
            let created_str: String = row.get(6)?;

            Ok(Commit {
                // parse the stored text back into real types
                // .unwrap() = "trust this is valid" (our own save() wrote it)
                // TODO: replace unwraps with proper error handling later
                id: Uuid::parse_str(&id_str).unwrap(),
                pillar: Pillar::from_str(&pillar_str),
                scope: row.get(2)?, // option: NULL becomes None automatically
                subject: row.get(3)?,
                body: row.get(4)?,         // option again
                is_highlight: row.get(5)?, // 0/1 in sqlite becomes false/true
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .unwrap()
                    .with_timezone(&Utc), // normalise back to utc
                trailers: Vec::new(),      // filled in by the loop below
            })
        })?;

        // query_map yields an iterator of Results - collect them, propagating errors
        let mut commits: Vec<Commit> = Vec::new();
        for c in rows {
            commits.push(c?);
        }

        // for each commit, fetch its trailers and attach them
        // (One query per commit - fine for a day's worth of commits
        for commit in &mut commits {
            let mut tstmt = self
                .conn
                .prepare("SELECT key, value FROM trailers WHERE commit_id = ?1")?;
            let trailer_rows = tstmt.query_map([commit.id.to_string()], |row| {
                Ok(Trailer {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })?;
            for t in trailer_rows {
                commit.trailers.push(t?);
            }
        }

        Ok(commits)
    }

    // save one commit (and all its trailers) to the database
    // &mut self because opening a transaction mutates the connection
    pub fn save(&mut self, commit: &Commit) -> rusqlite::Result<()> {
        // a transaction: every write below either ALL succeeds or ALL rolls back
        // this prevents a half-saved commit (e.g. commit row but missing trailers)
        let tx = self.conn.transaction()?;

        // the date-only string used for grouping/the grid, e.g. "2026-06-17"
        let day = commit.created_at.format("%Y-%m-%d").to_string();

        // insert the commit row. ?1..?8 are filled, in order, by params!
        // never paste values into sql by hand - placeholders prevent injection bugs
        tx.execute(
            "INSERT INTO commits
                (id, pillar, scope, subject, body, is_highlight, created_at, day)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                commit.id.to_string(),
                commit.pillar.as_str(),
                commit.scope, // option - NULL or value automatically
                commit.subject,
                commit.body,
                commit.is_highlight,
                commit.created_at.to_rfc3339(),
                day,
            ],
        )?;

        // insert one row per trailer, all inside the same transaction
        for trailer in &commit.trailers {
            tx.execute(
                "INSERT INTO trailers (commit_id, key, value)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![commit.id.to_string(), trailer.key, trailer.value],
            )?;
        }

        // nothing is truly written until this line commits the transaction.
        tx.commit()?;
        Ok(())
    }

    pub fn seal_day(&mut self, day: &str) -> rusqlite::Result<PushResult> {
        // how many commits does this day have at all?
        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM commits WHERE day = ?1",
            [day],
            |row| row.get(0),
        )?;

        // rule 1: never seal an empty day
        if total == 0 {
            return Ok(PushResult::Empty);
        }

        // is there already a seal? query_row errors if no row exists, so we
        // use .optional() to turn "no row" into None instead of an error
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT pushed_at FROM days WHERE day = ?1 AND sealed = 1",
                [day],
                |row| row.get(0),
            )
            .optional()?;

        let now = chrono::Utc::now().to_rfc3339();

        match existing {
            // already sealed - only re-seal if commits arrived after pushed_at.
            Some(pushed_at) => {
                let new_count: u32 = self.conn.query_row(
                    "SELECT COUNT(*) FROM commits WHERE day = ?1 AND created_at > ?2",
                    rusqlite::params![day, pushed_at],
                    |row| row.get(0),
                )?;

                if new_count == 0 {
                    Ok(PushResult::NothingNew(pushed_at))
                } else {
                    // re-seal: bump pushed_at to now
                    self.conn.execute(
                        "UPDATE days SET pushed_at = ?2 WHERE day = ?1",
                        rusqlite::params![day, now],
                    )?;
                    Ok(PushResult::Resealed(total, new_count))
                }
            }
            // never sealed - create the row, sealed and stamped
            None => {
                self.conn.execute(
                    "INSERT OR REPLACE INTO days (day, sealed, pushed_at)
                     VALUES (?1, 1, ?2)",
                    rusqlite::params![day, now],
                )?;
                Ok(PushResult::Sealed(total))
            }
        }
    }

    pub fn day_status(&self, day: &str) -> rusqlite::Result<DayStatus> {
        // count today's commits
        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM commits WHERE day = ?1",
            [day],
            |row| row.get(0),
        )?;

        if total == 0 {
            return Ok(DayStatus::Empty);
        }

        // is this day sealed? .optional() turns "no row" into None
        let sealed: Option<bool> = self
            .conn
            .query_row("SELECT sealed FROM days WHERE day = ?1", [day], |row| {
                row.get(0)
            })
            .optional()?;

        // Some(true) = sealed; anything else (None, or a draft row) = draft
        match sealed {
            Some(true) => Ok(DayStatus::Sealed(total)),
            _ => Ok(DayStatus::Draft(total)),
        }
    }

    // for every day on/after `from` (a "YYYY-MM-DD" string), how many commits?
    // returns a map: "2026-06-15" -> 3
    // days with zero commits simply aren't in the map (we treat missing as 0 when drawing)
    pub fn daily_counts(&self, from: &str) -> rusqlite::Result<HashMap<String, u32>> {
        let mut stmt = self.conn.prepare(
            "SELECT day, COUNT(*) FROM commits
             WHERE day >= ?1
             GROUP BY day",
        )?;

        let rows = stmt.query_map([from], |row| {
            let day: String = row.get(0)?;
            let count: u32 = row.get(1)?;
            Ok((day, count))
        })?;

        // collect the (day, count) pairs into a HashMap
        let mut counts = HashMap::new();
        for r in rows {
            let (day, count) = r?;
            counts.insert(day, count);
        }
        Ok(counts)
    }
}

impl Pillar {
    // text - enum, for reading rows back from the database
    // has a catch-all because incoming text could theoretically be anything
    // our own data is always one of the five
    pub fn from_str(s: &str) -> Pillar {
        match s {
            "body" => Pillar::Body,
            "work" => Pillar::Work,
            "make" => Pillar::Make,
            "mind" => Pillar::Mind,
            _ => Pillar::Life,
        }
    }
}
