use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Pillar {
    Make,
    Work,
    Move,
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
            "make" => Pillar::Make,
            "work" => Pillar::Work,
            "move" => Pillar::Move,
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
