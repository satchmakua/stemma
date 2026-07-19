//! Foundation layer for Stemma.
//!
//! This crate sits at the bottom of the dependency graph and is depended on by
//! every other `stem_*` crate. It deliberately knows nothing about linguistics —
//! it holds only the three things everything else needs:
//!
//! - [`ids`] — stable, typed, human-readable entity IDs.
//! - [`errors`] — the shared [`StemmaError`] type.
//! - [`validate`] — the [`Validate`] trait and the report it produces.
//! - [`rng`] — seeded, reproducible randomness.
//!
//! Randomness earns its place in the foundation because it is domain-free and
//! because every milestone from M1 onward needs the identical determinism
//! discipline (`DESIGN.md` §9.4).
//!
//! See `DESIGN.md` §4 for why the crate graph is shaped this way.

pub mod errors;
pub mod ids;
pub mod rng;
pub mod validate;

pub use errors::{Result, StemmaError};
pub use ids::{CognateSetId, EventId, LanguageId, PhonemeId, RuleId, WordId};
pub use rng::{RngDomain, StemmaRng, rng_for};
pub use validate::{Issue, Severity, Validate, ValidationReport};
