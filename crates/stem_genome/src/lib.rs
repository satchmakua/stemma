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

pub mod culture;
pub mod edit;
pub mod etymology;
pub mod grammar;
pub mod language;
pub mod lineage;
pub mod paradigm;
pub mod profile;
pub mod say;
pub mod semantics_view;

pub use culture::{absences, render_culture};
pub use edit::{Edit, EditOutcome, apply as apply_edit, move_rule, new_errors, new_issues};
pub use etymology::render_etymology;
pub use grammar::render_grammar;
pub use language::LanguageGenome;
pub use lineage::{
    BranchSpec, CognateCell, CognateColumn, CognateCoverage, CognateRow, CognateTable, LineageEdge,
    LineageGraph, grow_family, render_cognate_table, render_family,
};
pub use paradigm::render_paradigm;
pub use say::{render_sentence, say};

/// The spaces needed to pad `text` out to `width` **display characters**.
///
/// One definition, shared by every renderer in this crate. Two reasons it is not
/// inlined at each call site, both learned from copies that had already drifted:
///
/// 1. **Char count, never byte length.** `{:<20}` and `text.len()` count bytes, and
///    these columns carry IPA (`ŋ`, `ɣ`), `§`, and free-text glosses in any script.
///    Padding by bytes silently misaligns exactly the output this project exists to
///    print.
/// 2. **`saturating_sub`, never bare `-`.** Several copies subtracted directly,
///    which is safe only while `width` is the max over precisely the strings being
///    padded — an invariant nothing stated and nothing checked. A label computed
///    rather than listed in the width array would panic on overflow in a
///    debug build.
pub(crate) fn pad(text: &str, width: usize) -> String {
    " ".repeat(width.saturating_sub(text.chars().count()))
}
pub use profile::{
    HistoricalDepth, MorphologicalIrregularity, NOT_MODELLED, NotModelled, PlausibilityProfile,
    SemanticDrift, VocabularyShaping, render_profile,
};
pub use semantics_view::{render_sense_history, render_word_history};
// The phonology bands the profile is built from, re-exported so a consumer needs
// one crate (the `render_family`/`CognateTable` convenience).
pub use stem_phonology::{Complexity, Rarity};
