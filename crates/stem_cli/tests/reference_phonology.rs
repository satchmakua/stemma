//! The reference language's phonology, pinned.
//!
//! These tests live here rather than in `stem_phonology` because loading a fixture
//! needs `stem_io` and `stem_genome`, which sit *above* `stem_phonology` in the
//! crate graph. `stem_cli` is the only crate that depends on all three, so it is
//! where a test that crosses every layer belongs.
//!
//! The natural-class tests at the bottom are the important ones: they discharge
//! M1's forward-compatibility claim toward M3 **today**, against the real fixture,
//! without shipping a rule engine. If `[-sonorant, -continuant, -voice]` does not
//! pick out exactly `{p, t, k}`, the feature model has failed at the one job M3
//! needs from it, and no amount of later engineering recovers that.

use std::path::{Path, PathBuf};

use stem_core::Validate;
use stem_genome::LanguageGenome;
use stem_phonology::{Feature, FeatureBundle, PhonemeInventory, Sign};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn asterian() -> LanguageGenome {
    stem_io::load(fixture("proto_asterian.ron")).expect("the reference fixture must load")
}

/// The set of IPA symbols whose bundles satisfy `pattern`, in inventory order.
fn matching(inventory: &PhonemeInventory, pattern: &[&str]) -> Vec<String> {
    let pattern =
        FeatureBundle::try_from(pattern.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid pattern");
    inventory
        .iter()
        .filter(|p| p.features.subsumes(pattern))
        .map(|p| p.ipa.clone())
        .collect()
}

/// The reference fixture is the worked example, so its phonology must stay
/// pristine. Since M2 it also carries one Note — it is a proto *definition*, the
/// thing `new-lexicon` is run against, so it legitimately has no lexicon yet.
/// Asserted exactly rather than loosely, so a real issue cannot hide behind it.
#[test]
fn the_reference_language_reports_nothing_but_its_missing_lexicon() {
    let report = asterian().validate();
    let codes: Vec<&str> = report.issues.iter().map(|i| i.code.as_str()).collect();
    assert_eq!(
        codes,
        ["lexicon.empty"],
        "the fixture must stay pristine apart from having no lexicon:\n{report}"
    );
    assert!(report.is_ok(), "{report}");
}

/// Golden. Every segment's full resolved matrix, in canonical order.
#[test]
fn the_reference_inventory_resolves_to_a_frozen_feature_matrix() {
    let genome = asterian();
    let rendered: Vec<String> = genome
        .phonemes
        .iter()
        .map(|p| format!("{} = {}", p.ipa, p.features.render()))
        .collect();

    let expected = [
        "p = -syllabic +consonantal -sonorant -approximant -continuant -nasal -lateral -trill -voice +labial -coronal -dorsal -round",
        "t = -syllabic +consonantal -sonorant -approximant -continuant -nasal -lateral -trill -voice -labial +coronal -dorsal",
        "k = -syllabic +consonantal -sonorant -approximant -continuant -nasal -lateral -trill -voice -labial -coronal +dorsal +high -low +back -round",
        "m = -syllabic +consonantal +sonorant -approximant -continuant +nasal -lateral -trill +voice +labial -coronal -dorsal -round",
        "n = -syllabic +consonantal +sonorant -approximant -continuant +nasal -lateral -trill +voice -labial +coronal -dorsal",
        "s = -syllabic +consonantal -sonorant -approximant +continuant -nasal -lateral -trill -voice -labial +coronal -dorsal",
        "l = -syllabic +consonantal +sonorant +approximant +continuant -nasal +lateral -trill +voice -labial +coronal -dorsal",
        "r = -syllabic +consonantal +sonorant -approximant +continuant -nasal -lateral +trill +voice -labial +coronal -dorsal",
        "w = -syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice +labial -coronal +dorsal +high -low +back +round",
        "j = -syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice -labial -coronal +dorsal +high -low -back -round",
        "a = +syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice -labial -coronal +dorsal -high +low +back -round",
        "e = +syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice -labial -coronal +dorsal -high -low -back -round",
        "i = +syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice -labial -coronal +dorsal +high -low -back -round",
        "o = +syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice +labial -coronal +dorsal -high -low +back +round",
        "u = +syllabic -consonantal +sonorant +approximant +continuant -nasal -lateral -trill +voice +labial -coronal +dorsal +high -low +back +round",
    ];
    assert_eq!(rendered, expected);
}

#[test]
fn all_fifteen_reference_bundles_are_pairwise_distinct() {
    let genome = asterian();
    let phonemes: Vec<_> = genome.phonemes.iter().collect();
    for (i, a) in phonemes.iter().enumerate() {
        for b in &phonemes[i + 1..] {
            assert_ne!(
                a.features, b.features,
                "/{}/ and /{}/ are featurally identical, so no rule could tell them apart",
                a.ipa, b.ipa
            );
        }
    }
}

#[test]
fn every_reference_phoneme_values_every_universally_required_feature() {
    let genome = asterian();
    let required = [
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
    for phoneme in genome.phonemes.iter() {
        for feature in required {
            assert!(
                phoneme.features.is_specified(feature),
                "/{}/ does not value `{}`",
                phoneme.ipa,
                feature.name()
            );
        }
    }
}

/// A plain alveolar genuinely has no rounding value. This asserts that absence is
/// *used*, not merely permitted — if every segment valued everything, the
/// three-valued storage would be dead weight and nobody would notice it breaking.
#[test]
fn absence_is_actually_used_by_the_reference_inventory() {
    let genome = asterian();
    let t = genome
        .phonemes
        .iter()
        .find(|p| p.ipa == "t")
        .expect("/t/ is in the inventory");
    assert_eq!(t.features.get(Feature::Round), None);
    assert!(!t.features.is(Feature::Round, Sign::Minus));
}

// --- The M3 forward-compatibility proof (DESIGN.md §11.1) ---

/// `DESIGN.md` §11.1 IntervocalicVoicing: `target: [-sonorant, -continuant, -voice]`.
#[test]
fn features_select_the_intervocalic_voicing_target() {
    let genome = asterian();
    assert_eq!(
        matching(&genome.phonemes, &["-sonorant", "-continuant", "-voice"]),
        ["p", "t", "k"]
    );
}

/// The environment of that same rule: `V _ V`.
#[test]
fn features_select_the_intervocalic_voicing_environment() {
    let genome = asterian();
    assert_eq!(
        matching(&genome.phonemes, &["+syllabic"]),
        ["a", "e", "i", "o", "u"]
    );
}

/// `DESIGN.md` §7.2: "nasals assimilate to the place of a following stop".
#[test]
fn features_select_the_nasal_assimilation_target() {
    let genome = asterian();
    assert_eq!(matching(&genome.phonemes, &["+nasal"]), ["m", "n"]);
}

/// `DESIGN.md` §11.1 VelarPalatalization, and §7.1's "front vowels trigger
/// palatalization of preceding velars".
#[test]
fn features_select_the_velar_palatalization_target_and_environment() {
    let genome = asterian();
    assert_eq!(matching(&genome.phonemes, &["-sonorant", "+dorsal"]), ["k"]);
    assert_eq!(
        matching(&genome.phonemes, &["+syllabic", "-back"]),
        ["e", "i"],
        "/a/ must not be a front vowel, or palatalization fires before it"
    );
}

#[test]
fn the_place_features_partition_the_reference_inventory() {
    let genome = asterian();
    assert_eq!(
        matching(&genome.phonemes, &["+labial"]),
        ["p", "m", "w", "o", "u"]
    );
    assert_eq!(
        matching(&genome.phonemes, &["+coronal"]),
        ["t", "n", "s", "l", "r"]
    );
    assert_eq!(
        matching(&genome.phonemes, &["+dorsal"]),
        ["k", "w", "j", "a", "e", "i", "o", "u"]
    );
}

#[test]
fn features_select_the_obstruents_and_the_liquids() {
    let genome = asterian();
    assert_eq!(
        matching(&genome.phonemes, &["-sonorant"]),
        ["p", "t", "k", "s"]
    );
    assert_eq!(
        matching(
            &genome.phonemes,
            &["+consonantal", "+sonorant", "+continuant"]
        ),
        ["l", "r"]
    );
}

/// The regression guard, named so a future session cannot "fix" it back. /w/ and
/// /j/ fill consonant slots *and* are `[-consonantal]`; both are true.
#[test]
fn a_glide_may_be_a_consonant_slot_and_minus_consonantal() {
    let genome = asterian();
    assert_eq!(
        matching(&genome.phonemes, &["-consonantal", "-syllabic"]),
        ["w", "j"]
    );
    for ipa in ["w", "j"] {
        let glide = genome.phonemes.iter().find(|p| p.ipa == ipa).unwrap();
        assert_eq!(
            glide.kind,
            stem_phonology::SegmentKind::Consonant,
            "/{ipa}/ fills a C slot"
        );
        assert!(
            glide.features.is(Feature::Consonantal, Sign::Minus),
            "/{ipa}/ is phonologically [-consonantal]"
        );
    }
}

/// Glide formation (`i -> j / _V`) and vocalization should be single-feature
/// rules in M3, not an invention of place features out of nowhere.
#[test]
fn a_glide_differs_from_its_vowel_counterpart_in_exactly_one_feature() {
    let genome = asterian();
    let by_ipa = |ipa: &str| {
        genome
            .phonemes
            .iter()
            .find(|p| p.ipa == ipa)
            .unwrap_or_else(|| panic!("/{ipa}/ missing"))
            .features
    };

    for (vowel, glide) in [("i", "j"), ("u", "w")] {
        let (v, g) = (by_ipa(vowel), by_ipa(glide));
        let differing: Vec<&str> = Feature::ALL
            .iter()
            .filter(|&&f| v.get(f) != g.get(f))
            .map(|f| f.name())
            .collect();
        assert_eq!(
            differing,
            ["syllabic"],
            "/{vowel}/ and /{glide}/ should differ only in syllabicity"
        );
    }
}
