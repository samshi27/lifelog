use chrono::{DateTime, Local, Utc};
use std::fmt;
use uuid::Uuid;
pub mod db;

#[derive(Debug, Clone, PartialEq)]
pub enum Pillar {
    Body,
    Work,
    Make,
    Mind,
    Life,
}

#[derive(Debug, Clone)]
pub struct Trailer {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub id: Uuid,
    pub pillar: Pillar,
    pub scope: Option<String>,
    pub subject: String,
    pub body: Option<String>,
    pub trailers: Vec<Trailer>,
    pub is_highlight: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum ParseError {
    EmptyInput,
    UnknownVerb(String),
    FlagMissingValue(String),
}

impl Commit {
    pub fn parse(line: &str) -> Result<Commit, ParseError> {
        let mut tokens: Vec<&str> = line.split_whitespace().collect();

        if tokens.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        let head = tokens.remove(0);
        let (verb, scope) = match head.split_once(':') {
            Some((v, s)) => (v, Some(s.to_string())),
            None => (head, None),
        };

        let pillar = match verb {
            "body" => Pillar::Body,
            "work" => Pillar::Work,
            "make" => Pillar::Make,
            "mind" => Pillar::Mind,
            "life" => Pillar::Life,
            other => return Err(ParseError::UnknownVerb(other.to_string())),
        };

        let mut trailers: Vec<Trailer> = Vec::new();
        let mut subject_tokens: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "-t" => {
                    let value = tokens
                        .get(i + 1)
                        .ok_or(ParseError::FlagMissingValue("-t".into()))?;
                    trailers.push(Trailer {
                        key: "Mins".into(),
                        value: value.to_string(),
                    });
                    i += 2;
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
                other => {
                    subject_tokens.push(other);
                    i += 1;
                }
            }
        }

        let subject = subject_tokens.join(" ");

        Ok(Commit {
            id: Uuid::new_v4(),
            pillar,
            scope,
            subject,
            body: None,
            trailers,
            is_highlight: false,
            created_at: Utc::now(),
        })
    }
}

impl Pillar {
    pub fn as_str(&self) -> &str {
        match self {
            Pillar::Body => "body",
            Pillar::Work => "work",
            Pillar::Make => "make",
            Pillar::Mind => "mind",
            Pillar::Life => "life",
        }
    }

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

impl fmt::Display for Pillar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let time = self.created_at.with_timezone(&Local).format("%H:%M");

        let head = match &self.scope {
            Some(s) => format!("{}({})", self.pillar, s),
            None => self.pillar.to_string(),
        };

        let star = if self.is_highlight { "✦ " } else { "" };

        write!(f, "{}  {}{}: {}", time, star, head, self.subject)?;

        for t in &self.trailers {
            write!(f, " • {} {}", t.key, t.value)?;
        }

        Ok(())
    }
}
