//! Sound change: ordered, traceable phonological transformation.
//!
//! **Empty until M3.** This is the heart of the program — the thing that makes a
//! generated word *strange for a reason* rather than merely strange.
//!
//! What lands here, per `DESIGN.md` §8.4, §11 and Roadmap M3:
//!
//! - `SoundChangeRule` — target pattern, transformation, environment, exceptions,
//!   probability, chronology.
//! - Segment and environment matching against feature bundles.
//! - Ordered application over a lexicon.
//! - `RuleApplicationTrace` — input, output, rule, matched spans, changed segments.
//!
//! Two constraints that are easy to lose and expensive to retrofit:
//!
//! 1. **Every application produces a trace.** Tracing is not instrumentation to be
//!    added later; it is the product (`DESIGN.md` §3.3, §10.2). A rule that
//!    transforms silently is a bug even when its output is correct.
//! 2. **Rules match on features, not letters.** "Voiceless stops voice between
//!    vowels" is one rule over `[-sonorant, -continuant, -voice]`, not an
//!    enumeration of `p`, `t`, `k`. This is why M1's feature bundles must land
//!    before this crate is filled in.
