use crate::{Commit, Pillar, Trailer};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use uuid::Uuid;

// wraps the live sqlite connection; all database access goes through this
pub struct Database {
    conn: Connection,
}

impl Database {
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
