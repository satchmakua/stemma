//! The ROADMAP M4 acceptance tests, over the real fixtures and the real binary.
//!
//! Lives here for the same reason `m3_acceptance.rs` does: loading a fixture
//! needs `stem_io` + `stem_genome`, which sit above the engine crates.
//!
//! ROADMAP M4: "fork Proto-Asterian into Coastal, Highland, and Riverine; apply
//! a different rule history to each; all three lexicons differ; every cognate
//! set is present in all three; a trace walks unbroken from the modern form to
//! the proto-form." The parent is `asterian_attested.ron` — the proto stage
//! that *has* words (`proto_asterian.ron` has none), and M3's own acceptance
//! genome. That deviation is recorded in PROGRESS.md and the fixture headers.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::{Validate, WordId};
use stem_genome::{LanguageGenome, LineageGraph};
use stem_soundchange::RuleSet;

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn attested() -> LanguageGenome {
    stem_io::load(fixture("asterian_attested.ron")).expect("the attested fixture loads")
}

fn rules_of(name: &str) -> RuleSet {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

/// Evolve the attested proto under a daughter's rule set. This is exactly what
/// `stemma fork --rules` does (the CLI routes both verbs through one helper).
fn daughter(rules_file: &str, id: &str, name: &str, years: i32) -> LanguageGenome {
    let (genome, _report) = attested()
        .evolve(id, name, &rules_of(rules_file), years)
        .unwrap_or_else(|e| panic!("{id} evolves: {e:?}"));
    genome
}

fn coastal() -> LanguageGenome {
    daughter("rules_coastal.ron", "coastal", "Coastal Asterian", 470)
}
fn highland() -> LanguageGenome {
    daughter("rules_highland.ron", "highland", "Highland Asterian", 460)
}
fn riverine() -> LanguageGenome {
    daughter("rules_riverine.ron", "riverine", "Riverine Asterian", 420)
}

fn form_of(genome: &LanguageGenome, word: &str) -> String {
    genome
        .lexicon
        .require(&WordId::new(word))
        .expect("word exists")
        .written(&genome.phonemes)
        .expect("renders")
}

/// Every word's romanised form, in id order.
fn forms(genome: &LanguageGenome) -> Vec<String> {
    (1..=9)
        .map(|i| form_of(genome, &format!("w_{i:04}")))
        .collect()
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

// ------------------------------------------------------ the ROADMAP criteria

#[test]
fn forking_the_attested_fixture_three_ways_yields_three_valid_sisters() {
    for genome in [coastal(), highland(), riverine()] {
        let report = genome.validate();
        assert!(report.is_ok(), "{} must validate: {report}", genome.id);
        assert_eq!(
            genome.parent.as_ref().map(|p| p.as_str()),
            Some("asterian_attested")
        );
        assert_eq!(genome.lexicon.len(), 9);
    }
}

#[test]
fn the_three_rule_histories_produce_three_pairwise_different_lexicons() {
    let (c, h, r) = (forms(&coastal()), forms(&highland()), forms(&riverine()));
    assert_ne!(c, h, "Coastal and Highland must differ");
    assert_ne!(c, r, "Coastal and Riverine must differ");
    assert_ne!(h, r, "Highland and Riverine must differ");
}

/// **ROADMAP M4, the headline.** The full 24-cell golden table, hand-computed in
/// M4-SPEC §6 and verified against the engine before it was written here. STAR
/// alone separates all three daughters: `taal` / `tagal` / `tala`.
#[test]
fn takala_reflects_as_taal_tagal_and_tala() {
    // (word, coastal, highland, riverine)
    let table = [
        ("w_0001", "taal", "tagal", "tala"),
        ("w_0002", "ta", "tag", "ta"),
        ("w_0003", "sawel", "sawel", "sawel"),
        ("w_0004", "akw", "akw", "akwa"),
        ("w_0005", "rean", "regan", "rean"),
        ("w_0006", "sank", "saŋk", "saŋka"),
        ("w_0007", "anp", "amp", "ampa"),
        ("w_0008", "amt", "ant", "anta"),
        // MOTHER *mikala (added at M5): /i/ is [-low], so Riverine's coalescence
        // cannot fire and it alone keeps all three vowels — verified against the
        // engine, not hand-computed into these goldens.
        ("w_0009", "mial", "migal", "miala"),
    ];
    let (c, h, r) = (coastal(), highland(), riverine());
    for (word, ec, eh, er) in table {
        assert_eq!(form_of(&c, word), ec, "Coastal {word}");
        assert_eq!(form_of(&h, word), eh, "Highland {word}");
        assert_eq!(form_of(&r, word), er, "Riverine {word}");
    }
}

#[test]
fn riverine_keeps_the_final_vowels_its_sisters_lose() {
    let (c, r) = (coastal(), riverine());
    // WATER, STONE, MOUNTAIN, FIRE all keep their final vowel in Riverine and
    // lose it in Coastal.
    for word in ["w_0004", "w_0006", "w_0007", "w_0008"] {
        let rf = form_of(&r, word);
        let cf = form_of(&c, word);
        assert!(
            rf.ends_with('a'),
            "Riverine {word} = {rf} should keep its final vowel"
        );
        assert!(
            !cf.ends_with('a'),
            "Coastal {word} = {cf} should have lost its final vowel"
        );
    }
}

#[test]
fn highland_keeps_the_velars_coastal_destroys() {
    // STAR: Highland tagal keeps the (voiced) velar as a stop; Coastal taal has
    // spirantised it and then lost it entirely.
    assert_eq!(form_of(&highland(), "w_0001"), "tagal");
    assert_eq!(form_of(&coastal(), "w_0001"), "taal");
}

#[test]
fn the_nasal_isogloss_crosses_the_voicing_isogloss() {
    // Nasal place assimilation is a Highland+Riverine isogloss; voicing is a
    // Coastal+Highland one. STONE shows the nasal isogloss (saŋk / saŋka carry
    // the minted ŋ; Coastal's sank does not), and the two isoglosses share only
    // Highland — so no single daughter looks like any other across both.
    assert!(form_of(&highland(), "w_0006").contains('ŋ'));
    assert!(form_of(&riverine(), "w_0006").contains('ŋ'));
    assert!(!form_of(&coastal(), "w_0006").contains('ŋ'));
}

/// The product thesis in two rows: identical surfaces, different histories,
/// distinguishable only by the trace.
#[test]
fn convergent_rean_and_ta_keep_two_distinct_derivations() {
    let (c, r) = (coastal(), riverine());
    for word in ["w_0002", "w_0005"] {
        assert_eq!(
            form_of(&c, word),
            form_of(&r, word),
            "{word} converges on one surface"
        );
        let c_steps = &c.lexicon.require(&WordId::new(word)).unwrap().trace;
        let r_steps = &r.lexicon.require(&WordId::new(word)).unwrap().trace;
        assert_ne!(
            c_steps, r_steps,
            "{word}: the same surface must carry different derivations"
        );
    }
}

/// **ROADMAP M4:** every cognate set present in all three daughters.
#[test]
fn every_proto_cognate_set_survives_into_every_daughter() {
    let proto = attested();
    let daughters = [coastal(), highland(), riverine()];
    for entry in proto.lexicon.iter() {
        for d in &daughters {
            assert!(
                d.lexicon.by_cognate_set(&entry.cognate_set).is_some(),
                "cognate set {} missing from {}",
                entry.cognate_set,
                d.id
            );
        }
    }
    // And the family coverage agrees: 9/9 universal.
    let graph = LineageGraph::assemble(vec![proto, coastal(), highland(), riverine()]);
    let coverage = graph.cognate_coverage();
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0].universal, 9);
    assert_eq!(coverage[0].sets, 9);
    assert!(coverage[0].gaps.is_empty(), "{:?}", coverage[0].gaps);
}

#[test]
fn sisters_that_innovate_the_same_segment_agree_on_its_id() {
    // Coastal and Highland both innovate /ɡ/ by intervocalic voicing; the M3
    // reference table guarantees both call it `ph_g`.
    let g = stem_core::PhonemeId::new("ph_g");
    assert!(coastal().phonemes.get(&g).is_some(), "Coastal has ph_g");
    assert!(highland().phonemes.get(&g).is_some(), "Highland has ph_g");
    assert_eq!(
        coastal().phonemes.get(&g).unwrap().ipa,
        highland().phonemes.get(&g).unwrap().ipa,
        "both must spell /ɡ/ the same way"
    );
}

#[test]
fn riverine_coalescence_is_fed_by_velar_loss() {
    // w_0001's trace must show BOTH steps: velar loss (takala -> taala) then
    // low-vowel coalescence (taala -> tala).
    let r = riverine();
    let entry = r.lexicon.require(&WordId::new("w_0001")).unwrap();
    let trace = entry.trace.as_ref().expect("traced");
    let applied: Vec<_> = trace.steps.iter().map(|s| s.rule.as_str()).collect();
    assert!(
        applied.contains(&"r_velar_loss") && applied.contains(&"r_low_coalescence"),
        "both feeding steps must be recorded: {applied:?}"
    );
    assert_eq!(form_of(&r, "w_0001"), "tala");
}

#[test]
fn every_daughters_trace_replays_from_the_proto_form_to_its_stored_form() {
    for genome in [coastal(), highland(), riverine()] {
        for entry in genome.lexicon.iter() {
            let trace = entry.trace.as_ref().expect("evolved words carry a trace");
            // §16.3's property: the trace replays to the stored form.
            let replayed = trace.final_form();
            assert_eq!(
                replayed, entry.phonemic_form,
                "{} {}: trace must replay to the stored form",
                genome.id, entry.id
            );
            // And its input is the proto-form as the run received it: prosody is
            // assigned once at the start of a word's life (apply.rs), so the
            // stored `input` is the proto form with stress marked — compare
            // against exactly that, not the raw un-stressed fixture form.
            let proto = attested();
            let mut proto_form = proto
                .lexicon
                .require(&entry.id)
                .unwrap()
                .phonemic_form
                .clone();
            proto.prosody.assign(&mut proto_form);
            assert_eq!(
                trace.input, proto_form,
                "{} {}: the derivation begins at the proto-form",
                genome.id, entry.id
            );
        }
    }
}

/// **ROADMAP M4:** a trace walks unbroken from the modern form to the proto.
/// Asserted on the ledger's proto input line, the final form, and the four rule
/// blocks in order — never on an arrow-chain format the renderer does not emit.
#[test]
fn a_trace_walks_unbroken_from_modern_form_to_proto_form() {
    // Build coastal on disk so `stemma trace` can read it.
    let dir = std::env::temp_dir().join("stemma_m4_trace");
    std::fs::create_dir_all(&dir).unwrap();
    let coastal_path = dir.join("coastal.ron");
    let out = stemma(&[
        "fork",
        fixture("asterian_attested.ron").to_str().unwrap(),
        "--rules",
        fixture("rules_coastal.ron").to_str().unwrap(),
        "--id",
        "coastal",
        "--name",
        "Coastal Asterian",
        "--years",
        "470",
        "--out",
        coastal_path.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "fork failed: {out:?}");

    let out = stemma(&["trace", coastal_path.to_str().unwrap(), "w_0001"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("takala"), "proto input line: {text}");
    assert!(text.contains("taal"), "modern form: {text}");
    for rule in ["r_ivv", "r_velar_lenition", "r_gamma_loss", "r_apocope"] {
        assert!(text.contains(rule), "missing rule block {rule}: {text}");
    }
    // The four rule blocks appear in chronological order.
    let idx = |r: &str| text.find(r).unwrap();
    assert!(
        idx("r_ivv") < idx("r_velar_lenition")
            && idx("r_velar_lenition") < idx("r_gamma_loss")
            && idx("r_gamma_loss") < idx("r_apocope"),
        "rule blocks must be in order: {text}"
    );
}

/// CLAUDE.md's determinism rule as a test: forking all three daughters twice
/// must produce byte-identical serialized genomes.
#[test]
fn the_fork_pipeline_is_byte_identical_across_two_runs() {
    let run = || {
        [
            stem_io::to_string(&coastal(), stem_io::Format::Ron).unwrap(),
            stem_io::to_string(&highland(), stem_io::Format::Ron).unwrap(),
            stem_io::to_string(&riverine(), stem_io::Format::Ron).unwrap(),
        ]
    };
    assert_eq!(
        run(),
        run(),
        "two runs must serialize identically, byte for byte"
    );
}

/// The family rendering is snapshot-pinned (tree + coverage only, no report),
/// because M6's demo scripts against exactly this text.
#[test]
fn the_family_rendering_is_stable() {
    let graph = LineageGraph::assemble(vec![attested(), coastal(), highland(), riverine()]);
    let rendered = stem_genome::render_family(&graph);
    let expected = "\
Attested Asterian (asterian_attested) — proto · 15 phonemes · 9 words · 0 rules
├─ Coastal Asterian (coastal) — +470y · 17 phonemes · 9 words · 4 rules
├─ Highland Asterian (highland) — +460y · 17 phonemes · 9 words · 3 rules
└─ Riverine Asterian (riverine) — +420y · 16 phonemes · 9 words · 3 rules

cognate coverage — asterian_attested: 9 sets, 3 descendants, 9/9 present in all
";
    assert_eq!(rendered, expected, "the family rendering drifted");
}

#[test]
fn the_family_report_carries_only_the_expected_stale_pattern_notes() {
    let graph = LineageGraph::assemble(vec![attested(), coastal(), highland(), riverine()]);
    let report = graph.validate_family();
    assert!(report.is_ok(), "the acceptance family is valid: {report}");
    // No family-level codes fire; every issue is a daughter's stale-pattern Note.
    for issue in &report.issues {
        assert!(
            issue.code.ends_with(".lexicon.syllable_shape_mismatch"),
            "unexpected issue in the acceptance family: {issue}"
        );
    }
    assert!(
        report
            .issues
            .iter()
            .all(|i| i.severity == stem_core::Severity::Note),
        "every acceptance-family issue is a Note: {report}"
    );
}

#[test]
fn the_daughter_rule_fixtures_validate_against_the_attested_language() {
    let proto = attested();
    for file in [
        "rules_coastal.ron",
        "rules_highland.ron",
        "rules_riverine.ron",
    ] {
        let rules = rules_of(file);
        let report = stem_soundchange::check_against_language(
            &rules.rules,
            &proto.phonemes,
            &proto.prosody,
            &proto.lexicon,
        );
        // Nothing blocks. Coastal's two fed rules warn `target_matches_nothing`
        // (their classes are empty until an earlier rule mints into them); that
        // is the M3 feeding pattern and is not an error.
        assert!(report.is_ok(), "{file}: {report}");
    }
}

#[test]
fn family_reports_a_dangling_parent_when_the_proto_is_not_passed() {
    // Only the daughters, not the proto they name.
    let graph = LineageGraph::assemble(vec![coastal(), highland(), riverine()]);
    let report = graph.validate_family();
    assert!(
        report
            .warnings()
            .any(|i| i.code == "family.dangling_parent"),
        "a family missing its root should warn, not error: {report}"
    );
    assert!(
        report.is_ok(),
        "a dangling parent is a Warning, not an Error: {report}"
    );
}

#[test]
fn the_fork_and_family_subcommands_are_wired_up() {
    let out = stemma(&["fork", "--help"]);
    assert!(out.status.success());
    let out = stemma(&["family", "--help"]);
    assert!(out.status.success());
}
