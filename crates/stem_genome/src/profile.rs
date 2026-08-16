//! The typological plausibility profile (`DESIGN.md` §17, M7).
//!
//! §17's scored-dimensions block as a **derived, read-only** value —
//! [`LanguageGenome::plausibility_profile`] — plus a pure text renderer,
//! [`render_profile`]. It is the `render_family` / `CognateTable` precedent:
//! built in memory, never persisted, no map, no float, no clock, so two calls are
//! byte-identical (§9.4).
//!
//! **It is NOT a validation subsystem** (`docs/adr/0009`). The plausibility
//! *warnings* live in the `Validate` impls and reach `stemma validate` as ordinary
//! report checks; this is the descriptive *summary* alongside them, computed from
//! the same fields over the same shared threshold constants (so a band can never
//! disagree with the check that fired). It produces no `Issue` and carries no
//! `Severity`.
//!
//! **It scores only what the engine can see** — phonology (M1) and coarse lineage
//! (M3–M4). The four §17 dimensions that need unbuilt milestones
//! ([`NOT_MODELLED`]) are named as such, never given a fabricated number, and
//! §17's composite percentage is deliberately dropped (any number would overclaim
//! or average over dimensions that do not exist).

use stem_lexicon::{
    HIGH_ALLOMORPH_COUNT, LARGE_VOCABULARY_GAP, LONG_SENSE_CHAIN, morphological_irregularity,
    sense_chains,
};
use stem_phonology::{Complexity, Rarity};

use crate::LanguageGenome;

/// A coarse bucket of the recorded rule count and elapsed years — **not** a
/// typological measurement (there is no attested "historical depth" scale), and
/// the count is of *authored rules*, partly an editorial granularity. `None` for
/// a language with no recorded sound changes (a proto is the root — a fact, not a
/// low score). The renderer shows the raw basis beside the band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalDepth {
    None,
    Shallow,
    Moderate,
    Deep,
}

/// How irregular this language's affixal morphology is, measured directly (M8) as
/// the largest number of distinct surface shapes any one affix takes across the
/// lexicon — the row §17 called "morphological irregularity", filled by the real
/// [`morphological_irregularity`] measure rather than deferred.
///
/// A band, not a number, like every other profile dimension. `None` is honest for
/// a language with no affixation — a proto with a monomorphemic lexicon has nothing
/// to be irregular *about*, which is a fact, not a low score (the `HistoricalDepth`
/// precedent). The threshold between `Allomorphic` and `HighlyAllomorphic` is the
/// shared [`HIGH_ALLOMORPH_COUNT`], so this band is `HighlyAllomorphic` exactly when
/// the `high_morphological_irregularity` validation Note fires (`docs/adr/0009`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorphologicalIrregularity {
    /// No affix appears in the lexicon — nothing to measure.
    None,
    /// Every affix surfaces one way — a fully regular paradigm.
    Regular,
    /// Some affix surfaces in 2..[`HIGH_ALLOMORPH_COUNT`] shapes — an ordinary
    /// conditioned split (the M8 demo's `-ɡa`/`-ka`).
    Allomorphic,
    /// Some affix surfaces in [`HIGH_ALLOMORPH_COUNT`] or more shapes — extreme
    /// allomorphy, the band the validation Note is paired to.
    HighlyAllomorphic,
}

/// How far this language's meanings have travelled (M9), measured as the longest
/// **recorded** sense chain any one word has undergone.
///
/// # Deliberately not called "semantic plausibility"
///
/// It fills §17's semantic row, but judging whether `star → omen` is a *plausible
/// pathway* needs a typology of attested shifts this project does not have and
/// §20.1 fences out. Emitting a number for that would be exactly the fabrication
/// `docs/adr/0009` forbids. This measures what the engine can actually see —
/// distance travelled — and the renderer says so. The count is of **authored drift
/// events**, an editorial granularity: one author's single metaphor is another's
/// two steps.
///
/// The threshold between [`Self::Drifted`] and [`Self::HighlyDrifted`] is the shared
/// [`LONG_SENSE_CHAIN`], so this band reads `HighlyDrifted` exactly when the
/// `long_semantic_drift_chain` validation Note fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticDrift {
    /// No sense declared and no history recorded — nothing to measure. A fact, not
    /// a low score (the [`HistoricalDepth::None`] precedent).
    None,
    /// Senses are modelled and no shift is recorded.
    Stable,
    /// Some word records 1..[`LONG_SENSE_CHAIN`] shifts — an ordinary chain.
    Drifted,
    /// Some word records [`LONG_SENSE_CHAIN`] or more — the band the Note is paired
    /// to.
    HighlyDrifted,
}

/// How deliberately this language's vocabulary has been shaped by its culture (M15).
///
/// Descriptive, not a score, and emphatically not a plausibility ranking: a heavily
/// shaped vocabulary is a **more** considered language, not a less believable one.
/// It sits in this block because without it the profile is blind to the largest
/// single fact about a language's word list — a 673-meaning language and a
/// 600-meaning one read identically everywhere else.
///
/// The threshold between [`Self::Shaped`] and [`Self::HeavilyShaped`] is the shared
/// [`LARGE_VOCABULARY_GAP`], so this band reads `HeavilyShaped` exactly when the
/// `large_vocabulary_gap` validation Note fires (`docs/adr/0009`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyShaping {
    /// No culture profile — every available meaning is coined. A fact, not a low
    /// score (the [`HistoricalDepth::None`] precedent), though it is the *undeliberate*
    /// state M15 exists to give authors a way out of.
    Unshaped,
    /// A profile is declared and removes fewer than [`LARGE_VOCABULARY_GAP`]
    /// meanings.
    Shaped,
    /// A profile removes [`LARGE_VOCABULARY_GAP`] meanings or more — the band the
    /// Note is paired to.
    HeavilyShaped,
}

/// How far this language's **spelling** has fallen behind its pronunciation (M21) —
/// §17's script-history row, and the last dimension M7 deferred.
///
/// # It measures drift, not quality
///
/// A deep orthography is not a worse one. English and French are deep, Finnish and
/// Turkish are shallow, and none of them is thereby a better language — depth is the
/// ordinary consequence of a script staying still while speech moves, which is what
/// scripts do. The band describes; it does not rank, and no value of it is an Error.
///
/// Both inputs are **findings**: sounds the script cannot write and signs whose sound
/// is gone, counted by [`script_drift`](stem_script::script_drift) from the lexicon.
/// The author writes a glyph's biography; the engine measures the fit.
///
/// The threshold between [`Self::Historical`] and [`Self::Deep`] is the shared
/// [`DEEP_ORTHOGRAPHY`], so this band reads `Deep` exactly when the `deep_orthography`
/// validation Note fires (`docs/adr/0009`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptHistory {
    /// This language has no script. A fact, not a low score — most languages that have
    /// ever existed were unwritten (the [`HistoricalDepth::None`] precedent).
    Unwritten,
    /// A script, but no lexicon to hold it against. Nothing can be measured yet, and
    /// saying so is better than reporting a clean fit nothing checked.
    Unmeasured,
    /// Every script this language has writes **meanings**, so there is no pronunciation
    /// in the spelling for the pronunciation to drift away from.
    ///
    /// Not a high score and not a low one — an evasion of the question, and the reason
    /// logographies are so durable. Reporting it as [`Self::Phonemic`] would claim
    /// every sound has a sign in a script that has no sound signs at all.
    SoundIndependent,
    /// Every sound the language says has a sign, and every sign still has a sound.
    Phonemic,
    /// Some mismatch exists: the spelling has begun to be historical.
    Historical,
    /// [`DEEP_ORTHOGRAPHY`] mismatches or more — the band the Note is paired to.
    Deep,
}

/// A §17 dimension no shipped milestone can measure yet. Carried explicitly so the
/// profile is transparent about its own coverage rather than silently omitting
/// four of §17's rows — or, worse, fabricating a number for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotModelled {
    AlienEmbodimentDependence,
}

impl NotModelled {
    /// The §17 dimension name.
    pub fn label(self) -> &'static str {
        match self {
            Self::AlienEmbodimentDependence => "Alien embodiment dependence",
        }
    }

    /// The design section that would make it measurable.
    ///
    /// A **section** reference, not a milestone number: both remaining dimensions
    /// sit in ROADMAP's "Beyond" list rather than at a numbered milestone, and
    /// naming a milestone that does not exist would be a schedule claim the project
    /// has not made.
    pub fn milestone(self) -> &'static str {
        match self {
            Self::AlienEmbodimentDependence => "§18",
        }
    }
}

/// The §17 dimensions still deferred, in §17's listed order — one list the renderer
/// and future milestones share, so a renumber is a one-line change.
///
/// Morphological irregularity left this list at M8, **semantic plausibility at M9**,
/// and **script-history coherence at M21** ([`ScriptHistory`]); all three are scored
/// bands now, and one row is left. Syntax / word order is deliberately absent: §20.1
/// forbids a syntax engine, and a "not modelled" line would invite the build.
pub const NOT_MODELLED: &[NotModelled] = &[NotModelled::AlienEmbodimentDependence];

/// §17's scored-dimensions block as a value. Never persisted (the `CognateTable`
/// precedent); grows fields additively as milestones fill dimensions. No serde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlausibilityProfile {
    /// Typological rarity of the phoneme inventory.
    pub rarity: Rarity,
    /// Phonotactic complexity of the declared root templates.
    pub complexity: Complexity,
    /// A coarse bucket of the recorded history.
    pub historical_depth: HistoricalDepth,
    /// The raw basis for `historical_depth`, shown beside the band for honesty.
    pub recorded_changes: usize,
    /// Simulated years since the lineage root.
    pub elapsed_years: i32,
    /// How irregular the affixal morphology is (M8).
    pub morphological_irregularity: MorphologicalIrregularity,
    /// The raw basis for `morphological_irregularity`: each affix's distinct-shape
    /// count, by gloss, in first-appearance order — shown beside the band for
    /// honesty, exactly as `recorded_changes` is shown beside `historical_depth`.
    /// An ordered `Vec`, never a map (§9.4).
    pub affix_allomorphy: Vec<(String, usize)>,
    /// How far the meanings have travelled (M9).
    pub semantic_drift: SemanticDrift,
    /// The raw basis for `semantic_drift`: each drifted word's (displayed gloss,
    /// recorded shift count), in lexicon order — the `affix_allomorphy` shape, and
    /// shown beside the band for the same honesty reason.
    pub sense_chains: Vec<(String, usize)>,
    /// How deliberately the culture profile shapes the vocabulary (M15).
    pub vocabulary_shaping: VocabularyShaping,
    /// The raw basis for `vocabulary_shaping`: `(uncoined, elaborated, extra words)`,
    /// shown beside the band so the number can be checked rather than trusted — the
    /// `recorded_changes` discipline.
    pub shaping_counts: (usize, usize, usize),
    /// How far the spelling has fallen behind the pronunciation (M21).
    pub script_history: ScriptHistory,
    /// The raw basis for `script_history`: each script's `(id, mismatch count)`, in
    /// declared order — the `affix_allomorphy` shape, for the same honesty reason. A
    /// language with several scripts drifts from each at its own rate, and the band is
    /// the worst of them.
    pub script_drift: Vec<(String, usize)>,
}

impl LanguageGenome {
    /// The §17 plausibility profile — a derived, read-only description in bands.
    ///
    /// A pure function of the genome: no RNG, no clock, no map, no float. NOT a
    /// validation subsystem — the plausibility *warnings* are checks in the
    /// `Validate` impls; this is §17's descriptive block, computed from the same
    /// fields (`docs/adr/0009`).
    pub fn plausibility_profile(&self) -> PlausibilityProfile {
        let recorded_changes = self.applied_rules.len();
        let years = self.lineage_depth_years;

        // Coarse; both inputs matter (a verbatim fork at 2000 years with no rules
        // is `None`, not `Deep`). Documented as a presentational bucket, not a
        // typological scale.
        let historical_depth = if recorded_changes == 0 {
            HistoricalDepth::None
        } else if recorded_changes >= 4 && years >= 400 {
            HistoricalDepth::Deep
        } else if recorded_changes >= 2 && years >= 150 {
            HistoricalDepth::Moderate
        } else {
            HistoricalDepth::Shallow
        };

        // M8: the direct allomorph measure, bucketed. `None` when no affix appears
        // (a monomorphemic proto has nothing to be irregular about — the
        // `HistoricalDepth::None` precedent); otherwise the worst affix's count
        // decides the band, against the same shared threshold the validation Note
        // reads, so band and Note cannot disagree (`docs/adr/0009`).
        let affix_allomorphy: Vec<(String, usize)> = morphological_irregularity(&self.lexicon)
            .into_iter()
            .map(|set| {
                let count = set.count();
                (set.gloss, count)
            })
            .collect();
        let morphological_irregularity = match affix_allomorphy.iter().map(|(_, n)| *n).max() {
            None => MorphologicalIrregularity::None,
            Some(max) if max >= HIGH_ALLOMORPH_COUNT => {
                MorphologicalIrregularity::HighlyAllomorphic
            }
            Some(max) if max > 1 => MorphologicalIrregularity::Allomorphic,
            Some(_) => MorphologicalIrregularity::Regular,
        };

        // M9: the longest RECORDED chain, bucketed. `None` when nothing semantic
        // is modelled at all (a fact, not a zero); otherwise the worst chain picks
        // the band, against the same shared threshold the validation Note reads, so
        // band and Note cannot disagree (`docs/adr/0009`).
        let chains: Vec<(String, usize)> = sense_chains(&self.lexicon)
            .into_iter()
            .map(|c| (c.gloss, c.shifts))
            .collect();
        let semantic_drift = if self.semantics.is_empty() && chains.is_empty() {
            // Nothing declared and nothing recorded: there is no meaning history to
            // measure. A fact, not a low score.
            SemanticDrift::None
        } else {
            match chains.iter().map(|(_, n)| *n).max() {
                // Senses exist; no word has a recorded history.
                None => SemanticDrift::Stable,
                Some(max) if max >= LONG_SENSE_CHAIN => SemanticDrift::HighlyDrifted,
                Some(max) if max > 0 => SemanticDrift::Drifted,
                // Histories exist but record no shift — still stable.
                Some(_) => SemanticDrift::Stable,
            }
        };

        // M15: how deliberately the culture profile shapes the word list. The band
        // reads the same `LARGE_VOCABULARY_GAP` the `large_vocabulary_gap` Note
        // reads, so the two cannot disagree about when a shaping is remarkable
        // (`docs/adr/0009`). `Unshaped` is "no profile declared", which is a
        // different fact from "a profile that removes nothing" — the latter is a
        // deliberate decision to keep everything, and reads as `Shaped`.
        let counts = stem_lexicon::shaping_counts(
            &self.environment,
            &stem_lexicon::meanings(&self.concepts),
        );
        let vocabulary_shaping = if self.environment.is_empty() {
            VocabularyShaping::Unshaped
        } else if counts.0 >= LARGE_VOCABULARY_GAP {
            VocabularyShaping::HeavilyShaped
        } else {
            VocabularyShaping::Shaped
        };

        // M21: how far the spelling has fallen behind the pronunciation. Every script
        // is measured, because a language may carry several and they drift at their own
        // rates; the band is the worst of them, and the per-script counts ride along so
        // the number can be checked. Same shared `DEEP_ORTHOGRAPHY` the
        // `deep_orthography` Note reads, so band and Note cannot disagree
        // (`docs/adr/0009`) — the fourth instance of that rule.
        let script_drift: Vec<(String, usize)> = self
            .scripts
            .iter()
            .map(|s| {
                let drift = stem_script::script_drift(s, &self.lexicon, &self.phonemes);
                (s.id.clone(), drift.distance())
            })
            .collect();
        let script_history = if self.scripts.is_empty() {
            ScriptHistory::Unwritten
        } else if self.lexicon.is_empty() {
            // A script with nothing to write. Reporting `Phonemic` here would be a
            // clean bill of health nothing checked.
            ScriptHistory::Unmeasured
        } else if !self.scripts.iter().any(|s| s.writes_sound()) {
            // Every script writes meanings. There is no spelling here to fall behind a
            // pronunciation, and calling that `Phonemic` would be the opposite of true.
            ScriptHistory::SoundIndependent
        } else {
            match script_drift.iter().map(|(_, n)| *n).max() {
                Some(worst) if worst >= stem_script::DEEP_ORTHOGRAPHY => ScriptHistory::Deep,
                Some(worst) if worst > 0 => ScriptHistory::Historical,
                _ => ScriptHistory::Phonemic,
            }
        };

        PlausibilityProfile {
            rarity: self.phonemes.rarity(),
            complexity: self.phonotactics.complexity(),
            historical_depth,
            recorded_changes,
            elapsed_years: years,
            morphological_irregularity,
            affix_allomorphy,
            semantic_drift,
            sense_chains: chains,
            vocabulary_shaping,
            shaping_counts: counts,
            script_history,
            script_drift,
        }
    }
}

fn rarity_label(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Typical => "typical",
        Rarity::Unusual => "unusual",
        Rarity::Rare => "rare",
    }
}

fn complexity_label(complexity: Complexity) -> &'static str {
    match complexity {
        Complexity::Simple => "simple",
        Complexity::Moderate => "moderate",
        Complexity::Complex => "complex",
    }
}

/// Renders a profile as §17's scored-dimensions block (terminal text).
///
/// In the library, not the CLI (the `render_family` precedent): the M11 UI renders
/// the identical text through this function. Pure; newline-terminated; padded by
/// **char count** (for the `§18` glyph); no map, no float. The closing line makes
/// §17's "transparent, not authoritarian" literal.
pub fn render_profile(profile: &PlausibilityProfile, name: &str) -> String {
    // The dimension labels and their column width (char count, for alignment).
    const RARITY: &str = "Typological rarity";
    const COMPLEXITY: &str = "Phonotactic complexity";
    const DEPTH: &str = "Historical depth";
    const MORPH: &str = "Morphological irregularity";
    const SEMANTIC: &str = "Semantic drift";
    const CULTURE: &str = "Vocabulary shaping";
    const SCRIPT: &str = "Script history";
    let width = [RARITY, COMPLEXITY, DEPTH, MORPH, SEMANTIC, CULTURE, SCRIPT]
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let pad = |label: &str| crate::pad(label, width);

    let depth_line = match profile.historical_depth {
        HistoricalDepth::None => "none  (no recorded sound changes)".to_owned(),
        band => format!(
            "{}  ({} recorded sound change{} over {} year{})",
            match band {
                HistoricalDepth::Shallow => "shallow",
                HistoricalDepth::Moderate => "moderate",
                HistoricalDepth::Deep => "deep",
                HistoricalDepth::None => unreachable!(),
            },
            profile.recorded_changes,
            if profile.recorded_changes == 1 {
                ""
            } else {
                "s"
            },
            profile.elapsed_years,
            if profile.elapsed_years == 1 { "" } else { "s" },
        ),
    };

    // The morphology line: a band plus its raw basis, the `depth_line` shape.
    // `None` reads honestly as "no affixation" rather than as a zero score.
    let morph_line = match profile.morphological_irregularity {
        MorphologicalIrregularity::None => "none  (no affixation)".to_owned(),
        band => {
            let label = match band {
                MorphologicalIrregularity::Regular => "regular",
                MorphologicalIrregularity::Allomorphic => "allomorphic",
                MorphologicalIrregularity::HighlyAllomorphic => "highly allomorphic",
                MorphologicalIrregularity::None => unreachable!(),
            };
            // Each affix's distinct-shape count, e.g. `PL: 2` — the basis, by gloss.
            let detail = profile
                .affix_allomorphy
                .iter()
                .map(|(gloss, n)| format!("{gloss}: {n}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{label}  ({detail})")
        }
    };

    // The semantics line: a band plus its raw basis, the `morph_line` shape.
    // `None` ("nothing declared") and `Stable` ("declared, and it has not moved")
    // are two different facts and must read differently — neither is a zero score.
    let semantic_line = match profile.semantic_drift {
        SemanticDrift::None => "none  (no senses declared)".to_owned(),
        SemanticDrift::Stable => "stable  (no recorded shift)".to_owned(),
        band => {
            let label = match band {
                SemanticDrift::Drifted => "drifted",
                SemanticDrift::HighlyDrifted => "highly drifted",
                SemanticDrift::None | SemanticDrift::Stable => unreachable!(),
            };
            // Each drifted word's recorded shift count, e.g. `omen: 2`.
            let detail = profile
                .sense_chains
                .iter()
                .map(|(gloss, n)| format!("{gloss}: {n}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{label}  ({detail})")
        }
    };

    // M15: the band plus its raw basis, the `semantic_line` shape. `Unshaped` is
    // "no profile at all", not a zero — and it names what that silently asserts.
    let (absent, elaborated, extra) = profile.shaping_counts;
    let culture_line = match profile.vocabulary_shaping {
        VocabularyShaping::Unshaped => {
            "unshaped  (no culture profile; every meaning coined)".to_owned()
        }
        band => {
            let label = match band {
                VocabularyShaping::Shaped => "shaped",
                VocabularyShaping::HeavilyShaped => "heavily shaped",
                VocabularyShaping::Unshaped => unreachable!(),
            };
            format!("{label}  ({absent} uncoined, {elaborated} elaborated into {extra} extra)")
        }
    };

    // M21: the band plus its raw basis, the `culture_line` shape. §17's script-history
    // row, and the last one M7 deferred. `Unwritten` is a fact about a language, not a
    // low score: most languages that have ever existed were never written down.
    let script_line = match profile.script_history {
        ScriptHistory::Unwritten => "unwritten  (no script declared)".to_owned(),
        ScriptHistory::Unmeasured => {
            "not yet measurable  (a script, but no lexicon to hold it against)".to_owned()
        }
        ScriptHistory::SoundIndependent => {
            "sound-independent  (writes meanings; no pronunciation to drift from)".to_owned()
        }
        band => {
            let label = match band {
                ScriptHistory::Phonemic => "phonemic",
                ScriptHistory::Historical => "historical",
                ScriptHistory::Deep => "deep",
                ScriptHistory::Unwritten
                | ScriptHistory::Unmeasured
                | ScriptHistory::SoundIndependent => unreachable!(),
            };
            let basis: Vec<String> = profile
                .script_drift
                .iter()
                .map(|(id, n)| format!("{id} {n}"))
                .collect();
            format!(
                "{label}  ({} mismatch(es): {})",
                profile
                    .script_drift
                    .iter()
                    .map(|(_, n)| *n)
                    .max()
                    .unwrap_or(0),
                basis.join(", ")
            )
        }
    };

    let mut out = String::new();
    out.push_str(&format!("Plausibility profile — {name}\n\n"));
    out.push_str(&format!(
        "  {RARITY}{}  {}\n",
        pad(RARITY),
        rarity_label(profile.rarity)
    ));
    out.push_str(&format!(
        "  {COMPLEXITY}{}  {}  (declared root templates)\n",
        pad(COMPLEXITY),
        complexity_label(profile.complexity)
    ));
    out.push_str(&format!("  {DEPTH}{}  {depth_line}\n", pad(DEPTH)));
    out.push_str(&format!("  {MORPH}{}  {morph_line}\n", pad(MORPH)));
    out.push_str(&format!("  {SEMANTIC}{}  {semantic_line}\n", pad(SEMANTIC)));
    out.push_str(&format!("  {CULTURE}{}  {culture_line}\n", pad(CULTURE)));
    out.push_str(&format!("  {SCRIPT}{}  {script_line}\n", pad(SCRIPT)));

    out.push_str("\n  not yet modelled:\n");
    // The unbuilt §17 dimensions, in NOT_MODELLED order — no fabricated score.
    let nm_width = NOT_MODELLED
        .iter()
        .map(|d| d.label().chars().count())
        .max()
        .unwrap_or(0);
    for dim in NOT_MODELLED {
        let gap = crate::pad(dim.label(), nm_width);
        out.push_str(&format!(
            "    {}{}  {}\n",
            dim.label(),
            gap,
            dim.milestone()
        ));
    }

    out.push_str(
        "\n  This profile describes; it does not police. Bands sit against attested\n  \
         typological ranges — \"unusual\" is not \"wrong\".\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_phonology::{FeatureBundle, Phoneme, PhonemeInventory, SegmentKind};

    fn vowel(id: &str) -> Phoneme {
        let bundle = FeatureBundle::try_from(
            [
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
            ]
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>(),
        )
        .expect("valid vowel");
        Phoneme::new(id, "a", SegmentKind::Vowel).with_features(bundle)
    }

    fn consonant(id: &str, ipa: &str) -> Phoneme {
        let bundle = FeatureBundle::try_from(
            [
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
            ]
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>(),
        )
        .expect("valid consonant");
        Phoneme::new(id, ipa, SegmentKind::Consonant).with_features(bundle)
    }

    /// A genome shaped like Proto-Asterian's inventory (10C/5V) — but the profile
    /// only reads counts and templates, so distinct featured phonemes suffice.
    fn reference_shaped() -> LanguageGenome {
        let mut phonemes = Vec::new();
        for i in 0..10 {
            phonemes.push(consonant(
                &format!("ph_c{i}"),
                // Distinct ipa per consonant to avoid duplicate_ipa.
                match i {
                    0 => "p",
                    1 => "t",
                    2 => "k",
                    3 => "m",
                    4 => "n",
                    5 => "s",
                    6 => "l",
                    7 => "r",
                    8 => "w",
                    _ => "j",
                },
            ));
        }
        for i in 0..5 {
            phonemes.push(Phoneme::new(
                format!("ph_v{i}"),
                match i {
                    0 => "a",
                    1 => "e",
                    2 => "i",
                    3 => "o",
                    _ => "u",
                },
                SegmentKind::Vowel,
            ));
        }
        // Re-feature the vowels so the inventory is well-formed.
        let phonemes: Vec<Phoneme> = phonemes
            .into_iter()
            .map(|p| {
                if p.kind == SegmentKind::Vowel {
                    vowel(p.id.as_str())
                } else {
                    p
                }
            })
            .collect();
        LanguageGenome::proto("ref", "Reference")
            .with_phonemes(PhonemeInventory::from_phonemes(phonemes))
            .with_phonotactics(stem_phonology::Phonotactics {
                templates: vec![
                    stem_phonology::WeightedTemplate::new("CV"),
                    stem_phonology::WeightedTemplate::new("CVC"),
                    stem_phonology::WeightedTemplate::new("V"),
                    stem_phonology::WeightedTemplate::new("VC"),
                ],
                syllables_per_root: vec![stem_phonology::WeightedSyllableCount::new(1)],
            })
    }

    #[test]
    fn proto_asterian_shaped_inventory_scores_typical_simple_and_no_history() {
        let profile = reference_shaped().plausibility_profile();
        assert_eq!(profile.rarity, Rarity::Typical);
        assert_eq!(profile.complexity, Complexity::Simple);
        assert_eq!(profile.historical_depth, HistoricalDepth::None);
    }

    #[test]
    fn an_extreme_inventory_scores_rare() {
        let mut phonemes: Vec<Phoneme> = (0..60)
            .map(|i| consonant(&format!("ph_c{i}"), &format!("c{i}")))
            .collect();
        phonemes.push(vowel("ph_a"));
        let genome = LanguageGenome::proto("x", "X")
            .with_phonemes(PhonemeInventory::from_phonemes(phonemes));
        assert_eq!(genome.plausibility_profile().rarity, Rarity::Rare);
    }

    #[test]
    fn a_clustered_phonotactics_scores_complex() {
        let genome = reference_shaped().with_phonotactics(stem_phonology::Phonotactics {
            templates: vec![stem_phonology::WeightedTemplate::new("CCCVC")],
            syllables_per_root: vec![stem_phonology::WeightedSyllableCount::new(1)],
        });
        assert_eq!(
            genome.plausibility_profile().complexity,
            Complexity::Complex
        );
    }

    #[test]
    fn the_profile_is_a_pure_function_of_the_genome() {
        let genome = reference_shaped();
        assert_eq!(genome.plausibility_profile(), genome.plausibility_profile());
    }

    #[test]
    fn render_profile_is_deterministic_and_names_the_scored_and_deferred_dimensions() {
        let genome = reference_shaped();
        let a = render_profile(&genome.plausibility_profile(), &genome.name);
        let b = render_profile(&genome.plausibility_profile(), &genome.name);
        assert_eq!(a, b, "the renderer is a pure function");
        // M8 filled the morphological-irregularity row: it is now a SCORED band
        // (the reference proto has no affixation → "none"), not a deferred line.
        assert!(
            a.contains("Morphological irregularity"),
            "the scored morphology dimension appears: {a}"
        );
        assert!(
            !a.contains("M8"),
            "morphology left the not-yet-modelled block at M8: {a}"
        );
        // M9 filled the semantic row the same way. Asserted as an ABSENCE, and
        // deliberately: the old `contains("M9")` would have kept passing on
        // `ScriptHistoryCoherence`'s "M9+" long after semantics shipped, so the
        // test would have gone quietly dishonest. The one surviving dimension names a
        // design section (`§18`) because it does not sit at a numbered milestone.
        assert!(
            a.contains("Semantic drift"),
            "the scored semantic dimension appears: {a}"
        );
        assert!(
            !a.contains("M9"),
            "semantics left the not-yet-modelled block at M9: {a}"
        );
        // M21 filled the script row, so §7.6 left this block too — asserted as an
        // absence for the reason M9's is.
        assert!(
            a.contains("Script history"),
            "the scored script dimension appears: {a}"
        );
        assert!(a.contains("not yet modelled"), "{a}");
        assert!(
            !a.contains("§7.6"),
            "script history left the not-yet-modelled block at M21: {a}"
        );
        assert!(
            a.contains("§18"),
            "alien modality is the last deferred dimension: {a}"
        );
        assert!(!a.contains('%'), "no fabricated percentage: {a}");
        assert!(
            !a.to_lowercase().contains("word order") && !a.to_lowercase().contains("syntax"),
            "no syntax/word-order row: {a}"
        );
    }

    /// A proto with a monomorphemic (or empty) lexicon has no affixation, so the
    /// band is honestly `None` — a fact, not a zero score.
    #[test]
    fn a_language_with_no_affixation_scores_no_morphological_irregularity() {
        let profile = reference_shaped().plausibility_profile();
        assert_eq!(
            profile.morphological_irregularity,
            MorphologicalIrregularity::None
        );
        assert!(profile.affix_allomorphy.is_empty());
    }

    /// Builds a genome whose lexicon records one affix `m_pl` realised by the given
    /// distinct surface segment sequences (one word each), so the measure reads
    /// exactly that many allomorphs. Synthetic by design — it exercises the
    /// band/Note projection, not a realistic derivation.
    fn genome_with_affix_allomorphs(surfaces: &[&[&str]]) -> LanguageGenome {
        use stem_core::{CognateSetId, PhonemeId, WordId};
        use stem_lexicon::{
            Lexicon, MorphemeRef, MorphemeRole, PartOfSpeech, WordEntry, WordSource,
        };
        use stem_phonology::{Root, Syllable};

        let entries = surfaces.iter().enumerate().map(|(i, segs)| {
            let ordinal = i + 1;
            WordEntry {
                id: WordId::sequential(ordinal),
                concept: None,
                phonemic_form: Root {
                    syllables: vec![Syllable {
                        pattern: "X".to_owned(),
                        segments: segs.iter().map(|s| PhonemeId::new(*s)).collect(),
                        stress: None,
                    }],
                },
                glosses: vec!["thing PL".to_owned()],
                part_of_speech: PartOfSpeech::Noun,
                cognate_set: CognateSetId::new(format!("cog_x_{ordinal:04}")),
                source: WordSource::Derived,
                trace: None,
                // The whole form IS the affix, so its span is the whole word — the
                // measure reads `segs` as this occurrence's surface allomorph.
                morphemes: vec![MorphemeRef {
                    morpheme: stem_core::MorphemeId::new("m_pl"),
                    role: MorphemeRole::Suffix,
                    gloss: "PL".to_owned(),
                    start: 0,
                    end: segs.len() as u32,
                }],
                // Monomorphemic on the derivation axis: this is an inflected cell,
                // not a compound, so it has an affix ref and no base (M14).
                bases: Vec::new(),
                // This fixture exercises the morphology band; it declares no
                // senses, so the semantic band reads `None`.
                senses: Vec::new(),
                sense_history: None,
            }
        });
        reference_shaped().with_lexicon(Lexicon::from_entries(entries))
    }

    /// The ADR-0009 projection: the `HighlyAllomorphic` band holds **exactly** when
    /// the `high_morphological_irregularity` validation Note fires — both read the
    /// one shared `HIGH_ALLOMORPH_COUNT`, so they can never disagree.
    #[test]
    fn the_highly_allomorphic_band_and_the_note_are_the_same_threshold() {
        use stem_core::Validate;

        // In-inventory segments (`reference_shaped` declares ph_c0.. and ph_v0..),
        // so the only issue in play is the morphology Note we are pinning.
        // Two allomorphs: Allomorphic band, and the Note must stay silent.
        let two = genome_with_affix_allomorphs(&[&["ph_c0", "ph_v0"], &["ph_c1", "ph_v0"]]);
        assert_eq!(
            two.plausibility_profile().morphological_irregularity,
            MorphologicalIrregularity::Allomorphic
        );
        assert!(
            !two.validate()
                .issues
                .iter()
                .any(|i| i.code == "high_morphological_irregularity"),
            "a two-way split is ordinary and must not fire the Note"
        );

        // Three allomorphs: HighlyAllomorphic band, and the Note must fire.
        let three = genome_with_affix_allomorphs(&[
            &["ph_c0", "ph_v0"],
            &["ph_c1", "ph_v0"],
            &["ph_c2", "ph_v0"],
        ]);
        assert_eq!(
            three.plausibility_profile().morphological_irregularity,
            MorphologicalIrregularity::HighlyAllomorphic
        );
        let report = three.validate();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "high_morphological_irregularity"),
            "the band is HighlyAllomorphic, so the paired Note must fire: {report}"
        );
        // Report, do not police (§17): the morphology check contributes no Error —
        // extreme allomorphy is unusual, not broken. (Asserted on this check
        // specifically, since `reference_shaped`'s toy inventory carries its own
        // unrelated `duplicate_ipa` Error.)
        assert!(
            !report
                .errors()
                .any(|i| i.code == "high_morphological_irregularity"),
            "morphological irregularity must never be an Error: {report}"
        );
    }

    /// Builds a genome whose single word records `shifts` drift steps, so the
    /// semantic band and its paired Note can be exercised against one number.
    /// Synthetic by design — it pins the projection, not a realistic history.
    fn genome_with_sense_chain(shifts: usize) -> LanguageGenome {
        use stem_core::{EventId, PhonemeId, SemanticNodeId, WordId};
        use stem_lexicon::{
            Lexicon, PartOfSpeech, SemanticNode, SemanticSpace, SenseHistory, SenseRef, SenseShift,
            WordEntry, WordSource,
        };
        use stem_phonology::{Root, Syllable};

        // `sn_0` is the inherited sense; each step swaps it for the next.
        let nodes: Vec<SemanticNode> = (0..=shifts)
            .map(|i| SemanticNode {
                id: SemanticNodeId::new(format!("sn_{i}")),
                gloss: format!("sense {i}"),
                concept: None,
                note: String::new(),
            })
            .collect();
        let steps: Vec<SenseShift> = (0..shifts)
            .map(|i| SenseShift {
                event: EventId::sequential(i + 1),
                index: i as u32,
                removed: vec![SemanticNodeId::new(format!("sn_{i}"))],
                added: vec![SemanticNodeId::new(format!("sn_{}", i + 1))],
            })
            .collect();

        let entry = WordEntry {
            id: WordId::sequential(1),
            concept: None,
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![PhonemeId::new("ph_c0"), PhonemeId::new("ph_v0")],
                    stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: stem_lexicon::scoped_cognate_set(&stem_core::LanguageId::new("ref"), 1),
            source: WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: vec![SenseRef {
                node: SemanticNodeId::new(format!("sn_{shifts}")),
                gloss: format!("sense {shifts}"),
            }],
            sense_history: Some(SenseHistory {
                input: vec![SemanticNodeId::new("sn_0")],
                steps,
            }),
        };
        reference_shaped()
            .with_lexicon(Lexicon::from_entries([entry]))
            .with_semantics(SemanticSpace { nodes })
    }

    /// The ADR-0009 projection for M9: the `HighlyDrifted` band holds **exactly**
    /// when `long_semantic_drift_chain` fires — both read the one
    /// `LONG_SENSE_CHAIN`, so they can never disagree.
    #[test]
    fn the_highly_drifted_band_and_the_note_are_the_same_threshold() {
        use stem_core::Validate;

        // Two shifts — the M9 demo's own Coastal chain. Drifted, and quiet.
        let ordinary = genome_with_sense_chain(2);
        assert_eq!(
            ordinary.plausibility_profile().semantic_drift,
            SemanticDrift::Drifted
        );
        assert!(
            !ordinary
                .validate()
                .issues
                .iter()
                .any(|i| i.code == "long_semantic_drift_chain"),
            "the tool's own showcase must sit below the extreme bar"
        );

        // Three — the threshold. Band flips, Note fires, still not an Error.
        let long = genome_with_sense_chain(LONG_SENSE_CHAIN);
        assert_eq!(
            long.plausibility_profile().semantic_drift,
            SemanticDrift::HighlyDrifted
        );
        let report = long.validate();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "long_semantic_drift_chain"),
            "the band is HighlyDrifted, so the paired Note must fire: {report}"
        );
        assert!(
            !report
                .errors()
                .any(|i| i.code == "long_semantic_drift_chain"),
            "a long chain is unusual, not broken (§17): {report}"
        );
    }

    /// `None` and `Stable` are different facts: nothing declared versus declared
    /// and unmoved. Neither is a fabricated low score.
    #[test]
    fn a_language_with_no_senses_scores_none_and_one_with_unmoved_senses_scores_stable() {
        assert_eq!(
            reference_shaped().plausibility_profile().semantic_drift,
            SemanticDrift::None,
            "nothing semantic is modelled at all"
        );
        assert_eq!(
            genome_with_sense_chain(0)
                .plausibility_profile()
                .semantic_drift,
            SemanticDrift::Stable,
            "senses exist and have not moved"
        );
    }
}
