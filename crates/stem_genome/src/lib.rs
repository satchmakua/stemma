//! The Language Genome — Stemma's central abstraction (`DESIGN.md` §8.1).
//!
//! One [`LanguageGenome`] is one language at one point in its history: proto,
//! intermediate stage, daughter, dialect, register, or creole. Forking copies a
//! genome and gives the copy its own history; evolution transforms a genome into
//! a new one. Nothing is shared by reference between languages, because two
//! languages that look alike still have separate histories, and history is the
//! thing this program is about.
//!
//! ## Why this crate exists
//!
//! The design sketch (§9.2) puts `language.rs` inside `stem_core`. It cannot live
//! there: the genome *owns* a phonology, a lexicon, and a rule history, so it must
//! depend on `stem_phonology`, `stem_lexicon`, and `stem_soundchange` — while
//! those crates depend on `stem_core` for IDs and errors. Putting the aggregate in
//! `stem_core` makes the crate graph cyclic. Splitting it out keeps the graph a
//! DAG and keeps `stem_core` a true foundation. See `docs/adr/0002-crate-layering.md`.

pub mod language;

pub use language::LanguageGenome;
