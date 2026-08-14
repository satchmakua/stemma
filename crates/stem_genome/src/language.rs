//! The genome itself.

use serde::{Deserialize, Serialize};
use stem_core::{LanguageId, Result, Severity, StemmaError, Validate, ValidationReport};
use stem_lexicon::{
    DriftEvent, DriftSet, EnvironmentProfile, HIGH_ALLOMORPH_COUNT, LONG_SENSE_CHAIN, Lexicon,
    Morphology, ProjectConcept, SemanticSpace, apply_drift, check_against_concepts,
    check_against_semantics, check_drift_against_language, morphological_irregularity,
    sense_chains,
};
use stem_phonology::{PhonemeInventory, Phonotactics, Prosody};
use stem_soundchange::{RuleSet, SoundChangeRule};
use stem_syntax::SyntaxProfile;

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
    /// §8.5's `HistoricalEvent` stays deferred past M4: of its five fields the
    /// only one §10.4's timeline reads is the date, and that ships as
    /// `chronology_years` on the rule. A union with one inhabited variant is
    /// still scaffolding, and M4's lineage graph derives descent from `parent`
    /// rather than from a stored event log (`docs/adr/0008`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_rules: Vec<SoundChangeRule>,

    /// This language's morphology — its morphemes and paradigms (ROADMAP M8,
    /// `DESIGN.md` §8.1).
    ///
    /// `#[serde(default)]` per this type's own contract, and the default is
    /// **empty** — every pre-M8 file, the reference fixtures included, loads and
    /// round-trips byte-identically because `skip_serializing_if` keeps an empty
    /// morphology out of the file. `evolve` and `fork` carry it verbatim, so a
    /// daughter knows its own paradigms and can re-inflect them; the sound changes
    /// that made a suffix irregular already live in each cell's `trace`, so nothing
    /// here needs to record them a second time.
    #[serde(default, skip_serializing_if = "Morphology::is_empty")]
    pub morphology: Morphology,

    /// This language's ecology and culture, as the vocabulary it implies (ROADMAP
    /// M15, `DESIGN.md` §7.5, §18.1).
    ///
    /// Why a language has the words it has: which meanings its speakers make several
    /// distinctions inside, and which they have no word for **and why**. Empty by
    /// default, so every pre-M15 file loads and round-trips byte-identically and
    /// coins exactly the vocabulary it did before.
    ///
    /// In the genome for M12's reason: `new-lexicon` reads it, so a language would
    /// not be reproducible from its own file alone if it lived in a sidecar. `fork`
    /// and `evolve` carry it verbatim — a daughter inherits its parents' ecology
    /// until an author says otherwise, which is the truthful default.
    #[serde(default, skip_serializing_if = "EnvironmentProfile::is_empty")]
    pub environment: EnvironmentProfile,

    /// This language's syntactic parameters (ROADMAP M17, `DESIGN.md` §7.4).
    ///
    /// How it builds a clause: word order, adpositions, alignment, and the rest.
    /// **Description, not an engine** — nothing in the workspace generates a sentence
    /// yet, and M18 is where that starts.
    ///
    /// Empty by default, so every pre-M17 file loads and round-trips byte-identically
    /// and — more to the point — a language nobody has given a grammar *says so*
    /// rather than silently claiming to be SVO. `fork` and `evolve` carry it verbatim:
    /// a daughter inherits its parent's syntax until an author says otherwise, which
    /// is the truthful default and the thing M19's syntactic change will act on.
    #[serde(default, skip_serializing_if = "SyntaxProfile::is_empty")]
    pub syntax: SyntaxProfile,

    /// Meanings **this project declares**, alongside the built-in concept list
    /// (ROADMAP M12).
    ///
    /// The built-in 103 are a *basic* vocabulary; a language whose speakers sail, or
    /// herd, or keep a priesthood needs words the Swadesh list has no opinion about.
    /// Declaring them here rather than in a sidecar file is what preserves [`Self::seed`]'s
    /// contract — the language stays reproducible **from its own file alone**, which
    /// is the whole reason `stem_lexicon::CONCEPTS` is compiled in rather than read
    /// from disk.
    ///
    /// **Appended after the built-in list, never interleaved.** The draw contract
    /// (`stem_lexicon::build`) makes `build(n)` a strict prefix of `build(n + k)`, so
    /// declaring a new meaning cannot change a word that was already coined.
    ///
    /// Default empty; `skip_serializing_if` keeps every pre-M12 file byte-identical.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concepts: Vec<ProjectConcept>,

    /// This language's senses (`DESIGN.md` §8.1's `semantics: SemanticSpace`,
    /// ROADMAP M9).
    ///
    /// Default **empty**, so every pre-M9 file — both reference fixtures included —
    /// loads and round-trips byte-identically. `fork` and `evolve` carry it
    /// verbatim, which is what lets a daughter render "star → omen" from its own
    /// file with no drift set on disk: the `seed` contract's "reproducible from the
    /// file alone", applied to meaning.
    #[serde(default, skip_serializing_if = "SemanticSpace::is_empty")]
    pub semantics: SemanticSpace,

    /// The semantic shifts that produced this language's current meanings (M9).
    ///
    /// **Past tense, and a log rather than a set** — `applied_rules`' contract
    /// verbatim, for the same reasons: applying is an explicit operation over a
    /// separate [`stem_lexicon::DriftSet`] file, ids may repeat across strata, and
    /// `SenseShift::index` indexes into this `Vec`, so it is never re-sorted.
    ///
    /// Kept beside `applied_rules` rather than merged into §8.5's `HistoricalEvent`
    /// union: migrating would renumber every `RuleApplication.index` in every stored
    /// derivation, a format break the project forbids. §10.4's unified timeline is a
    /// *derived* merge of the two ordered logs by `chronology_years` — computable,
    /// and therefore never stored (`docs/adr/0008`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_drifts: Vec<DriftEvent>,

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
            morphology: Morphology::default(),
            environment: EnvironmentProfile::default(),
            syntax: SyntaxProfile::default(),
            concepts: Vec::new(),
            semantics: SemanticSpace::new(),
            applied_drifts: Vec::new(),
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

    /// Sets the morphology (M8), replacing any existing one.
    #[must_use]
    pub fn with_morphology(mut self, morphology: Morphology) -> Self {
        self.morphology = morphology;
        self
    }

    /// Sets the semantic space (M9), replacing any existing one.
    #[must_use]
    pub fn with_semantics(mut self, semantics: SemanticSpace) -> Self {
        self.semantics = semantics;
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

        // §17's "extreme irregularity after only 100 years" names MORPHOLOGICAL
        // irregularity, now measured directly by `high_morphological_irregularity`
        // below (M8). This distinct, coarser signal reads the sound-change LOG —
        // rules per century — and is kept because it says something the allomorph
        // count does not: how fast the *history* moved. A Note, not a Warning: it
        // is a coarse proxy over an authoring artifact (`applied_rules.len()`
        // counts *authored* rules — one author's "voicing" is another's three), so
        // the register is "consider", and the threshold is a deliberately loose
        // tripwire, not a cited typological constant (the field has no clean rate).
        // Integer math (§9.4); guarded so it never divides by zero and never
        // collides with `no_elapsed_time`.
        const RAPID_CHANGE_RULES_PER_CENTURY: i64 = 3;
        if self.lineage_depth_years > 0 && !self.applied_rules.is_empty() {
            let changes = self.applied_rules.len() as i64;
            let years = i64::from(self.lineage_depth_years);
            if changes * 100 > years * RAPID_CHANGE_RULES_PER_CENTURY {
                report.note(
                    "high_change_density",
                    format!(
                        "this language records {changes} sound-change rules across {years} \
                         simulated years — a high rate for the elapsed time by this tool's \
                         coarse counting. Rapid-change scenarios are real (heavy contact, \
                         creolization); if that is not the intent, consider more time depth \
                         or fewer strata. (This counts authored rules, an editorial \
                         granularity, and reads only the sound-change log — morphological \
                         irregularity is measured separately, below.)"
                    ),
                );
            }
        }

        // M8's direct measure of §17's "extreme irregularity": an affix that
        // surfaces in `HIGH_ALLOMORPH_COUNT` or more distinct shapes. A Note, never
        // an Error — a wildly irregular paradigm is unusual, not broken, and the
        // tool reports rather than polices (§17). The FIRING is single-sourced on
        // the allomorph count via the shared `HIGH_ALLOMORPH_COUNT` constant, so it
        // holds exactly when the profile's band is `HighlyAllomorphic` — a
        // projection test pins the agreement (`docs/adr/0009`). The M8 demo's
        // two-way `-ɡa`/`-ka` alternation is `Allomorphic`, below this bar, and
        // correctly does not trip it: a single conditioned split is ordinary.
        for allomorphs in morphological_irregularity(&self.lexicon) {
            if allomorphs.count() >= HIGH_ALLOMORPH_COUNT {
                report.note(
                    "high_morphological_irregularity",
                    format!(
                        "the affix `{}` (\"{}\") surfaces in {} distinct shapes across this \
                         lexicon — extreme allomorphy by this tool's counting. Deep, ordered \
                         sound change genuinely produces this (think strong verbs); if it was \
                         not intended, a conditioning rule may be firing more broadly than \
                         meant.",
                        allomorphs.morpheme,
                        allomorphs.gloss,
                        allomorphs.count()
                    ),
                );
            }
        }

        // M9's direct measure of how far a meaning has travelled: a word carrying
        // `LONG_SENSE_CHAIN` or more recorded shifts. A Note, never an Error — a
        // long chain is unusual, not broken, and the tool reports rather than
        // polices (§17). Single-sourced on the shared constant, so it fires exactly
        // when the profile band reads `HighlyDrifted` (`docs/adr/0009`); a
        // projection test pins the agreement. The M9 demo's own two-step Coastal
        // chain sits below this bar and correctly stays quiet.
        for chain in sense_chains(&self.lexicon) {
            if chain.shifts >= LONG_SENSE_CHAIN {
                report.note(
                    "long_semantic_drift_chain",
                    format!(
                        "the word `{}` (\"{}\") records {} meaning shifts — a long chain by this \
                         tool's counting. Long chains are real (hand > control > authority > \
                         power is attested); if it was not intended, an intermediate sense may \
                         be doing no work. (This counts authored drift events, an editorial \
                         granularity.)",
                        chain.word, chain.gloss, chain.shifts
                    ),
                );
            }
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

        // M9, the same shape and for the same reason: it needs the lexicon *and*
        // the semantic space, and `Validate::validate` takes no context. Every
        // check inside is gated on non-empty senses / history / space, so a pre-M9
        // language absorbs an empty report and gains no scope line.
        report.absorb(
            "semantics",
            check_against_semantics(&self.lexicon, &self.semantics, &self.applied_drifts),
        );

        // M12: `unknown_concept` moved out of `Lexicon::validate` because a bare
        // lexicon cannot tell an invented key from one this project declares. Same
        // shape, same reason, as the two checks above.
        report.absorb(
            "concepts",
            check_against_concepts(&self.lexicon, &self.concepts),
        );

        // M17, the sixth instance of the same pattern — except this one needs no
        // context at all, so it is an ordinary `Validate` impl absorbed under its own
        // scope. Gated on a non-empty profile, so a pre-M17 language absorbs an empty
        // report and gains no scope line.
        if !self.syntax.is_empty() {
            report.absorb("syntax", self.syntax.validate());
        }

        // M15, the fifth instance: a culture trait names concepts that may be
        // built-in or project-declared, so the check needs both lists. Gated on a
        // non-empty profile, so a pre-M15 language absorbs an empty report.
        if !self.environment.is_empty() {
            report.absorb(
                "culture",
                stem_lexicon::check_against_environment(
                    &self.environment,
                    &self.concepts,
                    &stem_lexicon::meanings(&self.concepts),
                ),
            );
        }

        // M14, and the fourth instance of the same pattern: a derivation pattern's
        // affix lives in the morphology while the words it coined live in the
        // lexicon, so no single type can check that a `BaseRef` still points at the
        // word it claims. Every check inside is gated on a non-empty patterns list
        // or a non-empty `bases`, so a pre-M14 language absorbs an empty report and
        // gains no scope line.
        report.absorb(
            "derivation",
            stem_lexicon::check_against_derivations(
                &self.lexicon,
                &self.morphology.derivations,
                &self.morphology.morphemes,
            ),
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
    /// M4's split (`DESIGN.md` §3.5, §8.6): this genome copied verbatim under a
    /// new identity, with a parent edge back. **No rules run, no RNG, no form
    /// changes** — forking is a statement about lineage, not about change. Rule
    /// application stays [`Self::evolve`]'s job; the CLI's `fork` verb calls
    /// `evolve` when given `--rules`, so at file level a bare fork and a
    /// rule-bearing fork are the same two-file, two-id shape (`docs/adr/0008`).
    ///
    /// The cognate obligation (`docs/adr/0007`) is discharged **by
    /// construction**: the lexicon is cloned whole, so every `cognate_set` is
    /// byte-identical and no code path here can mint. Word ids are copied
    /// verbatim too, which is why a daughter word's ancestor is always the
    /// same-id entry of the parent and needs no stored field (§2.4 of the spec,
    /// `docs/adr/0008`).
    ///
    /// Traces and `applied_rules` are carried verbatim: those changes *did*
    /// happen to these forms, and `Derivation::input` stays the ultimate
    /// proto-form, so a later `evolve` on the daughter extends the same
    /// derivation and `stemma trace` still walks unbroken to the proto.
    ///
    /// Infallible — a clone-and-relabel has nothing to report that
    /// [`Self::validate`] does not already say. The caller validates; a fork
    /// given the parent's own id yields a genome the `self_parent` Error catches.
    #[must_use]
    pub fn fork(
        &self,
        id: impl Into<LanguageId>,
        name: impl Into<String>,
        elapsed_years: i32,
    ) -> LanguageGenome {
        LanguageGenome {
            id: id.into(),
            name: name.into(),
            parent: Some(self.id.clone()),
            // Same arithmetic as `evolve`; a bare split is conventionally +0y,
            // and the daughter's total depth is the parent's plus the split gap.
            lineage_depth_years: self.lineage_depth_years + elapsed_years,
            // Copied verbatim, per `evolve`'s precedent: the seed reproduces the
            // inherited lexicon, so the daughter file stays reproducible from
            // itself alone. Two sisters therefore share a seed — a documented
            // consequence (`docs/adr/0008`), harmless until a daughter-side
            // stochastic step exists, since no RNG runs at a fork.
            seed: self.seed,
            phonemes: self.phonemes.clone(),
            phonotactics: self.phonotactics.clone(),
            lexicon: self.lexicon.clone(),
            prosody: self.prosody,
            applied_rules: self.applied_rules.clone(),
            // Carried verbatim: a fork is a statement about lineage, and the
            // daughter keeps the same morphemes and paradigms it inherited.
            morphology: self.morphology.clone(),
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            // A daughter inherits the meanings its parent could express.
            concepts: self.concepts.clone(),
            // Likewise its senses and their recorded history: a daughter means what
            // its parent meant until a drift event says otherwise (M9).
            semantics: self.semantics.clone(),
            applied_drifts: self.applied_drifts.clone(),
            notes: self.notes.clone(),
        }
    }

    /// Applies a rule set, producing the **next stage of this lineage**.
    ///
    /// A new genome with a new `id`, `parent: Some(self.id)`, and
    /// `lineage_depth_years` advanced. It is emphatically not an in-place edit: in
    /// a file-based project format there is no in-place, there are two files, and
    /// two genomes sharing a `LanguageId` is exactly what `docs/adr/0003` says
    /// must stay an Error. It differs from [`Self::fork`] only in that fork runs
    /// no rules: the *operations* are distinct (advance a stage vs relabel a
    /// copy), while the CLI *verb* `fork` covers both because the file records no
    /// verb (`docs/adr/0008`).
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
            // Carried verbatim: the sound changes that made a paradigm irregular
            // are recorded in each evolved cell's `trace`, not here — the morphemes
            // and paradigms themselves are unchanged by a rule run.
            morphology: self.morphology.clone(),
            environment: self.environment.clone(),
            syntax: self.syntax.clone(),
            concepts: self.concepts.clone(),
            // A sound change moves forms, never meanings. The senses and their
            // history ride along untouched — and each word's `senses` /
            // `sense_history` survive because `apply_rules` clones the entry whole
            // (M9; the free ride M8's `morphemes` got).
            semantics: self.semantics.clone(),
            applied_drifts: self.applied_drifts.clone(),
            notes: self.notes.clone(),
        };

        Ok((descendant, report))
    }

    /// Applies a drift set **within this stage**: same id, name, parent, depth,
    /// seed, phonology, forms, `applied_rules` and morphology. Only `lexicon`,
    /// `semantics` and `applied_drifts` change (M9).
    ///
    /// This is the in-place-shaped operation — what `grow_family` uses, so a
    /// drifted branch stays **one node and one column** rather than sprouting a
    /// phantom stage in the family tree. A caller that wants drift to *mint a
    /// stage* wants [`Self::drift`].
    ///
    /// `apply_drift` never touches `cognate_set`, so the §8.6 obligation is
    /// discharged by construction — the drifted reflex keeps its row in the
    /// comparative table, which is exactly the M9 acceptance.
    pub fn with_drift(&self, set: &DriftSet) -> Result<(LanguageGenome, ValidationReport)> {
        let mut report = ValidationReport::new();
        report.absorb("drift", set.validate());
        report.absorb(
            "drift",
            check_drift_against_language(
                set,
                &self.semantics,
                &self.lexicon,
                self.lineage_depth_years,
            ),
        );
        if !report.is_ok() {
            return Err(StemmaError::Invalid(
                format!("the drift set `{}` cannot be applied", set.id),
                report,
            ));
        }

        let outcome = apply_drift(
            set,
            self.applied_drifts.len() as u32,
            &self.semantics,
            &self.lexicon,
        )?;
        report.absorb("drift", outcome.report);

        let mut applied_drifts = self.applied_drifts.clone();
        applied_drifts.extend(set.events.iter().cloned());

        Ok((
            LanguageGenome {
                lexicon: outcome.lexicon,
                semantics: outcome.space,
                applied_drifts,
                ..self.clone()
            },
            report,
        ))
    }

    /// Drift as the **next stage of this lineage** — literally
    /// `self.fork(id, name, elapsed_years)` followed by [`Self::with_drift`].
    ///
    /// No new copy logic: `fork` already clones verbatim and discharges the cognate
    /// obligation by construction. This is what the CLI's `drift` verb uses, because
    /// a `--out` file sharing its input's [`LanguageId`] would be the
    /// `duplicate_language_id` Error (`docs/adr/0003`).
    pub fn drift(
        &self,
        id: impl Into<LanguageId>,
        name: impl Into<String>,
        set: &DriftSet,
        elapsed_years: i32,
    ) -> Result<(LanguageGenome, ValidationReport)> {
        self.fork(id, name, elapsed_years).with_drift(set)
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

    // --- M7: high_change_density (a Note, never an Error) ---

    #[test]
    fn a_daughter_with_many_rules_in_few_years_notes_high_change_density() {
        let rule = voicing().rules[0].clone();
        let mut genome = asterian();
        genome.parent = Some("proto".into());
        genome.lineage_depth_years = 100;
        genome.applied_rules = std::iter::repeat_n(rule, 10).collect(); // 10 rules / 100y
        let report = genome.validate();
        assert!(
            report.is_ok(),
            "a rapid history is a Note, not a rejection (§17): {report}"
        );
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "high_change_density"),
            "10 rules in 100 years is a high rate: {report}"
        );
    }

    #[test]
    fn a_proto_with_no_rule_history_is_never_flagged_for_change_density() {
        // asterian() is a proto: no parent, no applied_rules, depth 0.
        let report = asterian().validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "high_change_density"),
            "the guard holds for a proto: {report}"
        );
    }

    #[test]
    fn a_realistic_change_rate_stays_quiet() {
        let rule = voicing().rules[0].clone();
        let mut genome = asterian();
        genome.parent = Some("proto".into());
        genome.lineage_depth_years = 470;
        genome.applied_rules = std::iter::repeat_n(rule, 4).collect(); // the Coastal shape
        let report = genome.validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "high_change_density"),
            "4 rules over 470 years is an ordinary rate: {report}"
        );
    }

    #[test]
    fn summary_reports_the_inventory_split() {
        let summary = asterian().summary();
        assert!(summary.contains("2C/1V"), "{summary}");
        assert!(summary.contains("seed 42"), "{summary}");
        assert!(summary.contains("proto"), "{summary}");
    }

    // ----- M4: fork -----

    use stem_core::{CognateSetId, PhonemeId, WordId};
    use stem_lexicon::{Lexicon, PartOfSpeech, WordEntry, WordSource};
    use stem_phonology::{Root, Syllable};
    use stem_soundchange::{Change, EnvItem, Environment, SegmentPattern};

    /// A one-word lexicon (`taka`) so fork's copy-verbatim obligation has
    /// something to carry, and so the intervocalic /k/ gives [`voicing`] a site.
    fn with_one_word(mut genome: LanguageGenome) -> LanguageGenome {
        let syl = |c: &str, v: &str| Syllable {
            pattern: "CV".to_owned(),
            segments: vec![PhonemeId::new(c), PhonemeId::new(v)],
            stress: None,
        };
        let entry = WordEntry {
            id: WordId::new("w_0001"),
            concept: Some(stem_lexicon::ConceptKey::new("MOON")),
            phonemic_form: Root {
                syllables: vec![syl("ph_t", "ph_a"), syl("ph_k", "ph_a")],
            },
            glosses: vec![],
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new("cog_proto_asterian_0001"),
            source: WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: Vec::new(),
            sense_history: None,
        };
        genome.lexicon = Lexicon::from_entries([entry]);
        genome
    }

    /// Intervocalic voicing as a one-rule set: voices the /k/ of `taka` to a
    /// minted /ɡ/. Enough to give a fork a real trace and rule history to carry.
    fn voicing() -> RuleSet {
        let bundle = |tokens: &[&str]| {
            stem_phonology::FeatureBundle::try_from(
                tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            )
            .expect("valid feature list")
        };
        RuleSet {
            id: "rules_voicing".to_owned(),
            name: "Voicing".to_owned(),
            description: String::new(),
            rules: vec![SoundChangeRule {
                id: stem_core::RuleId::new("r_ivv"),
                name: "Intervocalic voicing".to_owned(),
                description: String::new(),
                chronology_years: 100,
                target: SegmentPattern {
                    features: bundle(&["-sonorant", "-continuant", "-voice"]),
                    stress: None,
                },
                environment: Environment {
                    before: vec![EnvItem::Segment(SegmentPattern {
                        features: bundle(&["+syllabic"]),
                        stress: None,
                    })],
                    after: vec![EnvItem::Segment(SegmentPattern {
                        features: bundle(&["+syllabic"]),
                        stress: None,
                    })],
                },
                change: Change::Set(bundle(&["+voice"])),
            }],
        }
    }

    #[test]
    fn a_fork_copies_every_cognate_set_verbatim() {
        let parent = with_one_word(asterian());
        let daughter = parent.fork("coastal", "Coastal", 100);
        let parent_sets: Vec<_> = parent.lexicon.iter().map(|e| &e.cognate_set).collect();
        let daughter_sets: Vec<_> = daughter.lexicon.iter().map(|e| &e.cognate_set).collect();
        assert_eq!(
            parent_sets, daughter_sets,
            "fork must copy cognate sets byte-for-byte, never mint"
        );
    }

    #[test]
    fn a_fork_copies_word_ids_verbatim_so_ancestry_is_derivable() {
        let parent = with_one_word(asterian());
        let daughter = parent.fork("coastal", "Coastal", 100);
        for child_word in daughter.lexicon.iter() {
            assert!(
                parent.lexicon.get(&child_word.id).is_some(),
                "every daughter word id `{}` must resolve in the parent, so `ancestor` \
                 need not be stored",
                child_word.id
            );
        }
    }

    #[test]
    fn a_fork_changes_identity_parent_and_years_and_nothing_else() {
        let parent = with_one_word(asterian());
        let daughter = parent.fork("coastal", "Coastal Asterian", 250);

        assert_eq!(daughter.id, LanguageId::new("coastal"));
        assert_eq!(daughter.name, "Coastal Asterian");
        assert_eq!(daughter.parent, Some(parent.id.clone()));
        assert_eq!(
            daughter.lineage_depth_years,
            parent.lineage_depth_years + 250
        );

        // Everything else is byte-identical to the parent.
        assert_eq!(daughter.seed, parent.seed, "seed copied verbatim");
        assert_eq!(daughter.phonemes, parent.phonemes);
        assert_eq!(daughter.phonotactics, parent.phonotactics);
        assert_eq!(daughter.prosody, parent.prosody);
        assert_eq!(daughter.lexicon, parent.lexicon);
        assert_eq!(daughter.applied_rules, parent.applied_rules);
        assert_eq!(daughter.notes, parent.notes);
    }

    #[test]
    fn a_zero_year_fork_of_a_proto_warns_no_elapsed_time_but_stays_valid() {
        // The genome-level warning fires only when TOTAL depth is 0, i.e. the
        // parent is itself a depth-0 proto (§2.2). A daughter of a deep parent
        // would not trip it; `family.no_divergence` is the complement there.
        let parent = with_one_word(asterian()); // depth 0
        let daughter = parent.fork("coastal", "Coastal", 0);
        let report = daughter.validate();
        assert!(report.is_ok(), "a fresh fork is valid: {report}");
        assert!(
            report.warnings().any(|i| i.code == "no_elapsed_time"),
            "a zero-year fork of a depth-0 proto has had no time to diverge: {report}"
        );
    }

    #[test]
    fn a_fork_given_its_parents_id_fails_validation_as_self_parent() {
        let parent = with_one_word(asterian());
        let daughter = parent.fork(parent.id.clone(), "Impostor", 100);
        let report = daughter.validate();
        assert!(
            report.errors().any(|i| i.code == "self_parent"),
            "a fork onto the parent's own id is the self_parent Error: {report}"
        );
    }

    #[test]
    fn forking_an_evolved_language_keeps_its_traces_and_rule_history() {
        let proto = with_one_word(asterian());
        let (evolved, _) = proto
            .evolve("stage1", "Stage One", &voicing(), 100)
            .unwrap();
        // Sanity: evolution produced a trace and a rule log.
        let evolved_word = evolved.lexicon.iter().next().unwrap();
        assert!(evolved_word.trace.is_some(), "evolve records a trace");
        assert_eq!(evolved.applied_rules.len(), 1);

        let sister = evolved.fork("sister", "Sister", 50);
        let sister_word = sister.lexicon.iter().next().unwrap();
        assert_eq!(
            sister_word.trace, evolved_word.trace,
            "fork carries the derivation verbatim; the history happened to these forms"
        );
        assert_eq!(
            sister.applied_rules, evolved.applied_rules,
            "fork carries the rule history verbatim, or RuleApplication::index is orphaned"
        );
    }

    #[test]
    fn evolving_a_fork_extends_its_derivation_from_the_proto_form() {
        let proto = with_one_word(asterian());
        let proto_form = proto.lexicon.iter().next().unwrap().phonemic_form.clone();

        let (stage1, _) = proto
            .evolve("stage1", "Stage One", &voicing(), 100)
            .unwrap();
        let forked = stage1.fork("branch", "Branch", 0);
        // A second stratum on the fork: voicing again is a no-op (already voiced),
        // but evolve still extends the derivation and its indices continue.
        let (stage2, _) = forked
            .evolve("stage2", "Stage Two", &voicing(), 100)
            .unwrap();

        let word = stage2.lexicon.iter().next().unwrap();
        let derivation = word.trace.as_ref().expect("carries a derivation");
        assert_eq!(
            derivation.input, proto_form,
            "Derivation::input stays the ultimate proto-form across fork and re-evolution"
        );
        assert_eq!(
            stage2.applied_rules.len(),
            2,
            "the second stratum extends applied_rules; it does not replace it"
        );
    }
}
