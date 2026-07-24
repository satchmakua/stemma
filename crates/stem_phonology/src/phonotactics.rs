//! Phonotactics: what shapes a root of this language may take
//! (`DESIGN.md` §7.1, ROADMAP M1).

use serde::{Deserialize, Serialize};
use stem_core::{Issue, Severity, Validate, ValidationReport};

use crate::PhonemeInventory;
use crate::phoneme::SegmentKind;

/// One position in a syllable template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A `C` slot: filled by any [`SegmentKind::Consonant`] in the inventory.
    Consonant,
    /// A `V` slot: filled by any [`SegmentKind::Vowel`] in the inventory.
    Vowel,
}

impl Slot {
    /// The [`SegmentKind`] eligible to fill this slot.
    pub fn kind(self) -> SegmentKind {
        match self {
            Self::Consonant => SegmentKind::Consonant,
            Self::Vowel => SegmentKind::Vowel,
        }
    }

    /// Parses one template character.
    pub fn from_char(c: char) -> Option<Slot> {
        match c {
            'C' => Some(Self::Consonant),
            'V' => Some(Self::Vowel),
            _ => None,
        }
    }
}

/// Ten, so an unweighted item sits mid-scale and an author can make one thing
/// rarer without rescaling everything else.
fn default_weight() -> u32 {
    10
}

/// One weighted syllable shape.
///
/// **Templates are concrete: `C` and `V` only, no parentheses.** ROADMAP writes
/// the system as `(C)V(C)`, and the fixture encodes exactly that — written out as
/// the four shapes it stands for, each with its own weight. Parenthesis sugar
/// needs an optional-slot probability that nothing in the design specifies, and
/// its expansion order would silently become part of the RNG draw order. Four
/// explicit shapes cost three lines of RON, remove an undefined semantics, and
/// give strictly more control: a common onset with a rare coda is one line here
/// and impossible with a single per-template rate.
///
/// Sugar can land in M2 as a load-time rewrite that changes no stored meaning.
/// This is a deliberate deviation from `DESIGN.md` §15 Ticket 4's "optional
/// groups"; ROADMAP wins on sequencing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedTemplate {
    /// The shape as authored, e.g. `"CVC"`.
    ///
    /// Stored as the raw string and parsed by [`Self::slots`], **not** by a
    /// `TryFrom<String>` in serde. `inventory.rs` states the convention:
    /// construction always succeeds, and `validate` reports what is wrong, so the
    /// CLI can load a broken fixture and *explain* it. A serde-level parse would
    /// abort the whole load on the first bad template, so a file with two typos
    /// would report one serde error instead of a full report.
    pub pattern: String,

    /// Relative likelihood of choosing this template. Exact integer; see
    /// [`crate::Phoneme::frequency_weight`].
    #[serde(default = "default_weight")]
    pub weight: u32,
}

impl WeightedTemplate {
    /// Builds a template with the default weight.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            weight: default_weight(),
        }
    }

    /// Sets the relative weight.
    #[must_use]
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// The parsed slots, or the first bad character with its position.
    pub fn slots(&self) -> Result<Vec<Slot>, TemplateError> {
        if self.pattern.is_empty() {
            return Err(TemplateError::Empty);
        }
        let mut slots = Vec::with_capacity(self.pattern.len());
        for (position, c) in self.pattern.chars().enumerate() {
            match Slot::from_char(c) {
                Some(slot) => slots.push(slot),
                None => return Err(TemplateError::BadSymbol { c, position }),
            }
        }
        if !slots.contains(&Slot::Vowel) {
            return Err(TemplateError::NoNucleus);
        }
        Ok(slots)
    }

    /// The longest run of consecutive consonant slots in this template — its
    /// heaviest cluster (M7, §17). A malformed template (already the
    /// `bad_template` Error) contributes **0**, the same skip discipline
    /// `check_against_inventory` uses, so a plausibility read never panics on and
    /// never piles a second finding onto a broken template.
    pub fn consonant_run(&self) -> usize {
        let Ok(slots) = self.slots() else {
            return 0;
        };
        let mut run = 0;
        let mut best = 0;
        for slot in slots {
            match slot {
                Slot::Consonant => {
                    run += 1;
                    best = best.max(run);
                }
                Slot::Vowel => run = 0,
            }
        }
        best
    }
}

/// Why a template string is not a syllable shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    /// The pattern is `""`.
    Empty,
    /// A character other than `C` or `V`.
    BadSymbol {
        /// The offending character.
        c: char,
        /// Its zero-based position in the pattern.
        position: usize,
    },
    /// No `V` slot — that is not a syllable.
    NoNucleus,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the template is empty"),
            // Parentheses are the mistake an author is most likely to make, since
            // the design doc itself writes `(C)V(C)`. Point at the fix.
            Self::BadSymbol {
                c: c @ ('(' | ')'),
                position,
            } => write!(
                f,
                "parenthesised templates are not supported (found `{c}` at position \
                 {position}); write out the shapes, e.g. `(C)V(C)` as the four \
                 templates CV, CVC, V, VC with their own weights"
            ),
            Self::BadSymbol { c, position } => write!(
                f,
                "unknown template symbol `{c}` at position {position}; only `C` and \
                 `V` are slots"
            ),
            Self::NoNucleus => f.write_str(
                "the template has no `V` slot, so it could generate a syllable with \
                 no nucleus (`DESIGN.md` §16.5)",
            ),
        }
    }
}

impl std::error::Error for TemplateError {}

/// One weighted root length.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedSyllableCount {
    /// How many syllables. Must be at least 1.
    pub count: u8,
    /// Relative likelihood of this length.
    #[serde(default = "default_weight")]
    pub weight: u32,
}

impl WeightedSyllableCount {
    /// Builds a syllable count with the default weight.
    pub fn new(count: u8) -> Self {
        Self {
            count,
            weight: default_weight(),
        }
    }

    /// Sets the relative weight.
    #[must_use]
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }
}

/// Onsets and codas of three-plus consonants are typologically marked — most
/// languages cap clusters at two (`DESIGN.md` §17, M7). Measured over the
/// **declared** root templates; surface clusters a sound change may create are
/// not counted (there is no resyllabifier yet — deferred).
pub const MARKED_CLUSTER_LEN: usize = 3;

/// A qualitative phonotactic-complexity band (§17, M7), over the declared root
/// templates. Enum, not a number: no float reaches output (§9.4). `Complex`
/// holds exactly when a template trips `large_consonant_cluster`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    /// No clusters — every template is (C)V(C) at most.
    Simple,
    /// Clusters present but capped at two (CCV, CVCC).
    Moderate,
    /// A run of three-plus consonants somewhere — typologically marked.
    Complex,
}

/// The phonotactic system of one language.
///
/// **[`Default`] is empty, not a standard `(C)V(C)` table.** A compiled-in default
/// would be a *semantic* default: a file that omitted `phonotactics:` would
/// generate roots from a table stored only in the binary, and a later release that
/// added one template would silently re-derive that user's whole lexicon from the
/// same seed with no diff in their file. `stem_genome` promises the opposite —
/// reproducible from the file alone. Empty means a file without phonotactics
/// simply cannot generate, and says so. The genome field stays
/// `#[serde(default)]`, so no saved project is stranded; it just cannot generate
/// until it declares a shape.
///
/// Order is authored order in both lists, and both reach a weighted-index prefix
/// sum, so order is part of the determinism contract. `Vec`, never a map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phonotactics {
    /// The syllable shapes a root may be built from.
    #[serde(default)]
    pub templates: Vec<WeightedTemplate>,
    /// How many syllables a root has.
    #[serde(default)]
    pub syllables_per_root: Vec<WeightedSyllableCount>,
}

impl Phonotactics {
    /// An empty system — declares no shapes and cannot generate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether anything is declared at all.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty() && self.syllables_per_root.is_empty()
    }

    /// The heaviest consonant cluster across the declared templates (M7). Malformed
    /// templates contribute 0 ([`WeightedTemplate::consonant_run`]); no templates
    /// → 0.
    pub fn max_consonant_run(&self) -> usize {
        self.templates
            .iter()
            .map(WeightedTemplate::consonant_run)
            .max()
            .unwrap_or(0)
    }

    /// The phonotactic-complexity band (§17), over the declared root templates.
    /// `Complex` holds exactly when a template trips `large_consonant_cluster`,
    /// so the band and the warning agree.
    pub fn complexity(&self) -> Complexity {
        match self.max_consonant_run() {
            n if n >= MARKED_CLUSTER_LEN => Complexity::Complex,
            2 => Complexity::Moderate,
            _ => Complexity::Simple,
        }
    }

    /// Whether a sequence of slot classes is admitted by some template.
    ///
    /// Exists so tests can check a generated root **without re-running the
    /// generator's own logic** — a test that recomputes the answer the same way
    /// proves nothing.
    pub fn admits_syllable(&self, kinds: &[SegmentKind]) -> bool {
        self.templates
            .iter()
            .any(|template| match template.slots() {
                // A malformed template admits nothing. It has already been reported as
                // an Error by `validate`, so swallowing it here cannot hide a fault —
                // and this is deliberately a `match`, not `is_ok_and`, so that is
                // visible at the call site.
                Err(_) => false,
                Ok(slots) => {
                    slots.len() == kinds.len()
                        && slots
                            .iter()
                            .zip(kinds)
                            .all(|(slot, kind)| slot.kind() == *kind)
                }
            })
    }

    /// A one-line summary for CLI output: `CV×45, CVC×35 · 1-3 syllables`.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "none declared".to_owned();
        }
        let shapes = self
            .templates
            .iter()
            .map(|t| format!("{}×{}", t.pattern, t.weight))
            .collect::<Vec<_>>()
            .join(", ");
        let counts: Vec<u8> = self.syllables_per_root.iter().map(|c| c.count).collect();
        match (counts.iter().min(), counts.iter().max()) {
            (Some(lo), Some(hi)) if lo == hi => format!("{shapes} · {lo} syllable(s)"),
            (Some(lo), Some(hi)) => format!("{shapes} · {lo}-{hi} syllables"),
            _ => format!("{shapes} · no syllable counts"),
        }
    }
}

impl Validate for Phonotactics {
    /// Checks everything decidable without the inventory. Slot satisfiability needs
    /// both halves of the language and lives in [`check_against_inventory`],
    /// because `Validate::validate(&self)` takes no context and M1 is not changing
    /// that signature.
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Notes, not Errors: a language may legitimately not have declared root
        // shapes yet. `RootGenerator::new` escalates these the moment generation is
        // actually requested.
        if self.templates.is_empty() {
            report.note(
                "no_templates",
                "no syllable templates are declared, so this language cannot generate roots yet",
            );
        }
        if self.syllables_per_root.is_empty() {
            report.note(
                "no_syllable_counts",
                "no root lengths are declared, so this language cannot generate roots yet",
            );
        }

        for template in &self.templates {
            if let Err(error) = template.slots() {
                report.push(
                    Issue::new(Severity::Error, "bad_template", error.to_string())
                        .about(&template.pattern),
                );
            }
            // §17 phonotactic-complexity warning: a run of three-plus consonants is
            // typologically marked. Warning, not Error — a bold design choice, not
            // a mistake. `consonant_run` skips malformed templates, so this never
            // doubles up on a `bad_template`.
            let run = template.consonant_run();
            if run >= MARKED_CLUSTER_LEN {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "large_consonant_cluster",
                        format!(
                            "the template `{}` packs {run} consonants in a row; onsets and \
                             codas of three or more are typologically marked (most languages \
                             cap clusters at two), so this is a bold design choice rather than \
                             a mistake — but worth motivating. (This counts the declared root \
                             template; surface clusters from sound change are not yet measured.)",
                            template.pattern
                        ),
                    )
                    .about(&template.pattern),
                );
            }
            if template.weight == 0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "zero_template_weight",
                        "a weight of 0 makes this template unselectable; remove it instead \
                         if that is what you mean",
                    )
                    .about(&template.pattern),
                );
            }
        }

        for entry in &self.syllables_per_root {
            if entry.count == 0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "zero_syllable_count",
                        "a root of zero syllables is the empty string",
                    )
                    .about(entry.count),
                );
            }
            if entry.weight == 0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "zero_count_weight",
                        "a weight of 0 makes this root length unselectable; remove it \
                         instead if that is what you mean",
                    )
                    .about(entry.count),
                );
            }
        }

        // The weights of each list must fit in a `u32`, for the same reason the
        // inventory's must: the sampler builds a `u32` prefix sum and errors on
        // overflow. Checked here so `stemma validate` cannot call a language fine
        // while `generate-roots` refuses it.
        for (total, what) in [
            (
                self.templates
                    .iter()
                    .try_fold(0u32, |acc, t| acc.checked_add(t.weight)),
                "syllable template",
            ),
            (
                self.syllables_per_root
                    .iter()
                    .try_fold(0u32, |acc, c| acc.checked_add(c.weight)),
                "root length",
            ),
        ] {
            if total.is_none() {
                report.error(
                    "weight_sum_overflow",
                    format!(
                        "the {what} weights sum past u32::MAX, which the weighted sampler \
                         cannot represent; scale them down"
                    ),
                );
            }
        }

        // A weighted draw over an all-zero list has no answer at all, which is a
        // different and worse fault than one dead entry.
        if !self.templates.is_empty() && self.templates.iter().all(|t| t.weight == 0) {
            report.error(
                "all_weights_zero",
                "every syllable template has weight 0, so no template can ever be chosen",
            );
        }
        if !self.syllables_per_root.is_empty()
            && self.syllables_per_root.iter().all(|c| c.weight == 0)
        {
            report.error(
                "all_weights_zero",
                "every root length has weight 0, so no length can ever be chosen",
            );
        }

        report
    }
}

/// The cross-check that needs both halves of the language.
///
/// Called from `LanguageGenome::validate` and absorbed under `"phonotactics"`, so
/// the codes land in the namespace their names imply. Catching this at validate
/// time is what lets [`crate::generate::RootGenerator::new`] be the only fallible
/// step and generation itself be total — there is no rejection sampling anywhere
/// in M1.
///
/// A validator that says the language is fine while `generate-roots` refuses would
/// be worse than no validator, so this deliberately runs on the ordinary
/// `stemma validate` path.
pub fn check_against_inventory(
    phonotactics: &Phonotactics,
    inventory: &PhonemeInventory,
) -> ValidationReport {
    let mut report = ValidationReport::new();

    let has_consonant = inventory.consonants().next().is_some();
    let has_vowel = inventory.vowels().next().is_some();

    for template in &phonotactics.templates {
        // A malformed template already produced `bad_template`; do not pile a
        // second, confusing error on the same line.
        let Ok(slots) = template.slots() else {
            continue;
        };
        for (needed, present, label) in [
            (Slot::Consonant, has_consonant, "consonant"),
            (Slot::Vowel, has_vowel, "vowel"),
        ] {
            if slots.contains(&needed) && !present {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "slot_unsatisfiable",
                        format!(
                            "this template needs a {label} slot, but the inventory declares \
                             no phoneme with `kind: {label}`"
                        ),
                    )
                    .about(&template.pattern),
                );
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Phoneme, SegmentKind};

    fn asterian_phonotactics() -> Phonotactics {
        Phonotactics {
            templates: vec![
                WeightedTemplate::new("CV").with_weight(45),
                WeightedTemplate::new("CVC").with_weight(35),
            ],
            syllables_per_root: vec![
                WeightedSyllableCount::new(1).with_weight(25),
                WeightedSyllableCount::new(2).with_weight(55),
            ],
        }
    }

    #[test]
    fn a_well_formed_system_reports_no_errors() {
        let report = asterian_phonotactics().validate();
        assert!(report.is_ok(), "{report}");
    }

    #[test]
    fn templates_parse_into_slots_in_order() {
        let slots = WeightedTemplate::new("CVC").slots().unwrap();
        assert_eq!(slots, [Slot::Consonant, Slot::Vowel, Slot::Consonant]);
    }

    #[test]
    fn a_parenthesised_template_is_rejected_with_a_pointer_to_the_explicit_form() {
        let error = WeightedTemplate::new("(C)V(C)").slots().unwrap_err();
        let text = error.to_string();
        assert!(text.contains("parenthesised"), "{text}");
        assert!(text.contains("CV, CVC, V, VC"), "{text}");
    }

    #[test]
    fn a_template_with_no_nucleus_is_an_error() {
        assert_eq!(
            WeightedTemplate::new("CC").slots().unwrap_err(),
            TemplateError::NoNucleus
        );
    }

    #[test]
    fn an_unknown_template_symbol_names_the_character_and_its_position() {
        let error = WeightedTemplate::new("CVX").slots().unwrap_err();
        assert_eq!(
            error,
            TemplateError::BadSymbol {
                c: 'X',
                position: 2
            }
        );
        let text = error.to_string();
        assert!(text.contains("`X`"), "{text}");
        assert!(text.contains("position 2"), "{text}");
    }

    #[test]
    fn an_empty_template_is_an_error() {
        assert_eq!(
            WeightedTemplate::new("").slots().unwrap_err(),
            TemplateError::Empty
        );
    }

    /// Guards the `inventory.rs` convention: construction succeeds, `validate`
    /// explains. A serde-level parse would abort the load instead.
    #[test]
    fn a_bad_template_is_a_validation_issue_not_a_construction_failure() {
        let phonotactics = Phonotactics {
            templates: vec![WeightedTemplate::new("(C)V")],
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "bad_template"),
            "{report}"
        );
    }

    // --- M7: the phonotactic-complexity warning and band ---

    fn with_templates(patterns: &[&str]) -> Phonotactics {
        Phonotactics {
            templates: patterns.iter().map(|p| WeightedTemplate::new(*p)).collect(),
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        }
    }

    #[test]
    fn a_template_with_a_three_consonant_cluster_warns_and_scores_complex() {
        let phonotactics = with_templates(&["CCCVC"]);
        let report = phonotactics.validate();
        assert!(
            report.is_ok(),
            "a marked cluster is a Warning, not an Error: {report}"
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "large_consonant_cluster"),
            "{report}"
        );
        assert_eq!(phonotactics.complexity(), Complexity::Complex);
    }

    #[test]
    fn a_malformed_template_contributes_no_cluster_run() {
        // `CCCC` has no nucleus (already `bad_template`); it must not also panic or
        // pile on a cluster warning.
        let phonotactics = with_templates(&["CCCC"]);
        assert_eq!(phonotactics.max_consonant_run(), 0);
        let report = phonotactics.validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "large_consonant_cluster"),
            "no second finding on a broken template: {report}"
        );
    }

    #[test]
    fn the_reference_c_v_c_phonotactics_trips_no_cluster_warning_and_scores_simple() {
        let phonotactics = with_templates(&["CV", "CVC", "V", "VC"]);
        let report = phonotactics.validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "large_consonant_cluster"),
            "{report}"
        );
        assert_eq!(phonotactics.complexity(), Complexity::Simple);
    }

    #[test]
    fn a_capped_two_cluster_scores_moderate() {
        assert_eq!(with_templates(&["CCVC"]).complexity(), Complexity::Moderate);
    }

    #[test]
    fn an_empty_system_notes_that_it_cannot_generate_but_is_not_invalid() {
        let report = Phonotactics::new().validate();
        assert!(report.is_ok(), "{report}");
        assert!(
            report.issues.iter().any(|i| i.code == "no_templates"),
            "{report}"
        );
        assert!(
            report.issues.iter().any(|i| i.code == "no_syllable_counts"),
            "{report}"
        );
    }

    #[test]
    fn a_zero_template_weight_is_an_error() {
        let phonotactics = Phonotactics {
            templates: vec![
                WeightedTemplate::new("CV").with_weight(0),
                WeightedTemplate::new("CVC").with_weight(5),
            ],
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "zero_template_weight"),
            "{report}"
        );
    }

    #[test]
    fn a_zero_syllable_count_is_an_error() {
        let phonotactics = Phonotactics {
            templates: vec![WeightedTemplate::new("CV")],
            syllables_per_root: vec![WeightedSyllableCount::new(0)],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "zero_syllable_count"),
            "{report}"
        );
    }

    /// `stemma validate` must not call a language fine while `generate-roots`
    /// refuses it — the guarantee the inventory's own overflow check exists for.
    #[test]
    fn template_weights_summing_past_u32_max_are_an_error() {
        let phonotactics = Phonotactics {
            templates: vec![
                WeightedTemplate::new("CV").with_weight(3_000_000_000),
                WeightedTemplate::new("CVC").with_weight(3_000_000_000),
            ],
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "weight_sum_overflow"),
            "{report}"
        );
    }

    #[test]
    fn syllable_count_weights_summing_past_u32_max_are_an_error() {
        let phonotactics = Phonotactics {
            templates: vec![WeightedTemplate::new("CV")],
            syllables_per_root: vec![
                WeightedSyllableCount::new(1).with_weight(3_000_000_000),
                WeightedSyllableCount::new(2).with_weight(3_000_000_000),
            ],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "weight_sum_overflow"),
            "{report}"
        );
    }

    #[test]
    fn all_weights_zero_is_its_own_error() {
        let phonotactics = Phonotactics {
            templates: vec![WeightedTemplate::new("CV").with_weight(0)],
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        };
        let report = phonotactics.validate();
        assert!(
            report.errors().any(|i| i.code == "all_weights_zero"),
            "{report}"
        );
    }

    #[test]
    fn a_template_needing_a_consonant_the_inventory_lacks_is_reported() {
        let vowels_only =
            PhonemeInventory::from_phonemes([Phoneme::new("ph_a", "a", SegmentKind::Vowel)]);
        let report = check_against_inventory(&asterian_phonotactics(), &vowels_only);
        assert!(
            report.errors().any(|i| i.code == "slot_unsatisfiable"),
            "{report}"
        );
    }

    #[test]
    fn a_satisfiable_system_cross_checks_clean() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let report = check_against_inventory(&asterian_phonotactics(), &inventory);
        assert!(report.is_ok(), "{report}");
    }

    #[test]
    fn admits_syllable_matches_shape_and_length_exactly() {
        let p = asterian_phonotactics();
        use SegmentKind::{Consonant, Vowel};
        assert!(p.admits_syllable(&[Consonant, Vowel]));
        assert!(p.admits_syllable(&[Consonant, Vowel, Consonant]));
        assert!(!p.admits_syllable(&[Vowel]), "V is not declared");
        assert!(
            !p.admits_syllable(&[Vowel, Consonant]),
            "VC is not declared"
        );
        assert!(!p.admits_syllable(&[Consonant, Consonant]));
        assert!(!p.admits_syllable(&[]));
    }

    #[test]
    fn a_malformed_template_admits_nothing() {
        let phonotactics = Phonotactics {
            templates: vec![WeightedTemplate::new("(C)V")],
            syllables_per_root: vec![WeightedSyllableCount::new(1)],
        };
        assert!(!phonotactics.admits_syllable(&[SegmentKind::Consonant, SegmentKind::Vowel]));
    }
}
