// turning a typed line into a Commit, with formatted errors

use crate::types::{Commit, Pillar, Trailer};
use chrono::Utc;
use uuid::Uuid;

// the ways parsing a line can fail; each variant carries detail
// e.g. WHICH verb was unknown — so the message can be helpful
#[derive(Debug)]
pub enum ParseError {
    EmptyInput,
    UnknownVerb(String),
    FlagMissingValue(String),
}

impl Commit {
    // turn a raw line like "work:report finished the draft -t 120" into a Commit
    // Ok(commit) on success, Err(ParseError) if malformed — caller must handle both
    pub fn parse(line: &str) -> Result<Commit, ParseError> {
        // split into whitespace-separated tokens.
        let mut tokens: Vec<&str> = line.split_whitespace().collect();

        // nothing typed? an error, not a crash.
        if tokens.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // first token is the verb, possibly with a ":scope" welded on
        let head = tokens.remove(0);
        let (verb, scope) = match head.split_once(':') {
            Some((v, s)) => (v, Some(s.to_string())),
            None => (head, None),
        };

        // map the verb to a pillar; an unrecognised verb is rejected
        let pillar = match verb {
            "body" => Pillar::Body,
            "work" => Pillar::Work,
            "make" => Pillar::Make,
            "mind" => Pillar::Mind,
            "life" => Pillar::Life,
            other => return Err(ParseError::UnknownVerb(other.to_string())),
        };

        // walk remaining tokens: pull -t / -s flags into trailers, rest is subject
        let mut trailers: Vec<Trailer> = Vec::new();
        let mut subject_tokens: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "-t" => {
                    // value is the next token; .get() is safe (None if missing),
                    // ok_or(...)? turns None into our error and propagates it up
                    let value = tokens
                        .get(i + 1)
                        .ok_or(ParseError::FlagMissingValue("-t".into()))?;
                    trailers.push(Trailer {
                        key: "Mins".into(),
                        value: value.to_string(),
                    });
                    i += 2; // skip flag and its value
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

        // re-join the leftover words into the subject
        let subject = subject_tokens.join(" ");

        // assemble the finished commit.
        Ok(Commit {
            id: Uuid::new_v4(), // new random uuid
            pillar,
            scope,
            subject,
            body: None,
            trailers,
            is_highlight: false,
            created_at: Utc::now(), // timestamp in utc
        })
    }
}
