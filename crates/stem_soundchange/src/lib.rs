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
pub mod dsl;
mod resolve;
pub mod rule;
pub mod view;

pub use apply::{Evolution, apply_rules};
pub use check::{check_against_language, check_applied_log};
pub use dsl::parse_rule_set;
pub use rule::{Change, EnvItem, Environment, Position, RuleSet, SegmentPattern, SoundChangeRule};
pub use view::{MatchView, TraceView, render_derivation, view};

#[cfg(test)]
mod guard {
    /// **The engine is the source of truth, and morphology must not leak into it**
    /// (`CLAUDE.md`; `docs/adr/0010`). M8 keeps `apply_rules` a pure phonological
    /// function by making an inflected cell an *ordinary* `WordEntry`: the engine
    /// gains conditioned allomorphy for free precisely because it never learns what
    /// a morpheme is. This scan enforces that by reading the sources — crude, and
    /// honest about being so, the same shape as `stem_export`'s clock/map/float
    /// scan and the cognate-mint scan.
    ///
    /// The banned tokens are the morphology *types and operations* in
    /// `stem_lexicon::morpheme`. The `WordEntry.morphemes` field (lowercase) is
    /// **allowed** — the engine clones it verbatim, which is how a cell's
    /// composition survives evolution — so the capitalised type names and the
    /// lowercase verb names are matched, never the field.
    ///
    /// M9 adds the semantic twin, [`the_engine_never_references_semantics`].
    #[test]
    fn the_engine_never_references_morphology() {
        for (name, src) in [
            ("apply.rs", include_str!("apply.rs")),
            ("check.rs", include_str!("check.rs")),
            ("resolve.rs", include_str!("resolve.rs")),
            ("rule.rs", include_str!("rule.rs")),
            ("view.rs", include_str!("view.rs")),
            ("lib.rs", include_str!("lib.rs")),
        ] {
            for (n, line) in src.lines().enumerate() {
                // This module names the banned tokens itself (as string literals and
                // prose); stop scanning `lib.rs` at the guard so it cannot flag its
                // own source.
                if line.contains("mod guard") {
                    break;
                }
                let code = line.split("//").next().unwrap_or("");
                for banned in [
                    "compose",
                    "inflect",
                    "Morpheme", // catches Morpheme, MorphemeId, MorphemeRef, MorphemeRole
                    "Morphology",
                    "Paradigm",
                    "morphological",
                ] {
                    assert!(
                        !code.contains(banned),
                        "{name}:{} references morphology (`{banned}`); the engine must stay \
                         phonology-only so `apply_rules` keeps giving cross-boundary sound \
                         change for free (`docs/adr/0010`)",
                        n + 1
                    );
                }
            }
        }
    }

    /// **The engine is the source of truth, and semantics must not leak into it
    /// either** (M9, `docs/adr/0011`). The morphology guard's twin, and load-bearing
    /// for the same reason: `apply_rules` stays a pure phonological function of five
    /// arguments precisely because it never learns what a *meaning* is. A drifted
    /// word survives further sound change for free because `apply.rs` clones each
    /// entry whole — not because the engine understands senses.
    ///
    /// The `WordEntry.senses` / `sense_history` **fields** (lowercase) are allowed:
    /// the engine carries them verbatim through a clone, exactly as it carries
    /// `morphemes`. What is banned is naming a semantic *type or operation*.
    #[test]
    fn the_engine_never_references_semantics() {
        for (name, src) in [
            ("apply.rs", include_str!("apply.rs")),
            ("check.rs", include_str!("check.rs")),
            ("resolve.rs", include_str!("resolve.rs")),
            ("rule.rs", include_str!("rule.rs")),
            ("view.rs", include_str!("view.rs")),
            ("lib.rs", include_str!("lib.rs")),
        ] {
            for (n, line) in src.lines().enumerate() {
                if line.contains("mod guard") {
                    break;
                }
                let code = line.split("//").next().unwrap_or("");
                for banned in [
                    "Semantic", // SemanticNode, SemanticSpace, SemanticNodeId
                    "SenseRef", // the fields `senses` / `sense_history` stay legal
                    "SenseHistory",
                    "SenseShift",
                    "SenseChain",
                    "Drift", // DriftEvent, DriftSet, DriftMechanism, DriftOutcome
                    // The lowercase *operations*, named in full. The capitalised
                    // entries above are case-sensitive, so `check_against_semantics`
                    // and `check_drift_against_language` would slip past them —
                    // and those are precisely the functions that would signal the
                    // engine had started reasoning about meaning. Bare `sense` is
                    // deliberately NOT banned: the engine legitimately carries the
                    // `senses` / `sense_history` fields through its entry clone.
                    "apply_drift",
                    "sense_chains",
                    "check_against_semantics",
                    "check_drift_against_language",
                ] {
                    assert!(
                        !code.contains(banned),
                        "{name}:{} references semantics (`{banned}`); the engine must stay \
                         phonology-only — meaning drift is a separate, authored history and \
                         `apply_rules` must not learn about it (`docs/adr/0011`)",
                        n + 1
                    );
                }
            }
        }
    }
}
