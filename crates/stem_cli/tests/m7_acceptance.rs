//! The ROADMAP M7 acceptance tests: the §17 plausibility warnings fire on
//! languages that deserve them and stay quiet on Proto-Asterian and its
//! daughters. Loading a fixture needs `stem_io` + `stem_genome`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{BranchSpec, LanguageGenome, grow_family};
use stem_soundchange::RuleSet;

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

/// The plausibility warnings M7 introduced. "Quiet" means none of these fired.
const NEW_M7_CODES: &[&str] = &[
    "phonology.large_consonant_inventory",
    "phonology.large_vowel_inventory",
    "phonotactics.large_consonant_cluster",
    "high_change_density",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn load_genome(name: &str) -> LanguageGenome {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

fn rules(name: &str) -> RuleSet {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

fn fired_codes(genome: &LanguageGenome) -> Vec<String> {
    genome
        .validate()
        .issues
        .iter()
        .map(|i| i.code.clone())
        .filter(|c| NEW_M7_CODES.contains(&c.as_str()))
        .collect()
}

// ------------------------------------------------------ quiet on the reference

/// **ROADMAP M7.** The reference proto trips none of the new plausibility checks.
#[test]
fn the_reference_proto_stays_quiet_on_every_new_plausibility_check() {
    let fired = fired_codes(&load_genome("proto_asterian.ron"));
    assert!(
        fired.is_empty(),
        "Proto-Asterian must stay quiet, saw {fired:?}"
    );
}

/// The whole reference family — extend "quiet on Proto-Asterian" to the daughters
/// built the M6 way (`grow_family` with the real branch years, so
/// `high_change_density` is not self-triggered by an artificially small gap).
#[test]
fn the_asterian_daughters_stay_quiet_on_every_new_plausibility_check() {
    let proto = load_genome("asterian_attested.ron");
    let (coastal, highland, riverine) = (
        rules("rules_coastal.ron"),
        rules("rules_highland.ron"),
        rules("rules_riverine.ron"),
    );
    let branches = [
        BranchSpec {
            id: "coastal",
            name: "Coastal",
            rules: &coastal,
            years: 470,
        },
        BranchSpec {
            id: "highland",
            name: "Highland",
            rules: &highland,
            years: 460,
        },
        BranchSpec {
            id: "riverine",
            name: "Riverine",
            rules: &riverine,
            years: 420,
        },
    ];
    let (graph, _) = grow_family(&proto, &branches).expect("the family grows");
    for daughter in &graph.nodes()[1..] {
        let fired = fired_codes(daughter);
        assert!(
            fired.is_empty(),
            "daughter `{}` must stay quiet, saw {fired:?}",
            daughter.id
        );
    }
}

// ------------------------------------------------------ fires when it should

/// **ROADMAP M7.** A language that deserves a warning gets it — and stays valid.
#[test]
fn the_clusters_fixture_earns_its_cluster_warning_and_still_validates() {
    let genome = load_genome("implausible_clusters.ron");
    let report = genome.validate();
    assert!(
        report.is_ok(),
        "a marked cluster is a Warning, not a rejection (§17): {report}"
    );
    assert!(
        report
            .warnings()
            .any(|i| i.code == "phonotactics.large_consonant_cluster"),
        "the CCCVCC template must earn a cluster warning: {report}"
    );
}

// ------------------------------------------------------ the CLI profile block

/// **ROADMAP M7.** `stemma profile` prints §17's scored block, and it stays quiet
/// for the reference proto.
#[test]
fn stemma_profile_prints_the_scored_block_and_stays_quiet_for_the_proto() {
    let out = stemma(&["profile", fixture("proto_asterian.ron").to_str().unwrap()]);
    assert!(out.status.success(), "profile failed: {out:?}");
    let text = String::from_utf8(out.stdout).unwrap();

    assert!(
        text.contains("Plausibility profile — Proto-Asterian"),
        "{text}"
    );
    assert!(text.contains("Typological rarity"), "{text}");
    assert!(text.contains("typical"), "{text}");
    assert!(text.contains("simple"), "{text}");
    assert!(text.contains("not yet modelled"), "{text}");
    // M8 filled the morphological-irregularity row: it is a scored band now (the
    // proto has no affixation → "none"), and the deferred milestones start at M9.
    assert!(
        text.contains("Morphological irregularity"),
        "morphology is a scored dimension now: {text}"
    );
    assert!(text.contains("M9"), "names a deferred milestone: {text}");
    // Honest: no fabricated percentage, no syntax dimension, no new warning.
    assert!(!text.contains('%'), "no fabricated percentage: {text}");
    for code in NEW_M7_CODES {
        assert!(
            !text.contains(code),
            "proto must be quiet, saw {code}: {text}"
        );
    }
}

/// `stemma profile` on a language that deserves a warning shows both the elevated
/// band and the warning.
#[test]
fn stemma_profile_shows_a_complex_band_and_the_cluster_warning() {
    let out = stemma(&[
        "profile",
        fixture("implausible_clusters.ron").to_str().unwrap(),
    ]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("complex"), "the band is Complex:\n{text}");
    assert!(
        text.contains("large_consonant_cluster"),
        "the warning is shown:\n{text}"
    );
}

#[test]
fn the_profile_subcommand_is_wired_up() {
    assert!(stemma(&["profile", "--help"]).status.success());
}
