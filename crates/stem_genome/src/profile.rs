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

/// A §17 dimension no shipped milestone can measure yet. Carried explicitly so the
/// profile is transparent about its own coverage rather than silently omitting
/// four of §17's rows — or, worse, fabricating a number for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotModelled {
    MorphologicalIrregularity,
    SemanticPlausibility,
    ScriptHistoryCoherence,
    AlienEmbodimentDependence,
}

impl NotModelled {
    /// The §17 dimension name.
    pub fn label(self) -> &'static str {
        match self {
            Self::MorphologicalIrregularity => "Morphological irregularity",
            Self::SemanticPlausibility => "Semantic plausibility",
            Self::ScriptHistoryCoherence => "Script-history coherence",
            Self::AlienEmbodimentDependence => "Alien embodiment dependence",
        }
    }

    /// The milestone that will make it measurable.
    pub fn milestone(self) -> &'static str {
        match self {
            Self::MorphologicalIrregularity => "M8",
            Self::SemanticPlausibility => "M9",
            Self::ScriptHistoryCoherence => "M9+",
            Self::AlienEmbodimentDependence => "§18",
        }
    }
}

/// The §17 dimensions M7 defers, in §17's listed order — one list the renderer and
/// future milestones share, so a renumber is a one-line change. Syntax / word
/// order is deliberately absent: §20.1 forbids a syntax engine, and a
/// "not modelled" line would invite the build.
pub const NOT_MODELLED: &[NotModelled] = &[
    NotModelled::MorphologicalIrregularity,
    NotModelled::SemanticPlausibility,
    NotModelled::ScriptHistoryCoherence,
    NotModelled::AlienEmbodimentDependence,
];

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

        PlausibilityProfile {
            rarity: self.phonemes.rarity(),
            complexity: self.phonotactics.complexity(),
            historical_depth,
            recorded_changes,
            elapsed_years: years,
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
    let width = [RARITY, COMPLEXITY, DEPTH]
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let pad = |label: &str| " ".repeat(width - label.chars().count());

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

    out.push_str("\n  not yet modelled:\n");
    // The unbuilt §17 dimensions, in NOT_MODELLED order — no fabricated score.
    let nm_width = NOT_MODELLED
        .iter()
        .map(|d| d.label().chars().count())
        .max()
        .unwrap_or(0);
    for dim in NOT_MODELLED {
        let gap = " ".repeat(nm_width - dim.label().chars().count());
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
    fn render_profile_is_deterministic_and_names_the_unmodelled_dimensions() {
        let genome = reference_shaped();
        let a = render_profile(&genome.plausibility_profile(), &genome.name);
        let b = render_profile(&genome.plausibility_profile(), &genome.name);
        assert_eq!(a, b, "the renderer is a pure function");
        assert!(a.contains("not yet modelled"), "{a}");
        assert!(a.contains("M8"), "names the milestone: {a}");
        assert!(!a.contains('%'), "no fabricated percentage: {a}");
        assert!(
            !a.to_lowercase().contains("word order") && !a.to_lowercase().contains("syntax"),
            "no syntax/word-order row: {a}"
        );
    }
}
