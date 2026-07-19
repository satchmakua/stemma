//! The phoneme inventory and its validation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use stem_core::{Issue, PhonemeId, Severity, Validate, ValidationReport};

use crate::phoneme::{Phoneme, SegmentKind};

/// Above this consonant-to-vowel ratio the inventory is flagged as typologically
/// lopsided. Natural languages cluster around 4:1; Ubykh, an extreme real case,
/// reaches roughly 40:1. The threshold sits well past attested territory so it
/// warns about designs that are genuinely unusual, not merely consonant-heavy.
const LOPSIDED_CONSONANT_VOWEL_RATIO: f32 = 20.0;

/// The set of contrastive sounds available to one language.
///
/// Order is preserved. Inventories are authored by hand in fixtures and read in
/// exports, and phonologists group segments meaningfully (stops, then fricatives,
/// then nasals…) — reordering them would destroy information the author put there.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhonemeInventory {
    phonemes: Vec<Phoneme>,
}

impl PhonemeInventory {
    /// An empty inventory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds an inventory from a sequence of phonemes.
    ///
    /// No validation happens here — construction always succeeds, and
    /// [`Validate::validate`] reports what is wrong. Keeping the two apart lets
    /// the CLI load a broken fixture and *explain* it rather than just refusing.
    pub fn from_phonemes(phonemes: impl IntoIterator<Item = Phoneme>) -> Self {
        Self {
            phonemes: phonemes.into_iter().collect(),
        }
    }

    /// Appends a phoneme.
    pub fn push(&mut self, phoneme: Phoneme) -> &mut Self {
        self.phonemes.push(phoneme);
        self
    }

    /// Looks a phoneme up by ID.
    pub fn get(&self, id: &PhonemeId) -> Option<&Phoneme> {
        self.phonemes.iter().find(|p| &p.id == id)
    }

    /// Looks a phoneme up by ID, erroring if absent.
    pub fn require(&self, id: &PhonemeId) -> stem_core::Result<&Phoneme> {
        self.get(id)
            .ok_or_else(|| stem_core::StemmaError::not_found("phoneme", id))
    }

    /// Every phoneme, in authored order.
    pub fn iter(&self) -> impl Iterator<Item = &Phoneme> {
        self.phonemes.iter()
    }

    /// Every phoneme of a given kind, in authored order.
    pub fn of_kind(&self, kind: SegmentKind) -> impl Iterator<Item = &Phoneme> {
        self.phonemes.iter().filter(move |p| p.kind == kind)
    }

    /// The consonants.
    pub fn consonants(&self) -> impl Iterator<Item = &Phoneme> {
        self.of_kind(SegmentKind::Consonant)
    }

    /// The vowels.
    pub fn vowels(&self) -> impl Iterator<Item = &Phoneme> {
        self.of_kind(SegmentKind::Vowel)
    }

    /// How many phonemes the inventory holds.
    pub fn len(&self) -> usize {
        self.phonemes.len()
    }

    /// Whether the inventory holds no phonemes.
    pub fn is_empty(&self) -> bool {
        self.phonemes.is_empty()
    }
}

impl<'a> IntoIterator for &'a PhonemeInventory {
    type Item = &'a Phoneme;
    type IntoIter = std::slice::Iter<'a, Phoneme>;

    fn into_iter(self) -> Self::IntoIter {
        self.phonemes.iter()
    }
}

impl FromIterator<Phoneme> for PhonemeInventory {
    fn from_iter<T: IntoIterator<Item = Phoneme>>(iter: T) -> Self {
        Self::from_phonemes(iter)
    }
}

impl Validate for PhonemeInventory {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.phonemes.is_empty() {
            report.error("empty", "the inventory has no phonemes");
            return report;
        }

        // --- Structural integrity: these break the engine. ---
        let mut seen_ids: HashMap<&str, usize> = HashMap::new();
        let mut seen_ipa: HashMap<&str, usize> = HashMap::new();

        for phoneme in &self.phonemes {
            if phoneme.id.is_empty() {
                report.push(
                    Issue::new(Severity::Error, "empty_id", "a phoneme has an empty id")
                        .about(&phoneme.ipa),
                );
            }
            if phoneme.ipa.trim().is_empty() {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "empty_ipa",
                        "a phoneme has an empty IPA form",
                    )
                    .about(&phoneme.id),
                );
            }
            if !phoneme.frequency_weight.is_finite() || phoneme.frequency_weight <= 0.0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "bad_weight",
                        format!(
                            "frequency weight must be finite and positive, got {}",
                            phoneme.frequency_weight
                        ),
                    )
                    .about(&phoneme.id),
                );
            }

            *seen_ids.entry(phoneme.id.as_str()).or_default() += 1;
            *seen_ipa.entry(phoneme.ipa.as_str()).or_default() += 1;
        }

        for (id, count) in sorted_duplicates(&seen_ids) {
            report.push(
                Issue::new(
                    Severity::Error,
                    "duplicate_id",
                    format!("{count} phonemes share this id; ids must be unique"),
                )
                .about(id),
            );
        }

        // Two phonemes with the same IPA are not contrastive — by definition they
        // are one phoneme. This is a modelling error, not a stylistic one.
        for (ipa, count) in sorted_duplicates(&seen_ipa) {
            report.push(
                Issue::new(
                    Severity::Error,
                    "duplicate_ipa",
                    format!(
                        "{count} phonemes share the IPA form /{ipa}/; they are not contrastive"
                    ),
                )
                .about(ipa),
            );
        }

        // --- Scientific sanity: DESIGN.md §16.5. ---
        let vowels = self.vowels().count();
        let consonants = self.consonants().count();

        if vowels == 0 {
            report.error(
                "no_nucleus",
                "no phoneme can be a syllable nucleus, so no syllable can be formed \
                 (a non-vocal language will need the alien modality model of §7.7)",
            );
        }

        // --- Plausibility: unusual, but the user may mean it (§17). ---
        if consonants == 0 {
            report.warn(
                "no_consonants",
                "the inventory is all vowels; this is typologically unattested",
            );
        } else if vowels > 0 {
            let ratio = consonants as f32 / vowels as f32;
            if ratio > LOPSIDED_CONSONANT_VOWEL_RATIO {
                report.warn(
                    "lopsided_inventory",
                    format!(
                        "{consonants} consonants to {vowels} vowels (ratio {ratio:.1}:1) is far \
                         outside the attested range; possible as a speculative design, but it \
                         deserves a historical explanation"
                    ),
                );
            }
        }

        if self.phonemes.len() < 5 {
            report.note(
                "very_small_inventory",
                format!(
                    "{} phonemes is smaller than any attested language (Rotokas, near the floor, \
                     has around 11)",
                    self.phonemes.len()
                ),
            );
        }

        report
    }
}

/// Duplicate keys with their counts, sorted for deterministic report ordering.
///
/// `HashMap` iteration order varies between runs; validation output is snapshot-
/// tested and read by humans, so it must not.
fn sorted_duplicates<'a>(counts: &HashMap<&'a str, usize>) -> Vec<(&'a str, usize)> {
    let mut duplicates: Vec<_> = counts
        .iter()
        .filter(|&(_, &count)| count > 1)
        .map(|(&key, &count)| (key, count))
        .collect();
    duplicates.sort_unstable();
    duplicates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel),
            Phoneme::new("ph_u", "u", SegmentKind::Vowel),
        ])
    }

    #[test]
    fn a_sound_inventory_reports_no_errors() {
        let report = tiny_inventory().validate();
        assert!(report.is_ok(), "unexpected errors: {report}");
    }

    #[test]
    fn an_empty_inventory_is_an_error() {
        let report = PhonemeInventory::new().validate();
        assert!(!report.is_ok());
        assert_eq!(report.errors().next().unwrap().code, "empty");
    }

    #[test]
    fn duplicate_ids_are_caught() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant),
            Phoneme::new("ph_p", "b", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let report = inventory.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_id"),
            "{report}"
        );
    }

    #[test]
    fn duplicate_ipa_forms_are_caught() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p1", "p", SegmentKind::Consonant),
            Phoneme::new("ph_p2", "p", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let report = inventory.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_ipa"),
            "{report}"
        );
    }

    #[test]
    fn an_inventory_with_no_vowels_cannot_form_a_syllable() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
        ]);
        let report = inventory.validate();
        assert!(report.errors().any(|i| i.code == "no_nucleus"), "{report}");
    }

    #[test]
    fn a_lopsided_inventory_warns_but_stays_valid() {
        let mut phonemes: Vec<_> = (0..60)
            .map(|n| {
                Phoneme::new(
                    PhonemeId::sequential(n),
                    format!("c{n}"),
                    SegmentKind::Consonant,
                )
            })
            .collect();
        phonemes.push(Phoneme::new("ph_a", "a", SegmentKind::Vowel));

        let report = PhonemeInventory::from_phonemes(phonemes).validate();
        assert!(
            report.is_ok(),
            "a speculative design must not be rejected outright: {report}"
        );
        assert!(
            report.warnings().any(|i| i.code == "lopsided_inventory"),
            "{report}"
        );
    }

    #[test]
    fn non_positive_weights_are_rejected() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant).with_weight(0.0),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let report = inventory.validate();
        assert!(report.errors().any(|i| i.code == "bad_weight"), "{report}");
    }

    #[test]
    fn validation_collects_every_problem_not_just_the_first() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "", SegmentKind::Consonant),
            Phoneme::new("ph_p", "t", SegmentKind::Consonant),
        ]);
        let report = inventory.validate();
        // empty_ipa + duplicate_id + no_nucleus — one pass should surface all three.
        assert!(report.errors().any(|i| i.code == "empty_ipa"), "{report}");
        assert!(
            report.errors().any(|i| i.code == "duplicate_id"),
            "{report}"
        );
        assert!(report.errors().any(|i| i.code == "no_nucleus"), "{report}");
    }

    #[test]
    fn lookup_finds_phonemes_by_id_and_reports_misses() {
        let inventory = tiny_inventory();
        assert_eq!(inventory.get(&PhonemeId::new("ph_k")).unwrap().ipa, "k");
        assert!(inventory.get(&PhonemeId::new("ph_zzz")).is_none());
        assert!(inventory.require(&PhonemeId::new("ph_zzz")).is_err());
    }

    #[test]
    fn kind_filters_partition_the_inventory() {
        let inventory = tiny_inventory();
        assert_eq!(inventory.consonants().count(), 3);
        assert_eq!(inventory.vowels().count(), 3);
        assert_eq!(
            inventory.consonants().count() + inventory.vowels().count(),
            inventory.len()
        );
    }

    #[test]
    fn authored_order_is_preserved() {
        let inventory = tiny_inventory();
        let ipas: Vec<_> = inventory.iter().map(|p| p.ipa.as_str()).collect();
        assert_eq!(ipas, ["p", "t", "k", "a", "i", "u"]);
    }
}
