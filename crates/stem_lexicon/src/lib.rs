//! Lexicon: words, their forms, and their ancestry.
//!
//! **Empty until M2.** The crate exists now so the dependency edges are fixed
//! before there is code to move: the lexicon sits above [`stem_phonology`]
//! (a word is a sequence of phonemes) and below `stem_soundchange` (rules
//! transform words).
//!
//! What lands here, per `DESIGN.md` §8.3 and Roadmap M2:
//!
//! - `WordEntry` — form, phonemic form, glosses, part of speech, source, ancestor,
//!   cognate set, register, frequency, and evolution trace.
//! - `Lexicon` — the ordered collection, addressable by [`stem_core::WordId`].
//! - `CognateSet` — the cross-language grouping that survives forking and makes
//!   the cognate table of §10.3 possible.
//!
//! The load-bearing invariant to preserve when filling this in: **a word's
//! `cognate_set` is stable across every descendant language.** It is the thread
//! that ties `*takala` to Coastal `taal` and Highland `tazal`; break it and the
//! whole comparative view collapses.
