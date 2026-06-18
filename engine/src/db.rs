use crate::{Commit, Pillar, Trailer};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

pub enum PushResult {
    Sealed(u32),
    Resealed(u32, u32),
    NothingNew(String),
    Empty,
}

pub enum DayStatus {
    Sealed(u32),
    Draft(u32),
    Empty,
}

impl Database {
    fn default_path() -> PathBuf {
        let proj =
            ProjectDirs::from("", "", "lifelog").expect("could not determine a home directory");

        let data_dir = proj.data_dir();

        std::fs::create_dir_all(data_dir).expect("could not create the lifelog data folder");

        data_dir.join("lifelog.db")
    }

    pub fn open_default() -> rusqlite::Result<Database> {
        let path = Self::default_path();
        Self::open(path.to_str().expect("db path was not valid UTF-8"))
    }

    pub fn open(path: &str) -> rusqlite::Result<Database> {
        let conn = Connection::open(path)?;

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

    pub fn commits_for_day(&self, day: &str) -> rusqlite::Result<Vec<Commit>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, pillar, scope, subject, body, is_highlight, created_at
             FROM commits WHERE day = ?1 ORDER BY created_at",
        )?;

        let rows = stmt.query_map([day], |row| {
            let id_str: String = row.get(0)?;
            let pillar_str: String = row.get(1)?;
            let created_str: String = row.get(6)?;

            Ok(Commit {
                id: Uuid::parse_str(&id_str).unwrap(),
                pillar: Pillar::from_str(&pillar_str),
                scope: row.get(2)?,
                subject: row.get(3)?,
                body: row.get(4)?,
                is_highlight: row.get(5)?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .unwrap()
                    .with_timezone(&Utc),
                trailers: Vec::new(),
            })
        })?;

        let mut commits: Vec<Commit> = Vec::new();
        for c in rows {
            commits.push(c?);
        }

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

    pub fn save(&mut self, commit: &Commit) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;

        let day = commit.created_at.format("%Y-%m-%d").to_string();

        tx.execute(
            "INSERT INTO commits
                (id, pillar, scope, subject, body, is_highlight, created_at, day)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                commit.id.to_string(),
                commit.pillar.as_str(),
                commit.scope,
                commit.subject,
                commit.body,
                commit.is_highlight,
                commit.created_at.to_rfc3339(),
                day,
            ],
        )?;

        for trailer in &commit.trailers {
            tx.execute(
                "INSERT INTO trailers (commit_id, key, value)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![commit.id.to_string(), trailer.key, trailer.value],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn seal_day(&mut self, day: &str) -> rusqlite::Result<PushResult> {
        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM commits WHERE day = ?1",
            [day],
            |row| row.get(0),
        )?;

        if total == 0 {
            return Ok(PushResult::Empty);
        }

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
            Some(pushed_at) => {
                let new_count: u32 = self.conn.query_row(
                    "SELECT COUNT(*) FROM commits WHERE day = ?1 AND created_at > ?2",
                    rusqlite::params![day, pushed_at],
                    |row| row.get(0),
                )?;

                if new_count == 0 {
                    Ok(PushResult::NothingNew(pushed_at))
                } else {
                    self.conn.execute(
                        "UPDATE days SET pushed_at = ?2 WHERE day = ?1",
                        rusqlite::params![day, now],
                    )?;
                    Ok(PushResult::Resealed(total, new_count))
                }
            }
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
        let total: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM commits WHERE day = ?1",
            [day],
            |row| row.get(0),
        )?;

        if total == 0 {
            return Ok(DayStatus::Empty);
        }

        let sealed: Option<bool> = self
            .conn
            .query_row("SELECT sealed FROM days WHERE day = ?1", [day], |row| {
                row.get(0)
            })
            .optional()?;

        match sealed {
            Some(true) => Ok(DayStatus::Sealed(total)),
            _ => Ok(DayStatus::Draft(total)),
        }
    }

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

        let mut counts = HashMap::new();
        for r in rows {
            let (day, count) = r?;
            counts.insert(day, count);
        }
        Ok(counts)
    }
}
