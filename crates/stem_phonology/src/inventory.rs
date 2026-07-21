//! The phoneme inventory and its validation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use stem_core::{Issue, PhonemeId, Severity, Validate, ValidationReport};

use crate::features::Feature;
use crate::phoneme::{Phoneme, SegmentKind};

/// Questions that always have an answer for a spoken segment. A segment that ducks
/// one cannot be matched reliably by any M3 rule, so leaving one out is an error
/// rather than a stylistic choice.
const REQUIRED_OF_ALL: &[Feature] = &[
    Feature::Syllabic,
    Feature::Consonantal,
    Feature::Sonorant,
    Feature::Approximant,
    Feature::Continuant,
    Feature::Nasal,
    Feature::Lateral,
    Feature::Trill,
    Feature::Voice,
    Feature::Labial,
    Feature::Coronal,
    Feature::Dorsal,
];

/// Height, backness and rounding are contrastive for anything articulated with the
/// tongue body — every vowel, every velar, and both glides. Requiring them of all
/// and only `[+dorsal]` segments is what makes `[+dorsal]` a usable class.
const REQUIRED_OF_DORSAL: &[Feature] =
    &[Feature::High, Feature::Low, Feature::Back, Feature::Round];

/// Rounding is a labial gesture, so a labial segment always answers it.
const REQUIRED_OF_LABIAL: &[Feature] = &[Feature::Round];

/// Which required features this bundle fails to value, in frozen [`Feature::ALL`]
/// order.
///
/// **The single implementation of the `REQUIRED_OF_ALL` / `REQUIRED_OF_DORSAL` /
/// `REQUIRED_OF_LABIAL` geometry**, extracted from `check_features` at M3 so it
/// has exactly three callers:
///
/// 1. [`PhonemeInventory`]'s `validate` → `missing_required_feature` (Error);
/// 2. the reference table's construction test — every row is a legal phoneme;
/// 3. the rule engine — a site whose output bundle fails this is not applied.
///
/// If the engine could write a bundle the validator calls broken, the two would
/// disagree. They cannot, because they are the same function. This is the
/// concrete discharge of the defect class this project has already been bitten
/// by.
pub fn required_features_missing(bundle: crate::FeatureBundle) -> Vec<Feature> {
    use crate::Sign;
    let dorsal = bundle.is(Feature::Dorsal, Sign::Plus);
    let labial = bundle.is(Feature::Labial, Sign::Plus);
    Feature::ALL
        .iter()
        .copied()
        .filter(|f| {
            let required = REQUIRED_OF_ALL.contains(f)
                || (dorsal && REQUIRED_OF_DORSAL.contains(f))
                || (labial && REQUIRED_OF_LABIAL.contains(f));
            required && !bundle.is_specified(*f)
        })
        .collect()
}

/// Why a feature was required *of this bundle*, for the diagnostic message.
///
/// Takes the bundle because `round` sits in both conditional tables: on a
/// `[+labial -dorsal]` segment it is the labial rule that demands it, and naming
/// the dorsal rule there would send the author looking at the wrong articulator.
fn requirement_reason(feature: Feature, bundle: crate::FeatureBundle) -> &'static str {
    use crate::Sign;
    if REQUIRED_OF_ALL.contains(&feature) {
        "required of every segment"
    } else if bundle.is(Feature::Dorsal, Sign::Plus) && REQUIRED_OF_DORSAL.contains(&feature) {
        "required of every [+dorsal] segment"
    } else {
        "required of every [+labial] segment"
    }
}

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

    /// The generation candidates of one slot class: ids and weights, in **authored
    /// order**, as parallel `Vec`s.
    ///
    /// Authored order is part of the determinism contract (`DESIGN.md` §9.4) — the
    /// weights reach a prefix-sum array, so reordering them rewrites every
    /// generated language. Returning `Vec`s rather than an iterator over a map is
    /// the point, not an implementation detail.
    pub fn candidates(&self, kind: SegmentKind) -> (Vec<PhonemeId>, Vec<u32>) {
        self.of_kind(kind)
            .map(|p| (p.id.clone(), p.frequency_weight))
            .unzip()
    }

    /// The first phoneme whose bundle is byte-identical to `bundle`, in authored
    /// order.
    ///
    /// "First in authored order" is a pin, not an accident: `identical_features`
    /// is deliberately only a Warning (length, tone and phonation are not
    /// modelled, so /a/ and /aː/ legitimately collide), so a valid language may
    /// contain two phonemes with one bundle. When it does, resolution reports
    /// `ambiguous_target_symbol` rather than choosing silently — see
    /// [`Self::all_by_bundle`].
    pub fn by_bundle(&self, bundle: crate::FeatureBundle) -> Option<&Phoneme> {
        self.phonemes.iter().find(|p| p.features == bundle)
    }

    /// Every phoneme with this exact bundle, in authored order.
    pub fn all_by_bundle(&self, bundle: crate::FeatureBundle) -> Vec<&Phoneme> {
        self.phonemes
            .iter()
            .filter(|p| p.features == bundle)
            .collect()
    }

    /// The phoneme holding this IPA string, if any.
    ///
    /// Used before minting, so the engine can never manufacture `duplicate_ipa` —
    /// an Error — by giving a new phoneme a glyph the language already writes.
    ///
    /// Comparison is **byte-exact**, not canonical-equivalence: an author glyph
    /// saved decomposed (NFD `c` + U+0327) will not match the reference table's
    /// precomposed `ç`, so in that corner a mint can slip past the guard and
    /// leave two indistinguishably rendered glyphs in one inventory. Closing it
    /// wants Unicode normalization data and belongs to M7's plausibility profile
    /// as an `ipa_not_nfc` warning on authored inventories — report at the file
    /// boundary once, not normalize in a hot comparison forever.
    pub fn by_ipa(&self, ipa: &str) -> Option<&Phoneme> {
        self.phonemes.iter().find(|p| p.ipa == ipa)
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
            if phoneme.frequency_weight == 0 {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "bad_weight",
                        "a weight of 0 makes this phoneme unselectable; remove it from the \
                         inventory instead if that is what you mean",
                    )
                    .about(&phoneme.id),
                );
            }

            self.check_features(phoneme, &mut report);

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

        self.check_identical_bundles(&mut report);
        self.check_weight_sums(&mut report);

        report
    }
}

impl PhonemeInventory {
    /// Feature checks for one phoneme.
    ///
    /// A wholly featureless phoneme reports **once**, not twelve times: an M0-era
    /// file has no features anywhere, and burying the author in a wall of
    /// `missing_required_feature` would obscure the single thing they need to know.
    fn check_features(&self, phoneme: &Phoneme, report: &mut ValidationReport) {
        if phoneme.features.is_empty() {
            // Error since M3, keeping the promise recorded here and in
            // `PROGRESS.md` at M1: "this becomes an Error in M3, when a rule
            // engine exists that can do nothing with such a segment." That engine
            // now exists and gates on this. Pre-M1 files still *load* —
            // `CLAUDE.md`'s constraint is about loading — and still generate,
            // because `generation_blocking` filters this code for the paths that
            // never read features.
            report.push(
                Issue::new(
                    Severity::Error,
                    "features_unspecified",
                    "this phoneme has no phonological features, so no sound-change rule \
                     can ever match it (`DESIGN.md` §7.1)",
                )
                .about(&phoneme.id),
            );
            return;
        }

        for feature in required_features_missing(phoneme.features) {
            report.push(
                Issue::new(
                    Severity::Error,
                    "missing_required_feature",
                    format!(
                        "`{}` is unvalued, but it is {}; leaving it out would make \
                         \"not specified\" indistinguishable from \"the author forgot\"",
                        feature.name(),
                        requirement_reason(feature, phoneme.features)
                    ),
                )
                .about(&phoneme.id),
            );
        }

        // Rounding IS a labial gesture — that is the stated reason rounded vowels
        // are `[+labial]` and the reason `[+labial]` and `[+dorsal]` both require a
        // `round` value. A segment claiming `[+round -labial]` therefore contradicts
        // the model: a labialised velar /kʷ/ authored that way would validate clean
        // and then escape every `[+labial]` rule in M3, silently.
        if phoneme.features.is(Feature::Round, crate::Sign::Plus)
            && phoneme.features.is(Feature::Labial, crate::Sign::Minus)
        {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "round_without_labial",
                    "this phoneme is [+round] but [-labial]; rounding is a labial gesture, so \
                     it would escape every [+labial] rule (a labialised /kʷ/ is \
                     [+labial +dorsal +round])",
                )
                .about(&phoneme.id),
            );
        }

        // Deliberately asymmetric. The reverse — `kind: consonant` with
        // `[+syllabic]` — is a real and common thing (syllabic nasals and liquids)
        // and is not flagged.
        if phoneme.kind.is_nucleus() && phoneme.features.is(Feature::Syllabic, crate::Sign::Minus) {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "nucleus_not_syllabic",
                    "this phoneme fills vowel slots but is [-syllabic]; one of the two is \
                     probably wrong",
                )
                .about(&phoneme.id),
            );
        }
    }

    /// Two segments with byte-identical non-empty bundles cannot be told apart by
    /// any rule M3 will write.
    ///
    /// Warning rather than Error because length, tone and phonation are not
    /// modelled yet, so /a/ and /aː/ would legitimately collide today. It wants to
    /// become an Error once they exist.
    fn check_identical_bundles(&self, report: &mut ValidationReport) {
        // Collect and sort rather than iterating a grouping map — report order must
        // not vary between runs. Same discipline as `sorted_duplicates`.
        //
        // Each pair is normalised *within itself* before the collection is sorted.
        // Sorting only the outer list is not enough: the same two phonemes in the
        // opposite authored order would yield `(b, a)` instead of `(a, b)`, so the
        // report would still depend on inventory order. A collision is symmetric,
        // so its rendering must be too.
        let mut collisions: Vec<(&str, &str)> = Vec::new();
        for (i, a) in self.phonemes.iter().enumerate() {
            if a.features.is_empty() {
                continue;
            }
            for b in &self.phonemes[i + 1..] {
                if a.features == b.features {
                    let (first, second) = if a.id <= b.id {
                        (a.id.as_str(), b.id.as_str())
                    } else {
                        (b.id.as_str(), a.id.as_str())
                    };
                    collisions.push((first, second));
                }
            }
        }
        collisions.sort_unstable();

        for (a, b) in collisions {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "identical_features",
                    format!(
                        "`{a}` and `{b}` have identical feature bundles, so no sound-change \
                         rule can distinguish them"
                    ),
                )
                .about(format!("{a}, {b}")),
            );
        }
    }

    /// The weights of one slot class must fit in a `u32`.
    ///
    /// Caught here so `stemma validate` cannot say a language is fine while
    /// `generate-roots` fails on it — a validator that disagrees with the engine is
    /// worse than no validator.
    fn check_weight_sums(&self, report: &mut ValidationReport) {
        for (kind, label) in [
            (SegmentKind::Consonant, "consonant"),
            (SegmentKind::Vowel, "vowel"),
        ] {
            let total = self
                .of_kind(kind)
                .try_fold(0u32, |acc, p| acc.checked_add(p.frequency_weight));
            if total.is_none() {
                report.error(
                    "weight_sum_overflow",
                    format!(
                        "the {label} frequency weights sum past u32::MAX, which the weighted \
                         sampler cannot represent; scale them down"
                    ),
                );
            }
        }
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

    /// Fully featured since M3: `features_unspecified` is an Error now, and this
    /// helper serves tests whose point is that a *sound* inventory is clean.
    /// Bundles are the reference fixture's own values.
    fn tiny_inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            featured(
                "ph_p",
                "p",
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
                    "+labial",
                    "-coronal",
                    "-dorsal",
                    "-round",
                ],
            ),
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
            featured(
                "ph_i",
                "i",
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
                    "+high",
                    "-low",
                    "-back",
                    "-round",
                ],
            ),
            featured(
                "ph_u",
                "u",
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
                    "+labial",
                    "-coronal",
                    "+dorsal",
                    "+high",
                    "-low",
                    "+back",
                    "+round",
                ],
            ),
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
        // Every consonant carries the same full bundle: that trips the
        // `identical_features` *Warning*, which is exactly the point — a
        // speculative design may be odd in several ways at once and still valid.
        let mut phonemes: Vec<_> = (0..60)
            .map(|n| {
                featured(
                    PhonemeId::sequential(n).as_str(),
                    &format!("c{n}"),
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
                )
            })
            .collect();
        phonemes.push(full_a());

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
            Phoneme::new("ph_p", "p", SegmentKind::Consonant).with_weight(0),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
        ]);
        let report = inventory.validate();
        assert!(report.errors().any(|i| i.code == "bad_weight"), "{report}");
    }

    // --- M1: features, candidates, and the new checks ---

    fn featured(id: &str, ipa: &str, kind: SegmentKind, tokens: &[&str]) -> Phoneme {
        let bundle = crate::FeatureBundle::try_from(
            tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
        .expect("valid feature list");
        Phoneme::new(id, ipa, kind).with_features(bundle)
    }

    /// A minimal but fully-specified /t/, for tests that need one valid segment.
    fn full_t() -> Phoneme {
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
        )
    }

    fn full_a() -> Phoneme {
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
        )
    }

    #[test]
    fn a_fully_specified_inventory_reports_no_feature_issues() {
        let report = PhonemeInventory::from_phonemes([full_t(), full_a()]).validate();
        assert!(
            !report.issues.iter().any(|i| i.code.contains("feature")),
            "{report}"
        );
    }

    #[test]
    fn a_phoneme_with_no_features_is_an_error_once_a_rule_engine_exists() {
        // The escalation promised at M1 ("this becomes an Error in M3") landing.
        // The file still LOADS — `CLAUDE.md`'s constraint is about loading — and
        // still generates, via `generation_blocking`; but `validate` now calls it
        // what it is: a segment no rule can ever match.
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            full_a(),
        ]);
        let report = inventory.validate();
        assert!(
            report.errors().any(|i| i.code == "features_unspecified"),
            "{report}"
        );
    }

    /// One clear message beats twelve. Guards the early return in `check_features`.
    #[test]
    fn a_phoneme_with_no_features_reports_once_not_twelve_times() {
        let inventory =
            PhonemeInventory::from_phonemes([Phoneme::new("ph_t", "t", SegmentKind::Consonant)]);
        let report = inventory.validate();
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|i| i.code == "features_unspecified")
                .count(),
            1,
            "one clear message, not twelve: {report}"
        );
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|i| i.code == "missing_required_feature")
                .count(),
            0,
            "{report}"
        );
    }

    #[test]
    fn a_phoneme_missing_a_required_feature_is_an_error() {
        // Everything but `voice`.
        let partial = featured(
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
                "-labial",
                "+coronal",
                "-dorsal",
            ],
        );
        let report = PhonemeInventory::from_phonemes([partial, full_a()]).validate();
        let issue = report
            .errors()
            .find(|i| i.code == "missing_required_feature")
            .unwrap_or_else(|| panic!("expected the error: {report}"));
        assert!(issue.message.contains("voice"), "{}", issue.message);
    }

    /// A `[+dorsal]` segment must answer height, backness and rounding.
    #[test]
    fn a_dorsal_segment_missing_height_is_an_error() {
        let mut k = full_t();
        k.id = "ph_k".into();
        k.ipa = "k".into();
        k.features.set(crate::Feature::Coronal, crate::Sign::Minus);
        k.features.set(crate::Feature::Dorsal, crate::Sign::Plus);
        let report = PhonemeInventory::from_phonemes([k, full_a()]).validate();
        assert!(
            report
                .errors()
                .any(|i| i.code == "missing_required_feature" && i.message.contains("high")),
            "{report}"
        );
    }

    /// A plain alveolar legitimately has no rounding value; that must not error.
    #[test]
    fn a_non_dorsal_non_labial_segment_needs_no_rounding_value() {
        let report = PhonemeInventory::from_phonemes([full_t(), full_a()]).validate();
        assert!(report.is_ok(), "{report}");
    }

    #[test]
    fn two_phonemes_with_identical_features_warn_but_stay_valid() {
        let mut twin = full_t();
        twin.id = "ph_t2".into();
        twin.ipa = "t̪".into();
        let report = PhonemeInventory::from_phonemes([full_t(), twin, full_a()]).validate();
        assert!(report.is_ok(), "{report}");
        assert!(
            report.warnings().any(|i| i.code == "identical_features"),
            "{report}"
        );
    }

    #[test]
    fn identical_feature_reports_are_ordered_deterministically() {
        let mut twin = full_t();
        twin.id = "ph_t2".into();
        twin.ipa = "t̪".into();

        let forward = PhonemeInventory::from_phonemes([full_t(), twin.clone(), full_a()]);
        let backward = PhonemeInventory::from_phonemes([twin, full_t(), full_a()]);

        let codes = |inv: &PhonemeInventory| -> Vec<String> {
            inv.validate()
                .issues
                .iter()
                .filter(|i| i.code == "identical_features")
                .filter_map(|i| i.subject.clone())
                .collect()
        };
        assert_eq!(codes(&forward), codes(&backward));
    }

    #[test]
    fn featureless_phonemes_do_not_count_as_identical_to_each_other() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            full_a(),
        ]);
        let report = inventory.validate();
        assert!(
            !report.issues.iter().any(|i| i.code == "identical_features"),
            "empty bundles are a separate, already-reported problem: {report}"
        );
    }

    #[test]
    fn a_vowel_slot_segment_that_is_not_syllabic_warns() {
        let mut odd = full_t();
        odd.kind = SegmentKind::Vowel;
        let report = PhonemeInventory::from_phonemes([odd]).validate();
        assert!(
            report.warnings().any(|i| i.code == "nucleus_not_syllabic"),
            "{report}"
        );
    }

    /// Syllabic nasals are real; the reverse asymmetry must not fire.
    #[test]
    fn a_consonant_slot_segment_that_is_syllabic_is_not_flagged() {
        let mut syllabic_nasal = full_t();
        syllabic_nasal
            .features
            .set(crate::Feature::Syllabic, crate::Sign::Plus);
        let report = PhonemeInventory::from_phonemes([syllabic_nasal, full_a()]).validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "nucleus_not_syllabic"),
            "{report}"
        );
    }

    /// Rounding is a labial gesture. A `[+round -labial]` segment would validate
    /// clean and then escape every `[+labial]` rule in M3 — a labialised /kʷ/
    /// authored that way would silently not be labial.
    #[test]
    fn a_round_segment_that_is_not_labial_is_flagged() {
        let mut labialised_velar = full_t();
        labialised_velar.id = "ph_kw".into();
        labialised_velar.ipa = "kʷ".into();
        for (feature, sign) in [
            (crate::Feature::Coronal, crate::Sign::Minus),
            (crate::Feature::Dorsal, crate::Sign::Plus),
            (crate::Feature::High, crate::Sign::Plus),
            (crate::Feature::Low, crate::Sign::Minus),
            (crate::Feature::Back, crate::Sign::Plus),
            (crate::Feature::Round, crate::Sign::Plus),
        ] {
            labialised_velar.features.set(feature, sign);
        }
        // `labial` is still Minus, inherited from /t/ — the mistake being caught.
        let report = PhonemeInventory::from_phonemes([labialised_velar, full_a()]).validate();
        assert!(
            report.warnings().any(|i| i.code == "round_without_labial"),
            "{report}"
        );
    }

    #[test]
    fn a_properly_labialised_segment_is_not_flagged() {
        let report = PhonemeInventory::from_phonemes([full_t(), full_a()]).validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "round_without_labial"),
            "{report}"
        );
    }

    #[test]
    fn weights_summing_past_u32_max_are_an_error_before_generation() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant).with_weight(u32::MAX),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant).with_weight(2),
            full_a(),
        ]);
        let report = inventory.validate();
        assert!(
            report.errors().any(|i| i.code == "weight_sum_overflow"),
            "{report}"
        );
    }

    #[test]
    fn candidates_return_ids_and_weights_in_authored_order() {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_p", "p", SegmentKind::Consonant).with_weight(30),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_weight(60),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant).with_weight(50),
        ]);
        let (ids, weights) = inventory.candidates(SegmentKind::Consonant);
        assert_eq!(
            ids,
            [PhonemeId::new("ph_p"), PhonemeId::new("ph_t")],
            "authored order, not sorted"
        );
        assert_eq!(weights, [30, 50]);

        let (vowel_ids, vowel_weights) = inventory.candidates(SegmentKind::Vowel);
        assert_eq!(vowel_ids, [PhonemeId::new("ph_a")]);
        assert_eq!(vowel_weights, [60]);
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
