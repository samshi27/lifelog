// add in date/time types (chrono) and UUID generation (uuid)
use chrono::{DateTime, Local, Utc};
use uuid::Uuid;

// declare the `db` submodule - tells Rust that db.rs is part of this crate
pub mod db;

// the 5 life categories a commit can belong to
// an enum means a commit's pillar can only ever be one of these 5
// the compiler makes an invalid value impossible
#[derive(Debug, Clone, PartialEq)]
pub enum Pillar {
    Body,
    Work,
    Make,
    Mind,
    Life,
}

// one key/value fact attached to a commit; key="Mins", value="120"
#[derive(Debug, Clone)]
pub struct Trailer {
    pub key: String,
    pub value: String,
}

// the central unit: one thing you did and logged
// `Option<T>` means "maybe present" - scope and body can be absent (no null in Rust)
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Uuid,                  // unique id, generated fresh per commit
    pub pillar: Pillar,            // which life-area
    pub scope: Option<String>,     // optional sub-area, e.g. work:report
    pub subject: String,           // the free-text "what you did"
    pub body: Option<String>,      // optional longer note
    pub trailers: Vec<Trailer>,    // zero or more key/value facts
    pub is_highlight: bool,        // starred as the day's highlight?
    pub created_at: DateTime<Utc>, // when it happened (stored in utc)
}

// the ways parsing a typed line can fail
// each variant can carry detail - e.g. WHICH verb was unknown
#[derive(Debug)]
pub enum ParseError {
    EmptyInput,
    UnknownVerb(String),
    FlagMissingValue(String),
}

impl Commit {
    // turn a raw line like "work:report finished the draft -t 120"
    // into a Commit; returns Ok(commit) on success, or Err(ParseError) if the
    // input is malformed - the caller is forced to handle both
    pub fn parse(line: &str) -> Result<Commit, ParseError> {
        // split the line into whitespace-separated tokens
        let mut tokens: Vec<&str> = line.split_whitespace().collect();

        // nothing typed? that's an error, not a crash
        if tokens.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // the first token is the verb, possibly with a ":scope"
        let head = tokens.remove(0);
        // split_once(':') gives Some(("work","report")) or None if no colon
        let (verb, scope) = match head.split_once(':') {
            Some((v, s)) => (v, Some(s.to_string())),
            None => (head, None),
        };

        // map the verb word to a Pillar; an unrecognised verb is rejected
        let pillar = match verb {
            "body" => Pillar::Body,
            "work" => Pillar::Work,
            "make" => Pillar::Make,
            "mind" => Pillar::Mind,
            "life" => Pillar::Life,
            other => return Err(ParseError::UnknownVerb(other.to_string())),
        };

        // walk the remaining tokens: pull out -t / -s flags as trailers;
        // and collect everything else as the subject
        let mut trailers: Vec<Trailer> = Vec::new();
        let mut subject_tokens: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "-t" => {
                    // the value is the next token; .get() is safe: it returns
                    // None instead of crashing if there's no next token
                    // ok_or(...)? turns that None into our error and propagates it up
                    let value = tokens
                        .get(i + 1)
                        .ok_or(ParseError::FlagMissingValue("-t".into()))?;
                    trailers.push(Trailer {
                        key: "Mins".into(),
                        value: value.to_string(),
                    });
                    i += 2; // skip both the flag and its value
                }
                "-s" => {
                    let value = tokens
                        .get(i + 1)
                        .ok_or(ParseError::FlagMissingValue("-s".into()))?;
                    trailers.push(Trailer {
                        key: "Spent".into(),
                        value: value.to_string(),
                    });
                    i += 2;
                }
                // anything that isn't a flag is part of the subject
                other => {
                    subject_tokens.push(other);
                    i += 1;
                }
            }
        }

        // re-join the leftover words into the subject string
        let subject = subject_tokens.join(" ");

        // assemble the finished Commit. Ok(...) marks it a success
        Ok(Commit {
            id: Uuid::new_v4(), // a brand-new random uuid
            pillar,
            scope,
            subject,
            body: None,
            trailers,
            is_highlight: false,
            created_at: Utc::now(), // timestamp it right now, in utc
        })
    }
}

// std::fmt gives us the Display trait
use std::fmt;

impl Pillar {
    // enum - lowercase text; the single source of truth for pillar names
    // used both for saving to the database and (via Display) for printing
    pub fn as_str(&self) -> &str {
        match self {
            Pillar::Body => "body",
            Pillar::Work => "work",
            Pillar::Make => "make",
            Pillar::Mind => "mind",
            Pillar::Life => "life",
        }
    }
}

// teach Pillar how to print itself: just defer to as_str so the names
// live in exactly one place
impl fmt::Display for Pillar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// teach Commit how to print itself as a clean log line;
// e.g. "10:13 work(report): finished the Q2 draft . Mins 120 . Spent 1450"
impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // local time
        let time = self.created_at.with_timezone(&Local).format("%H:%M");

        // "work(report)" if there's a scope; else just "work"
        let head = match &self.scope {
            Some(s) => format!("{}({})", self.pillar, s),
            None => self.pillar.to_string(),
        };

        // a star prefix only for highlighted commits
        let star = if self.is_highlight { "✦ " } else { "" };

        // write the main line. The trailing `?` propagates up any write error
        write!(f, "{}  {}{}: {}", time, star, head, self.subject)?;

        // append each trailer, e.g. " . Mins 120"
        for t in &self.trailers {
            write!(f, " • {} {}", t.key, t.value)?;
        }

        Ok(())
    }
}
