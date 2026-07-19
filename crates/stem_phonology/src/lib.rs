//! Phonology: what sounds a language has, and what shapes its roots may take.
//!
//! - [`features`] — distinctive features (`DESIGN.md` §7.1): what a sound *is*.
//! - [`phoneme`] — one contrastive segment.
//! - [`inventory`] — the set of them, and its validation.
//! - [`phonotactics`] — syllable templates and root lengths.
//! - [`generate`] — seeded, weighted root generation.
//!
//! Two distinctions worth internalising before changing anything here:
//!
//! 1. **[`SegmentKind`] is a phonotactic slot class; [`Feature::Consonantal`] is a
//!    phonological claim.** /w/ fills a `C` slot and is `[-consonantal]`. Both are
//!    true. Conflating them is what makes glides unrepresentable.
//! 2. **Absent is not minus.** A feature a segment does not value means "the
//!    question does not arise", never "no". See [`features`].
//!
//! **Not yet here:** prosody (stress, tone, length) and the sound-change engine.
//! Stress is syllable-scoped, not segment-scoped, so it hangs off
//! [`generate::Syllable`] in M3 rather than going into a feature bundle.

pub mod features;
pub mod generate;
pub mod inventory;
pub mod phoneme;
pub mod phonotactics;

pub use features::{Feature, FeatureBundle, FeatureParseError, Sign};
pub use generate::{Root, RootGenerator, Syllable};
pub use inventory::PhonemeInventory;
pub use phoneme::{Phoneme, SegmentKind};
pub use phonotactics::{Phonotactics, WeightedSyllableCount, WeightedTemplate};
