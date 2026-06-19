// the core data types

use chrono::{DateTime, Local, Utc};
use std::fmt;
use uuid::Uuid;

// the five life-areas a commit can belong to
// an enum means a pillar can only ever be one of these five
// the compiler makes an invalid value impossible
#[derive(Debug, Clone, PartialEq)]
pub enum Pillar {
    Body,
    Work,
    Make,
    Mind,
    Life,
}

// one key/value fact attached to a commit; e.g. key="Mins", value="120"
#[derive(Debug, Clone)]
pub struct Trailer {
    pub key: String,
    pub value: String,
}

// the central unit: one thing you did and logged
// Option<T> means "maybe present" — scope and body can be absent (no null in rust)
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Uuid,                  // unique id, fresh per commit
    pub pillar: Pillar,            // which life-area
    pub scope: Option<String>,     // optional sub-area, e.g. work:report
    pub subject: String,           // the free-text "what you did"
    pub body: Option<String>,      // optional longer note
    pub trailers: Vec<Trailer>,    // zero or more key/value facts
    pub is_highlight: bool,        // starred as the day's highlight?
    pub created_at: DateTime<Utc>, // when it happened (stored in utc)
}

impl Pillar {
    // enum -> lowercase text; single source of truth for pillar names,
    // used for saving to the database and (via Display) for printing
    pub fn as_str(&self) -> &str {
        match self {
            Pillar::Body => "body",
            Pillar::Work => "work",
            Pillar::Make => "make",
            Pillar::Mind => "mind",
            Pillar::Life => "life",
        }
    }

    // text -> enum, for reading rows back from the database
    // every pillar is listed explicitly; the catch-all is the safety net
    // for unexpected text (our own data is always one of the five)
    pub fn from_str(s: &str) -> Pillar {
        match s {
            "body" => Pillar::Body,
            "work" => Pillar::Work,
            "make" => Pillar::Make,
            "mind" => Pillar::Mind,
            "life" => Pillar::Life,
            _ => Pillar::Life,
        }
    }
}

// teach Pillar to print itself by deferring to as_str (names live in one place)
impl fmt::Display for Pillar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// teach Commit to print as a clean log line, e.g.
// "10:13  work(report): finished the draft • Mins 120 • Spent 1450"
impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // time shown in local zone (stored as utc, converted for users)
        let time = self.created_at.with_timezone(&Local).format("%H:%M");

        // "work(report)" if there's a scope, else just "work"
        let head = match &self.scope {
            Some(s) => format!("{}({})", self.pillar, s),
            None => self.pillar.to_string(),
        };

        // a star prefix only for highlighted commits
        let star = if self.is_highlight { "✦ " } else { "" };

        // the main line; trailing ? propagates any write error
        write!(f, "{}  {}{}: {}", time, star, head, self.subject)?;

        // append each trailer, e.g. " • Mins 120"
        for t in &self.trailers {
            write!(f, " • {} {}", t.key, t.value)?;
        }

        Ok(())
    }
}
