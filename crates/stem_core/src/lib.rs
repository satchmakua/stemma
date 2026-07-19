//! Foundation layer for Stemma.
//!
//! This crate sits at the bottom of the dependency graph and is depended on by
//! every other `stem_*` crate. It deliberately knows nothing about linguistics —
//! it holds only the three things everything else needs:
//!
//! - [`ids`] — stable, typed, human-readable entity IDs.
//! - [`errors`] — the shared [`StemmaError`] type.
//! - [`validate`] — the [`Validate`] trait and the report it produces.
//!
//! See `DESIGN.md` §4 for why the crate graph is shaped this way.

pub mod errors;
pub mod ids;
pub mod validate;

pub use errors::{Result, StemmaError};
pub use ids::{CognateSetId, EventId, LanguageId, PhonemeId, RuleId, WordId};
pub use validate::{Issue, Severity, Validate, ValidationReport};
