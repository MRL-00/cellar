//! SQL parsing and reference analysis for Cellar, built on `sqlparser-rs`.
//!
//! Today this crate's job is reference detection for the "Find Usages" feature:
//! given the text of a view definition, routine body, trigger definition, or
//! constraint, decide whether it *really* references a given table or column —
//! structurally, via the SQL tokenizer, never by naive substring matching.

mod references;

pub use references::{find_references, Reference};
