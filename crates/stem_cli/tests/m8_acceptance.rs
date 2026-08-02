//! The ROADMAP M8 acceptance tests: **a regular paradigm becomes irregular purely
//! as a consequence of an ordered sound change, and the trace explains why.**
//!
//! The load-bearing test is `an_ordered_sound_change_turns_a_regular_plural_irregular`:
//! it inflects the reference morphology fixture with the real `inflect`, evolves the
//! cells with the real engine (`LanguageGenome::evolve`), and asserts the four
//! surface forms and each cell's trace. Everything else — the measure, the profile
//! band, the CLI wiring — trusts that mechanism.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, MorphologicalIrregularity, render_paradigm};
use stem_lexicon::{
    Lexicon, MorphemeRole, WordEntry, WordSource, inflect, morphological_irregularity,
};
use stem_soundchange::RuleSet;

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn load_genome(name: &str) -> LanguageGenome {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

fn load_rules(name: &str) -> RuleSet {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

/// Inflects the NUMBER paradigm of the reference morphology fixture into a genome —
/// the regular, pre-sound-change stage.
fn inflected_proto() -> LanguageGenome {
    let genome = load_genome("morphology_asterian.ron");
    let paradigm = &genome.morphology.paradigms[0];
    let cells = inflect(paradigm, &genome.morphology.morphemes, &genome.id).expect("inflects");
    genome.clone().with_lexicon(Lexicon::from_entries(cells))
}

/// The plural cell for the stem glossed `stem_gloss` (e.g. `"star"` → `star PL`).
fn plural_of<'a>(genome: &'a LanguageGenome, stem_gloss: &str) -> &'a WordEntry {
    let want = format!("{stem_gloss} PL");
    genome
        .lexicon
        .iter()
        .find(|e| e.glosses.first().map(|g| g == &want).unwrap_or(false))
        .unwrap_or_else(|| panic!("no cell glossed `{want}`"))
}

// ------------------------------------------------- the load-bearing mechanism

/// **ROADMAP M8, the acceptance.** The regular `-ka` plural, after one ordered
/// sound change (intervocalic voicing), surfaces as `-ɡa` on the vowel-final stems
/// and `-ka` on the consonant-final ones — an irregular paradigm produced *purely*
/// by the rule, with each cell's trace saying which rule fired where.
#[test]
fn an_ordered_sound_change_turns_a_regular_plural_irregular() {
    let proto = inflected_proto();

    // Before the rule: every plural is a regular `-ka`.
    for stem in ["star", "moon", "man", "salt"] {
        let form = plural_of(&proto, stem).written(&proto.phonemes).unwrap();
        assert!(
            form.ends_with("ka"),
            "the regular plural of {stem} is -ka, got {form}"
        );
    }

    // Evolve with the ONE-rule voicing set — the real engine, no hand-built trace.
    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, report) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");
    assert!(report.is_ok(), "the evolution is clean: {report}");

    // After the rule: the four surface forms, hand-computed in M8-SPEC §5.
    let surface = |stem: &str| plural_of(&early, stem).written(&early.phonemes).unwrap();
    assert_eq!(surface("star"), "tiraga", "vowel-final: /k/ voiced");
    assert_eq!(surface("moon"), "menaga", "vowel-final: /k/ voiced");
    assert_eq!(surface("man"), "tanka", "consonant-final: /k/ untouched");
    assert_eq!(surface("salt"), "sulka", "consonant-final: /k/ untouched");

    // The trace explains why. tira-PL carries an r_ivv step; tan-PL does not.
    let tira_pl = plural_of(&early, "star");
    let tira_steps = &tira_pl.trace.as_ref().expect("tira-PL evolved").steps;
    assert!(
        tira_steps.iter().any(|s| s.rule.as_str() == "r_ivv"),
        "the vowel-final plural records the voicing that made it irregular"
    );
    let tan_pl = plural_of(&early, "man");
    let tan_steps = &tan_pl
        .trace
        .as_ref()
        .expect("tan-PL still has a trace")
        .steps;
    assert!(
        tan_steps.is_empty(),
        "the consonant-final plural records no step — the rule did not apply"
    );
}

// ------------------------------------------------------------------- the measure

/// The allomorph measure reads the split off the evolved cells: the plural surfaces
/// in two shapes, so it is irregular — but only two, so it is not *extreme*.
#[test]
fn the_measure_reports_two_allomorphs_after_the_change_and_one_before() {
    let proto = inflected_proto();
    let before = morphological_irregularity(&proto.lexicon);
    assert_eq!(before.len(), 1, "one affix: the plural");
    assert_eq!(before[0].count(), 1, "regular before any sound change");

    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, _) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");
    let after = morphological_irregularity(&early.lexicon);
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].count(),
        2,
        "irregular after the change: -ɡa / -ka ({:?})",
        after[0].allomorphs
    );
    assert!(after[0].is_irregular());
}

// -------------------------------------------------------------- the profile band

/// The profile scores the split `Allomorphic` — and, being a two-way alternation,
/// does **not** trip the `high_morphological_irregularity` Note (report, don't
/// police: a single conditioned split is ordinary).
#[test]
fn the_profile_scores_allomorphic_and_stays_below_the_extreme_note() {
    let proto = inflected_proto();
    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, _) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");

    let profile = early.plausibility_profile();
    assert_eq!(
        profile.morphological_irregularity,
        MorphologicalIrregularity::Allomorphic
    );
    let report = early.validate();
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.code == "high_morphological_irregularity"),
        "a two-way split is ordinary, not extreme: {report}"
    );
    assert!(
        report.is_ok(),
        "an irregular paradigm is unusual, not broken: {report}"
    );
}

// ------------------------------------------------------------------- §3.3 & flow

/// §3.3: every inflected cell records its composition, and evolution carries it.
#[test]
fn every_inflected_cell_records_its_morphemes_and_is_derived() {
    let proto = inflected_proto();
    assert_eq!(proto.lexicon.len(), 8, "4 stems × 2 cells");
    for cell in proto.lexicon.iter() {
        assert_eq!(cell.source, WordSource::Derived);
        assert!(
            !cell.morphemes.is_empty(),
            "a composed form must record its composition (§3.3): {:?}",
            cell.id
        );
    }
    // A plural cell decomposes into stem + suffix; the suffix span survives evolution.
    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, _) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");
    let tira_pl = plural_of(&early, "star");
    let suffix = tira_pl
        .morphemes
        .iter()
        .find(|m| m.role == MorphemeRole::Suffix)
        .expect("the plural suffix is recorded");
    let carried = tira_pl
        .trace
        .as_ref()
        .unwrap()
        .surface_of_input_span(suffix.start as usize, suffix.end as usize);
    let rom: String = carried
        .iter()
        .map(|id| early.phonemes.require(id).unwrap().written())
        .collect();
    assert_eq!(
        rom, "ga",
        "the recorded suffix span surfaces as the -ɡa allomorph"
    );
}

// --------------------------------------------------------------- the render + CLI

/// The library renderer shows regular→irregular and names the conditioning rule.
#[test]
fn render_paradigm_shows_the_split_and_names_the_rule() {
    let proto = inflected_proto();
    let paradigm = proto.morphology.paradigms[0].clone();

    let regular = render_paradigm(&proto, &paradigm).expect("renders");
    assert!(
        regular.contains("1 allomorph"),
        "proto is regular:\n{regular}"
    );

    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, _) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");
    let irregular = render_paradigm(&early, &paradigm).expect("renders");
    assert!(irregular.contains("tiraga"), "{irregular}");
    assert!(irregular.contains("tanka"), "{irregular}");
    assert!(
        irregular.contains("2 allomorphs (irregular)"),
        "{irregular}"
    );
    assert!(
        irregular.contains("Intervocalic voicing fired"),
        "names the conditioning rule:\n{irregular}"
    );
    assert!(
        irregular.contains("did not apply here"),
        "marks the cell the rule skipped:\n{irregular}"
    );
}

/// The two verbs are wired up.
#[test]
fn the_inflect_and_paradigm_subcommands_are_wired_up() {
    assert!(stemma(&["inflect", "--help"]).status.success());
    assert!(stemma(&["paradigm", "--help"]).status.success());
}

/// End-to-end through the binary: inflect → apply-rules → paradigm, over real
/// files, reproduces the split. The morphology twin of the M3 `apply-rules → trace`
/// acceptance.
#[test]
fn the_cli_pipeline_inflects_evolves_and_renders_the_irregular_paradigm() {
    // A per-process temp dir keeps parallel test runs from colliding.
    let dir = std::env::temp_dir().join(format!("stemma_m8_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let proto = dir.join("number_proto.ron");
    let early = dir.join("number_early.ron");

    let inflect_out = stemma(&[
        "inflect",
        fixture("morphology_asterian.ron").to_str().unwrap(),
        "--paradigm",
        "NUMBER",
        "--out",
        proto.to_str().unwrap(),
    ]);
    assert!(
        inflect_out.status.success(),
        "inflect failed: {inflect_out:?}"
    );
    let regular = String::from_utf8(inflect_out.stdout).unwrap();
    assert!(regular.contains("1 allomorph"), "regular table:\n{regular}");

    let apply = stemma(&[
        "apply-rules",
        proto.to_str().unwrap(),
        "--rules",
        fixture("rules_intervocalic_voicing.ron").to_str().unwrap(),
        "--id",
        "early",
        "--name",
        "Early",
        "--years",
        "250",
        "--out",
        early.to_str().unwrap(),
    ]);
    assert!(apply.status.success(), "apply-rules failed: {apply:?}");

    let paradigm = stemma(&["paradigm", early.to_str().unwrap(), "--paradigm", "NUMBER"]);
    assert!(paradigm.status.success(), "paradigm failed: {paradigm:?}");
    let text = String::from_utf8(paradigm.stdout).unwrap();
    assert!(text.contains("tiraga") && text.contains("tanka"), "{text}");
    assert!(text.contains("2 allomorphs (irregular)"), "{text}");
    assert!(text.contains("Intervocalic voicing fired"), "{text}");

    let profile = stemma(&["profile", early.to_str().unwrap()]);
    let profile_text = String::from_utf8(profile.stdout).unwrap();
    assert!(
        profile_text.contains("Morphological irregularity") && profile_text.contains("allomorphic"),
        "{profile_text}"
    );

    // `stemma trace` over an inflected cell — the claim PROGRESS.md advertises,
    // pinned. Cell order is stem-major/cell-minor over [tira, mena, tan, sul] ×
    // [SG, PL], so w_0002 is tira-PL (voiced) and w_0006 is tan-PL (untouched).
    let trace_tira = stemma(&["trace", early.to_str().unwrap(), "w_0002"]);
    let tira_text = String::from_utf8(trace_tira.stdout).unwrap();
    assert!(
        tira_text.contains("r_ivv") && tira_text.contains("tiraga"),
        "trace of the voiced plural names the rule that made it irregular:\n{tira_text}"
    );
    let trace_tan = stemma(&["trace", early.to_str().unwrap(), "w_0006"]);
    let tan_text = String::from_utf8(trace_tan.stdout).unwrap();
    assert!(
        tan_text.contains("did not apply"),
        "trace of the consonant-final plural shows the rule did not fire:\n{tan_text}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Finding-2 projection: the allomorph count `render_paradigm` prints is the count
/// `morphological_irregularity` measures — they now read one shared
/// `WordEntry::morpheme_surface`, and this pins that they cannot drift apart.
#[test]
fn the_rendered_allomorph_count_equals_the_measure() {
    let proto = inflected_proto();
    let paradigm = proto.morphology.paradigms[0].clone();
    let rules = load_rules("rules_intervocalic_voicing.ron");
    let (early, _) = proto
        .evolve("early", "Early", &rules, 250)
        .expect("evolves");

    let measured = morphological_irregularity(&early.lexicon)
        .iter()
        .find(|s| s.gloss == "PL")
        .expect("the plural is measured")
        .count();
    let rendered = render_paradigm(&early, &paradigm).expect("renders");
    assert_eq!(measured, 2);
    assert!(
        rendered.contains(&format!("{measured} allomorphs")),
        "the rendered count must equal the measured count ({measured}):\n{rendered}"
    );
}

// --------------------------------------------------------------- serde stability

/// A pre-M8 file — the reference proto — still round-trips byte-identically: the new
/// `morphology` field, being empty, writes no bytes.
#[test]
fn a_pre_m8_language_serialises_without_a_morphology_field() {
    let genome = load_genome("proto_asterian.ron");
    let dir = std::env::temp_dir().join(format!("stemma_m8_serde_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("roundtrip.ron");
    stem_io::save(&path, &genome).expect("saves");
    let text = std::fs::read_to_string(&path).expect("reads back");
    assert!(
        !text.contains("morphology"),
        "an empty morphology must not appear in a pre-M8 file"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
