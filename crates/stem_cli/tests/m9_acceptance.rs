//! The ROADMAP M9 acceptance tests: **reproduce the design's worked example —
//! `*takala` "star" becomes "omen" in Coastal while staying "star" in Highland.**
//!
//! The load-bearing property is not that a gloss changed. It is that the gloss
//! changed *and the cognate set did not*, so the two reflexes stay one row in the
//! comparative table. Meaning diverges; ancestry does not. Every test here is a way
//! that separation could break.
//!
//! Everything runs the real pipeline — the real `evolve`, the real `apply_drift`,
//! the real fixtures. Nothing is hand-authored into place.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, SemanticDrift, render_word_history};
use stem_lexicon::{DriftSet, LONG_SENSE_CHAIN, WordEntry, sense_chains};
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

fn load_drift(name: &str) -> DriftSet {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

/// The reference family, built the real way: proto → (Coastal, Highland) by sound
/// change, then Coastal's meanings drift. Highland gets no drift file at all.
fn family() -> (LanguageGenome, LanguageGenome, LanguageGenome) {
    let proto = load_genome("asterian_attested.ron");
    let (coastal, report) = proto
        .evolve(
            "coastal",
            "Coastal Asterian",
            &load_rules("rules_coastal.ron"),
            470,
        )
        .expect("Coastal evolves");
    assert!(report.is_ok(), "the Coastal evolution is clean: {report}");

    let (coastal, report) = coastal
        .with_drift(&load_drift("drift_coastal.ron"))
        .expect("Coastal drifts");
    assert!(report.is_ok(), "the Coastal drift is clean: {report}");

    let (highland, _) = proto
        .evolve(
            "highland",
            "Highland Asterian",
            &load_rules("rules_highland.ron"),
            460,
        )
        .expect("Highland evolves");
    (proto, coastal, highland)
}

/// `*takala`'s reflex in a language — found by **cognate set**, never by meaning,
/// because meaning is the thing under test.
fn takala_reflex(genome: &LanguageGenome) -> &WordEntry {
    let set = stem_core::CognateSetId::new("cog_asterian_attested_0001");
    genome
        .lexicon
        .by_cognate_set(&set)
        .unwrap_or_else(|| panic!("`{}` lost the *takala cognate set", genome.id))
}

// ------------------------------------------------------------ THE acceptance

/// **ROADMAP M9, the acceptance.** `*takala` "star" means "omen" in Coastal and
/// still means "star" in Highland — and both are the same cognate set.
#[test]
fn takala_becomes_omen_in_coastal_and_stays_star_in_highland() {
    let (proto, coastal, highland) = family();

    assert_eq!(takala_reflex(&proto).display_gloss(), Some("star"));
    assert_eq!(
        takala_reflex(&coastal).display_gloss(),
        Some("omen"),
        "Coastal's reflex means something new"
    );
    assert_eq!(
        takala_reflex(&highland).display_gloss(),
        Some("star"),
        "Highland's reflex kept the inherited sense"
    );

    // The forms diverged too, by sound change — and independently of the meaning.
    assert_eq!(
        takala_reflex(&coastal).written(&coastal.phonemes).unwrap(),
        "taal"
    );
    assert_eq!(
        takala_reflex(&highland)
            .written(&highland.phonemes)
            .unwrap(),
        "tagal"
    );

    // THE invariant: one ancestry, three ways of showing it.
    let set = &takala_reflex(&proto).cognate_set;
    assert_eq!(&takala_reflex(&coastal).cognate_set, set);
    assert_eq!(&takala_reflex(&highland).cognate_set, set);
}

/// `by_meaning` follows the drift: "omen" resolves in Coastal and "star" does not,
/// while Highland is the mirror image. This is what makes the divergence *visible*
/// to every meaning-addressed view (`cognates`, `trace-word`, the demo).
#[test]
fn by_meaning_diverges_between_the_sisters() {
    let (proto, coastal, highland) = family();

    assert_eq!(coastal.lexicon.by_meaning("omen").len(), 1);
    assert!(
        coastal.lexicon.by_meaning("star").is_empty(),
        "Coastal's word no longer means star"
    );
    assert_eq!(highland.lexicon.by_meaning("star").len(), 1);
    assert!(highland.lexicon.by_meaning("omen").is_empty());
    assert_eq!(
        proto.lexicon.by_meaning("star").len(),
        1,
        "the reference resolution the table anchors on is untouched"
    );
}

/// The drifted reflex keeps its **row**: the comparative table joins by ancestry,
/// so a word that changed meaning is still shown beside its sisters — annotated
/// with what it means now.
#[test]
fn the_drifted_reflex_keeps_its_cognate_set_and_its_row() {
    let (proto, coastal, highland) = family();
    let graph = stem_genome::LineageGraph::assemble(vec![proto, coastal, highland]);
    let table = graph
        .cognate_table(&["star".to_owned()])
        .expect("the table builds");

    assert_eq!(table.rows.len(), 1);
    let row = &table.rows[0];
    assert_eq!(
        row.cognate_set.as_ref().unwrap().as_str(),
        "cog_asterian_attested_0001"
    );

    // All three columns are filled — the drifted daughter did NOT fall out.
    for (i, cell) in row.cells.iter().enumerate() {
        assert!(
            cell.is_some(),
            "column {i} lost its reflex; the join is not by meaning"
        );
    }
    let coastal_cell = row.cells[1].as_ref().unwrap();
    assert_eq!(coastal_cell.form, "taal");
    assert_eq!(coastal_cell.gloss.as_deref(), Some("omen"));
    assert!(coastal_cell.drifted, "Coastal's sense moved");

    let highland_cell = row.cells[2].as_ref().unwrap();
    assert_eq!(highland_cell.form, "tagal");
    assert!(
        !highland_cell.drifted,
        "Highland means what the reference means, so nothing is annotated"
    );
}

// -------------------------------------------------- §3.3 traceability, for meaning

/// The record reconstructs the state: replaying the stored history reproduces the
/// stored senses. The meaning twin of "a trace's final form equals the stored form".
#[test]
fn the_semantic_history_replays_to_the_stored_senses() {
    let (_, coastal, _) = family();
    let entry = takala_reflex(&coastal);
    let history = entry
        .sense_history
        .as_ref()
        .expect("the drifted word carries a history");

    assert_eq!(history.steps.len(), 2, "two recorded shifts");
    assert_eq!(
        history.input,
        vec![stem_core::SemanticNodeId::new("sn_star")],
        "the history begins at the INHERITED sense, not the current one"
    );

    let replayed = history.final_senses();
    let held: Vec<stem_core::SemanticNodeId> =
        entry.senses.iter().map(|s| s.node.clone()).collect();
    assert_eq!(replayed, held, "the record is lossless");
}

/// Drift moves meaning and **nothing else** — most of all not ancestry.
#[test]
fn drift_touches_only_meaning() {
    let proto = load_genome("asterian_attested.ron");
    let (evolved, _) = proto
        .evolve("c", "C", &load_rules("rules_coastal.ron"), 470)
        .expect("evolves");
    let (drifted, _) = evolved
        .with_drift(&load_drift("drift_coastal.ron"))
        .expect("drifts");

    for (before, after) in evolved.lexicon.iter().zip(drifted.lexicon.iter()) {
        assert_eq!(before.id, after.id);
        assert_eq!(
            before.cognate_set, after.cognate_set,
            "ancestry is untouched"
        );
        assert_eq!(
            before.concept, after.concept,
            "the etymological anchor holds"
        );
        assert_eq!(
            before.phonemic_form, after.phonemic_form,
            "forms are untouched"
        );
        assert_eq!(
            before.trace, after.trace,
            "the sound-change record is untouched"
        );
        assert_eq!(
            before.glosses, after.glosses,
            "authored labels are never clobbered"
        );
        assert_eq!(before.morphemes, after.morphemes);
        assert_eq!(before.source, after.source);
    }
    // And the lineage identity is the same stage — `with_drift` is not a fork.
    assert_eq!(evolved.id, drifted.id);
    assert_eq!(evolved.applied_rules.len(), drifted.applied_rules.len());
}

/// A drifted word survives further sound change with its meaning history intact —
/// the engine carries `senses`/`sense_history` through a clone, with no engine
/// change (the free ride M8's `morphemes` got).
#[test]
fn a_drifted_word_survives_a_later_sound_change() {
    let (_, coastal, _) = family();
    let (later, _) = coastal
        .evolve(
            "later",
            "Later Coastal",
            &load_rules("rules_intervocalic_voicing.ron"),
            50,
        )
        .expect("evolves again");

    let entry = takala_reflex(&later);
    assert_eq!(entry.display_gloss(), Some("omen"), "the meaning survived");
    assert_eq!(
        entry.sense_history.as_ref().unwrap().steps.len(),
        2,
        "the meaning history survived"
    );
    assert!(later.validate().is_ok(), "{}", later.validate());
}

/// Determinism: the same drift set applied twice gives byte-identical genomes.
#[test]
fn applying_a_drift_set_twice_produces_identical_output() {
    let proto = load_genome("asterian_attested.ron");
    let set = load_drift("drift_coastal.ron");
    let a = proto.with_drift(&set).expect("drifts").0;
    let b = proto.with_drift(&set).expect("drifts").0;
    assert_eq!(a, b, "no RNG anywhere on the drift path");
}

// ------------------------------------------------------------ profile & render

/// The band scores the two-step chain `Drifted` and — being below the bar — leaves
/// the "long chain" Note silent. The tool's own showcase must not trip its own
/// remarkable-ness warning.
#[test]
fn the_profile_scores_drifted_and_stays_below_the_long_chain_note() {
    let (proto, coastal, highland) = family();

    assert_eq!(
        coastal.plausibility_profile().semantic_drift,
        SemanticDrift::Drifted
    );
    assert_eq!(
        highland.plausibility_profile().semantic_drift,
        SemanticDrift::Stable,
        "Highland declares senses and never moved them"
    );
    assert_eq!(
        proto.plausibility_profile().semantic_drift,
        SemanticDrift::Stable
    );

    let chains = sense_chains(&coastal.lexicon);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].shifts, 2);
    assert!(
        chains[0].shifts < LONG_SENSE_CHAIN,
        "the demo's own chain sits below the extreme bar"
    );

    let report = coastal.validate();
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.code == "long_semantic_drift_chain"),
        "a two-step chain is ordinary: {report}"
    );
}

/// §10.2's worked trace, complete: the sound-change half **and** the meaning half,
/// both engine-produced.
#[test]
fn the_word_history_renders_both_halves_of_the_worked_example() {
    let (_, coastal, highland) = family();
    let text = render_word_history(&coastal, takala_reflex(&coastal)).expect("renders");

    // The sound-change half (M3), unchanged.
    assert!(text.contains("takala"), "{text}");
    assert!(text.contains("Intervocalic voicing"), "{text}");
    assert!(text.contains("taal"), "{text}");
    // The meaning half (M9) — §10.2's "Semantic shift: star → omen in priestly
    // register", produced rather than prose.
    assert!(text.contains("star > divine sign"), "{text}");
    assert!(text.contains("divine sign > omen, royal sign"), "{text}");
    assert!(
        text.contains("metaphor") && text.contains("metonymy"),
        "{text}"
    );
    assert!(text.contains("priestly"), "{text}");

    // An undrifted word renders no semantic block at all — the byte-identity
    // guarantee that keeps every pre-M9 trace output unchanged.
    let quiet = render_word_history(&highland, takala_reflex(&highland)).expect("renders");
    assert!(
        !quiet.contains("sense ") && !quiet.contains("means "),
        "Highland never drifted, so it has no semantic block:\n{quiet}"
    );
}

// ---------------------------------------------------------------- serde & CLI

/// A pre-M9 language serialises with **zero** semantic bytes, so every saved file
/// round-trips unchanged.
#[test]
fn a_pre_m9_language_serialises_without_any_semantic_field() {
    let genome = load_genome("proto_asterian.ron");
    let dir = std::env::temp_dir().join(format!("stemma_m9_serde_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("roundtrip.ron");
    stem_io::save(&path, &genome).expect("saves");
    let text = std::fs::read_to_string(&path).expect("reads back");

    for absent in ["semantics", "applied_drifts", "senses", "sense_history"] {
        assert!(
            !text.contains(absent),
            "an empty `{absent}` must not appear in a pre-M9 file"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_drift_and_drifts_subcommands_are_wired_up() {
    assert!(stemma(&["drift", "--help"]).status.success());
    assert!(stemma(&["drifts", "--help"]).status.success());
}

/// End-to-end through the binary: evolve → drift → compare → trace.
#[test]
fn the_cli_pipeline_evolves_drifts_and_shows_the_divergence() {
    let dir = std::env::temp_dir().join(format!("stemma_m9_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let coastal = dir.join("coastal.ron");
    let modern = dir.join("coastal_modern.ron");
    let highland = dir.join("highland.ron");

    let run = |args: &[&str]| {
        let out = stemma(args);
        assert!(out.status.success(), "`{args:?}` failed: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    };

    run(&[
        "apply-rules",
        fixture("asterian_attested.ron").to_str().unwrap(),
        "--rules",
        fixture("rules_coastal.ron").to_str().unwrap(),
        "--id",
        "coastal",
        "--name",
        "Coastal",
        "--years",
        "470",
        "--out",
        coastal.to_str().unwrap(),
    ]);
    run(&[
        "apply-rules",
        fixture("asterian_attested.ron").to_str().unwrap(),
        "--rules",
        fixture("rules_highland.ron").to_str().unwrap(),
        "--id",
        "highland",
        "--name",
        "Highland",
        "--years",
        "460",
        "--out",
        highland.to_str().unwrap(),
    ]);
    run(&[
        "drift",
        coastal.to_str().unwrap(),
        "--drift",
        fixture("drift_coastal.ron").to_str().unwrap(),
        "--id",
        "coastal_modern",
        "--name",
        "Modern Coastal",
        "--years",
        "30",
        "--out",
        modern.to_str().unwrap(),
    ]);

    // The comparative table shows one row, three reflexes, one annotated.
    let table = run(&[
        "cognates",
        fixture("asterian_attested.ron").to_str().unwrap(),
        coastal.to_str().unwrap(),
        modern.to_str().unwrap(),
        highland.to_str().unwrap(),
        "--meanings",
        "star",
    ]);
    assert!(table.contains("taal \"omen\""), "{table}");
    assert!(table.contains("tagal"), "{table}");

    // The trace shows both halves.
    let trace = run(&["trace-word", modern.to_str().unwrap(), "omen"]);
    assert!(trace.contains("Intervocalic voicing"), "{trace}");
    assert!(trace.contains("metaphor"), "{trace}");

    // And the profile scores it.
    let profile = run(&["profile", modern.to_str().unwrap()]);
    assert!(
        profile.contains("Semantic drift") && profile.contains("drifted"),
        "{profile}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
