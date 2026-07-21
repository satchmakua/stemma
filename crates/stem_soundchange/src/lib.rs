//! Sound change: ordered, traceable phonological transformation.
//!
//! The heart of the program — the thing that makes a generated word *strange for
//! a reason* rather than merely strange (`DESIGN.md` §7.2, §8.4, §11).
//!
//! - [`rule`] — rules as data: target, environment, change. Structs, no DSL
//!   (§20.4; the parser is M10's, designed against these semantics).
//! - [`apply`] — **the application contract**, normative in its module doc, and
//!   [`apply::apply_rules`], the entry point.
//! - [`resolve`] — how an output bundle becomes a symbol, including minting a
//!   phoneme the language did not have (private; its behaviour is visible through
//!   [`stem_lexicon::SymbolResolution`]).
//! - [`check`] — the rule checks that need the language.
//! - [`view`] — §11.2's JSON shape and `stemma trace`'s text.
//!
//! # The two constraints this crate was built around
//!
//! 1. **Every application produces a trace.** Tracing is not instrumentation; it
//!    is the product (§3.3, §10.2). A rule that transforms correctly but silently
//!    is a bug. Every matched site is recorded — applied or refused — on the
//!    word's [`stem_lexicon::Derivation`].
//! 2. **Rules match features, not letters.** "Voiceless stops voice between
//!    vowels" is one rule over `[-sonorant, -continuant, -voice]`, never an
//!    enumeration of `p`, `t`, `k`. Matching is `FeatureBundle::subsumes` over
//!    the working inventory, with no membership shortcut — a segment minted by an
//!    earlier rule must be matchable by a later one, or feeding order is broken.
//!
//! # No RNG
//!
//! `apply_rules` is a pure function of its five arguments. `RngDomain` gains no
//! variant at M3, and the strongest determinism claim the project can make —
//! same input, same output, no seed involved at all — is free.

pub mod apply;
pub mod check;
mod resolve;
pub mod rule;
pub mod view;

pub use apply::{Evolution, apply_rules};
pub use check::{check_against_language, check_applied_log};
pub use rule::{Change, EnvItem, Environment, Position, RuleSet, SegmentPattern, SoundChangeRule};
pub use view::{MatchView, TraceView, render_derivation, view};
