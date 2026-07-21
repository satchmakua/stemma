//! The genome itself.

use serde::{Deserialize, Serialize};
use stem_core::{LanguageId, Result, Severity, StemmaError, Validate, ValidationReport};
use stem_lexicon::Lexicon;
use stem_phonology::{PhonemeInventory, Phonotactics, Prosody};
use stem_soundchange::{RuleSet, SoundChangeRule};

/// Everything that defines one language at one point in its history.
///
/// At M0 a genome carries identity, lineage position, its RNG seed, and a
/// phoneme inventory. The remaining components of `DESIGN.md` §8.1 attach as
/// their milestones land: phonotactics and prosody (M1), lexicon (M2), rule
/// history (M3), morphology (M7), semantics (M8), writing systems (M9).
///
/// New fields must be `#[serde(default)]` so that fixtures and saved projects
/// written by an earlier milestone keep loading. A project file is a user's work;
/// a schema change must not strand it.
///
/// `deny_unknown_fields` does not conflict with that rule — `#[serde(default)]` is
/// what makes an *old* file load in *new* code, and that is the direction the rule
/// is about. What it buys is that a misspelled `sed:` cannot silently take the
/// default seed of 0 and hand back a completely different language with no
/// diagnostic (`DESIGN.md` §9.4). The same reasoning as [`stem_phonology::Phoneme`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageGenome {
    /// Stable identity within the lineage graph.
    pub id: LanguageId,

    /// Display name, e.g. `"Proto-Asterian"`.
    pub name: String,

    /// The language this one descends from, or `None` for a proto-language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<LanguageId>,

    /// Simulated years elapsed since the root of the lineage.
    ///
    /// This is narrative time, not wall-clock time — it orders the sound-change
    /// timeline (§10.4) and gives plausibility checks something to reason about
    /// ("this much irregularity after only 100 years is a lot").
    #[serde(default)]
    pub lineage_depth_years: i32,

    /// The RNG seed for every stochastic step taken from this genome.
    ///
    /// Determinism is a hard requirement (`DESIGN.md` §9.4): re-running a pipeline
    /// with the same seed must reproduce the same language, byte for byte.
    /// The seed is stored *with* the language so a result is always reproducible
    /// from the file alone, not from whatever the CLI happened to be invoked with.
    #[serde(default)]
    pub seed: u64,

    /// The contrastive sounds of the language.
    #[serde(default)]
    pub phonemes: PhonemeInventory,

    /// What shapes a root of this language may take (ROADMAP M1).
    ///
    /// `#[serde(default)]` per this type's own contract, and the default is
    /// **empty** — see [`Phonotactics`]. A pre-M1 file loads unchanged and simply
    /// reports that it has no root shape yet, rather than silently inheriting a
    /// table from whatever version of Stemma happens to be running. That matters
    /// because a compiled-in default would make generation depend on the binary
    /// rather than on the file, which is exactly what `seed`'s contract forbids.
    #[serde(default)]
    pub phonotactics: Phonotactics,

    /// This language's words (ROADMAP M2).
    ///
    /// `#[serde(default)]` per this type's contract, and the default is **empty** —
    /// a pre-M2 file, the reference fixture included, loads unchanged and reports
    /// that it has no lexicon yet. `skip_serializing_if` keeps `stemma convert`
    /// from adding an empty `lexicon: []` to a file that never had one, so a round
    /// trip of `proto_asterian.ron` stays byte-identical.
    #[serde(default, skip_serializing_if = "Lexicon::is_empty")]
    pub lexicon: Lexicon,

    /// How this language places stress (`DESIGN.md` §8.1's `prosody`, ROADMAP M3).
    ///
    /// `skip_serializing_if` the default, so a language with no stress system —
    /// every pre-M3 file — round-trips byte-identically.
    #[serde(default, skip_serializing_if = "Prosody::is_unspecified")]
    pub prosody: Prosody,

    /// The sound changes that produced this language's current lexicon.
    ///
    /// **Past tense, and that is the whole design.** Not a queue: a `rules:` field
    /// meaning "rules to apply" makes double application representable and needs
    /// an applied-up-to cursor that can desynchronise from the forms. Applying is
    /// an explicit operation over a separate [`RuleSet`] file; this is the record
    /// of what already happened — the "rule history (M3)" this type's own docs
    /// have promised since M0.
    ///
    /// An ordered `Vec` with stable indices — never a set, never a map, never
    /// re-sorted. `RuleApplication::index` indexes into it. Ids may repeat: real
    /// histories apply intervocalic voicing twice, in different strata.
    ///
    /// §8.5's `HistoricalEvent` is deferred to M4: of its five fields the only one
    /// §10.4's timeline reads is the date, and that ships as `chronology_years` on
    /// the rule. A union with one inhabited variant is scaffolding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_rules: Vec<SoundChangeRule>,

    /// Free-form authorial notes. Not interpreted by the engine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl LanguageGenome {
    /// Builds a proto-language: no parent, zero lineage depth.
    pub fn proto(id: impl Into<LanguageId>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent: None,
            lineage_depth_years: 0,
            seed: 0,
            phonemes: PhonemeInventory::new(),
            phonotactics: Phonotactics::new(),
            lexicon: Lexicon::new(),
            prosody: Prosody::new(),
            applied_rules: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Sets the prosodic system.
    #[must_use]
    pub fn with_prosody(mut self, prosody: Prosody) -> Self {
        self.prosody = prosody;
        self
    }

    /// Sets the RNG seed.
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Sets the phoneme inventory.
    #[must_use]
    pub fn with_phonemes(mut self, phonemes: PhonemeInventory) -> Self {
        self.phonemes = phonemes;
        self
    }

    /// Sets the phonotactic system.
    #[must_use]
    pub fn with_phonotactics(mut self, phonotactics: Phonotactics) -> Self {
        self.phonotactics = phonotactics;
        self
    }

    /// Sets the lexicon, replacing any existing one.
    #[must_use]
    pub fn with_lexicon(mut self, lexicon: Lexicon) -> Self {
        self.lexicon = lexicon;
        self
    }

    /// True when this language has no ancestor in the project.
    pub fn is_proto(&self) -> bool {
        self.parent.is_none()
    }

    /// A one-line summary for CLI output.
    pub fn summary(&self) -> String {
        let lineage = match &self.parent {
            Some(parent) => format!("< {parent}, +{}y", self.lineage_depth_years),
            None => "proto".to_owned(),
        };
        format!(
            "{} ({}) — {} phonemes ({}C/{}V), {lineage}, seed {}",
            self.name,
            self.id,
            self.phonemes.len(),
            self.phonemes.consonants().count(),
            self.phonemes.vowels().count(),
            self.seed,
        )
    }
}

impl Validate for LanguageGenome {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.id.is_empty() {
            report.error("empty_id", "the language has an empty id");
        }
        if self.name.trim().is_empty() {
            report.error("empty_name", "the language has an empty name");
        }

        // A language descending from itself would make the lineage graph cyclic
        // and every ancestor walk non-terminating.
        if self.parent.as_ref() == Some(&self.id) {
            report.error("self_parent", "the language is its own parent");
        }

        if self.lineage_depth_years < 0 {
            report.error(
                "negative_lineage_depth",
                format!(
                    "lineage depth is {} years; time runs forwards",
                    self.lineage_depth_years
                ),
            );
        }

        // A daughter at depth 0 has had no time to diverge from its parent.
        if self.parent.is_some() && self.lineage_depth_years == 0 {
            report.warn(
                "no_elapsed_time",
                "this language has a parent but zero elapsed years, so no change could have \
                 accumulated",
            );
        }

        // A `LanguageId` reaches every minted `CognateSetId` verbatim and every CSV
        // cell. A non-portable id does not break Stemma — only interchange — so it
        // warns rather than erroring (§17).
        if !self.id.is_empty() && !stem_lexicon::is_portable_id(self.id.as_str()) {
            report.push(
                stem_core::Issue::new(
                    Severity::Warning,
                    "id_not_portable",
                    "this language id contains characters outside [A-Za-z0-9_-]; it reaches \
                     every cognate-set id and every exported row, where a CLDF consumer \
                     would reject it",
                )
                .about(&self.id),
            );
        }

        report.absorb("phonology", self.phonemes.validate());
        report.absorb("phonotactics", self.phonotactics.validate());
        // Needs both halves of the language, so it cannot live in either type's
        // own `validate` — `Validate::validate(&self)` takes no context.
        report.absorb(
            "phonotactics",
            stem_phonology::phonotactics::check_against_inventory(
                &self.phonotactics,
                &self.phonemes,
            ),
        );

        report.absorb("lexicon", self.lexicon.validate());
        // Same shape as the phonotactics cross-check above, and for the same
        // reason: it needs both halves of the language, and `Validate::validate`
        // takes no context.
        report.absorb(
            "lexicon",
            stem_lexicon::check_against_inventory(&self.lexicon, &self.phonemes),
        );

        // Not optional: putting the rule checks only in a free function the gate
        // never calls is how the validator and the engine disagree *from the
        // validator side* — `rules.empty_target` is an Error and `stemma validate`
        // must report it.
        report.absorb(
            "rules",
            stem_soundchange::check_applied_log(
                &self.applied_rules,
                &self.phonemes,
                &self.prosody,
                &self.lexicon,
            ),
        );

        report
    }
}

impl LanguageGenome {
    /// Applies a rule set, producing the **next stage of this lineage**.
    ///
    /// A new genome with a new `id`, `parent: Some(self.id)`, and
    /// `lineage_depth_years` advanced. It is emphatically not an in-place edit: in
    /// a file-based project format there is no in-place, there are two files, and
    /// two genomes sharing a `LanguageId` is exactly what `docs/adr/0003` says
    /// must stay an Error. It is also not M4's fork, which produces *sisters* from
    /// one parent and copies cognate sets across a split; this produces one
    /// descendant and there is nothing to coordinate.
    ///
    /// `applied_rules` accumulates, so derivation indices stay meaningful across
    /// strata and `stemma trace` on the output is self-contained. The returned
    /// report carries the rule checks and the run report, bare-coded and already
    /// absorbed under `rules` / `soundchange`.
    pub fn evolve(
        &self,
        id: impl Into<LanguageId>,
        name: impl Into<String>,
        rules: &RuleSet,
        elapsed_years: i32,
    ) -> Result<(LanguageGenome, ValidationReport)> {
        let mut report = ValidationReport::new();
        report.absorb("rules", rules.validate());
        report.absorb(
            "rules",
            stem_soundchange::check_against_language(
                &rules.rules,
                &self.phonemes,
                &self.prosody,
                &self.lexicon,
            ),
        );
        if !report.is_ok() {
            return Err(StemmaError::Invalid(
                format!("the rule set `{}` cannot be applied", rules.id),
                report,
            ));
        }

        let evolution = stem_soundchange::apply_rules(
            &rules.rules,
            self.applied_rules.len() as u32,
            &self.phonemes,
            &self.prosody,
            &self.lexicon,
        )?;
        report.absorb("soundchange", evolution.report);

        let mut applied_rules = self.applied_rules.clone();
        applied_rules.extend(rules.rules.iter().cloned());

        let descendant = LanguageGenome {
            id: id.into(),
            name: name.into(),
            parent: Some(self.id.clone()),
            lineage_depth_years: self.lineage_depth_years + elapsed_years,
            seed: self.seed,
            phonemes: evolution.inventory,
            phonotactics: self.phonotactics.clone(),
            lexicon: evolution.lexicon,
            prosody: self.prosody,
            applied_rules,
            notes: self.notes.clone(),
        };

        Ok((descendant, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_phonology::{Phoneme, SegmentKind};

    fn featured(id: &str, ipa: &str, kind: SegmentKind, tokens: &[&str]) -> Phoneme {
        let bundle = stem_phonology::FeatureBundle::try_from(
            tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
        .expect("valid feature list");
        Phoneme::new(id, ipa, kind).with_features(bundle)
    }

    /// Fully featured since M3 — `features_unspecified` is an Error now, and this
    /// helper serves tests whose point is that a well-formed language is clean.
    fn asterian() -> LanguageGenome {
        LanguageGenome::proto("proto_asterian", "Proto-Asterian")
            .with_seed(42)
            .with_phonemes(PhonemeInventory::from_phonemes([
                featured(
                    "ph_t",
                    "t",
                    SegmentKind::Consonant,
                    &[
                        "-syllabic",
                        "+consonantal",
                        "-sonorant",
                        "-approximant",
                        "-continuant",
                        "-nasal",
                        "-lateral",
                        "-trill",
                        "-voice",
                        "-labial",
                        "+coronal",
                        "-dorsal",
                    ],
                ),
                featured(
                    "ph_k",
                    "k",
                    SegmentKind::Consonant,
                    &[
                        "-syllabic",
                        "+consonantal",
                        "-sonorant",
                        "-approximant",
                        "-continuant",
                        "-nasal",
                        "-lateral",
                        "-trill",
                        "-voice",
                        "-labial",
                        "-coronal",
                        "+dorsal",
                        "+high",
                        "-low",
                        "+back",
                        "-round",
                    ],
                ),
                featured(
                    "ph_a",
                    "a",
                    SegmentKind::Vowel,
                    &[
                        "+syllabic",
                        "-consonantal",
                        "+sonorant",
                        "+approximant",
                        "+continuant",
                        "-nasal",
                        "-lateral",
                        "-trill",
                        "+voice",
                        "-labial",
                        "-coronal",
                        "+dorsal",
                        "-high",
                        "+low",
                        "+back",
                        "-round",
                    ],
                ),
            ]))
    }

    #[test]
    fn a_well_formed_proto_language_validates() {
        let report = asterian().validate();
        assert!(report.is_ok(), "unexpected errors: {report}");
    }

    #[test]
    fn a_proto_language_has_no_parent() {
        assert!(asterian().is_proto());
    }

    #[test]
    fn phonology_issues_surface_namespaced_on_the_genome() {
        let broken = LanguageGenome::proto("l", "Broken");
        let report = broken.validate();
        assert!(
            report.errors().any(|i| i.code == "phonology.empty"),
            "inventory errors must reach the genome report: {report}"
        );
    }

    #[test]
    fn a_language_cannot_be_its_own_parent() {
        let mut genome = asterian();
        genome.parent = Some(genome.id.clone());
        let report = genome.validate();
        assert!(report.errors().any(|i| i.code == "self_parent"), "{report}");
    }

    #[test]
    fn a_daughter_with_no_elapsed_time_warns_but_stays_valid() {
        let mut genome = asterian();
        genome.parent = Some("proto_asterian_older".into());
        genome.lineage_depth_years = 0;
        let report = genome.validate();
        assert!(report.is_ok(), "{report}");
        assert!(
            report.warnings().any(|i| i.code == "no_elapsed_time"),
            "{report}"
        );
    }

    #[test]
    fn summary_reports_the_inventory_split() {
        let summary = asterian().summary();
        assert!(summary.contains("2C/1V"), "{summary}");
        assert!(summary.contains("seed 42"), "{summary}");
        assert!(summary.contains("proto"), "{summary}");
    }
}
