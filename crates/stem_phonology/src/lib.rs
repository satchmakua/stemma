//! Phonology: what sounds a language has.
//!
//! At M0 this is the inventory only — the set of contrastive segments, each with
//! an IPA form, a romanisation, a broad [`SegmentKind`], and a frequency weight.
//!
//! **Not yet here:** the feature bundles of `DESIGN.md` §7.1 (`[-sonorant, -voice]`
//! and friends), phonotactic templates, and prosody. Those arrive in M1, and the
//! sound-change engine (M3) depends on them — a rule like "voiceless stops become
//! voiced between vowels" is unwritable against `SegmentKind` alone. Keep that
//! ordering: features before rules.

pub mod inventory;
pub mod phoneme;

pub use inventory::PhonemeInventory;
pub use phoneme::{Phoneme, SegmentKind};
