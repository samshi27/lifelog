// the engine crate: lifelog's shared brain
// holds the data types, the grammar parser, and the database layer
// the cli (and later the gui) both build on this

// declare the modules that make up this crate
pub mod db;
pub mod parse;
pub mod types;

// re-export the main types so callers can write `engine::Commit`
// instead of the longer `engine::types::Commit` for convenience;
// the split into files stays invisible to anyone using the crate
pub use parse::ParseError;
pub use types::{Commit, Pillar, Trailer};
