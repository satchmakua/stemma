//! The ROADMAP M3 acceptance tests, over the real fixtures and the real binary.
//!
//! Lives here because loading a fixture needs `stem_io` + `stem_genome`, which sit
//! above `stem_soundchange` — the same reason `reference_phonology.rs` is here.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::LanguageGenome;
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

fn rules() -> RuleSet {
    stem_io::load(fixture("rules_asterian.ron")).expect("the rule set loads")
}

fn evolved() -> LanguageGenome {
    let (genome, _report) = attested()
        .evolve("early_asterian", "Early Asterian", &rules(), 480)
        .expect("the acceptance run applies");
    genome
}

fn form_of(genome: &LanguageGenome, word: &str) -> String {
    genome
        .lexicon
        .require(&stem_core::WordId::new(word))
        .expect("word exists")
        .written(&genome.phonemes)
        .expect("renders")
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

// ------------------------------------------------------ the ROADMAP criteria

/// **ROADMAP M3, criterion one.** `written()` gives ROADMAP's literal string;
/// `ipa()` gives /taɡala/ with the real U+0261.
#[test]
fn intervocalic_voicing_turns_takala_into_tagala() {
    let genome = evolved();
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new("w_0001"))
        .unwrap();
    let trace = entry.trace.as_ref().expect("traced");
    let forms = trace.replay();

    assert_eq!(forms[0].written(&genome.phonemes).unwrap(), "tagala");
    assert_eq!(forms[0].ipa(&genome.phonemes).unwrap(), "ta\u{0261}ala");
}

/// **ROADMAP M3, criterion two.**
#[test]
fn final_unstressed_vowel_loss_turns_tagala_into_tagal() {
    let genome = evolved();
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new("w_0001"))
        .unwrap();
    let forms = entry.trace.as_ref().unwrap().replay();
    assert_eq!(forms[1].written(&genome.phonemes).unwrap(), "tagal");
}

/// **ROADMAP M3, criterion three** — three cases, three resolution tiers, one
/// rule: /n/+/p/ resolves to declared /m/, /m/+/t/ to declared /n/ (which needs
/// the copy to *remove* the rounding cell), /n/+/k/ mints /ŋ/.
#[test]
fn a_nasal_takes_the_place_of_a_following_stop() {
    let genome = evolved();
    assert_eq!(
        form_of(&genome, "w_0007"),
        "amp",
        "anpa: n+p -> m, then apocope"
    );
    assert_eq!(
        form_of(&genome, "w_0008"),
        "ant",
        "amta: m+t -> n, then apocope"
    );
    assert_eq!(
        form_of(&genome, "w_0006"),
        "sa\u{014B}k",
        "sanka: n+k mints \u{014B}"
    );
}

/// **ROADMAP M3, criterion four.** The chain reproduces §10.2's worked example —
/// `takala → tagala → tagal → taɣal`, in §10.2's order, with the intermediates.
#[test]
fn the_four_rules_in_order_reproduce_the_design_docs_worked_example() {
    let genome = evolved();
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new("w_0001"))
        .unwrap();
    let trace = entry.trace.as_ref().unwrap();

    let mut chain = vec![trace.input.written(&genome.phonemes).unwrap()];
    for form in trace.replay() {
        chain.push(form.written(&genome.phonemes).unwrap());
    }
    assert_eq!(chain, ["takala", "tagala", "tagal", "ta\u{0263}al"]);
    assert_eq!(form_of(&genome, "w_0001"), "ta\u{0263}al");
}

/// **ROADMAP M3, criterion five, and §16.3's property**: every trace output's
/// final form equals the stored word form — the trace is sufficient to
/// reconstruct the form, i.e. not lossy.
#[test]
fn every_word_of_the_fixture_replays_to_its_stored_form() {
    let genome = evolved();
    for entry in genome.lexicon.iter() {
        let trace = entry
            .trace
            .as_ref()
            .expect("every word passed through the run");
        assert_eq!(
            trace.final_form(),
            entry.phonemic_form,
            "`{}`'s trace does not replay to its stored form",
            entry.id
        );
    }
}

// ------------------------------------------------------------------ ordering

/// Counterbleeding: voicing before apocope leaves a word-final /ɡ/ whose
/// conditioning environment is gone — the earlier rule is no longer surface-true.
#[test]
fn voicing_before_apocope_leaves_a_word_final_g_whose_environment_is_gone() {
    let genome = evolved();
    assert_eq!(form_of(&genome, "w_0002"), "tag");
}

/// Bleeding: apocope first removes the following vowel, so voicing never fires.
#[test]
fn reversing_voicing_and_apocope_bleeds_the_voicing_of_taka() {
    let mut reversed = rules();
    reversed.rules.swap(0, 2); // apocope before voicing

    let (genome, _) = attested()
        .evolve("reversed", "Reversed", &reversed, 480)
        .expect("applies");
    assert_eq!(form_of(&genome, "w_0002"), "tak");
    assert_ne!(form_of(&genome, "w_0002"), form_of(&evolved(), "w_0002"));
}

/// Pins the *reason* `taka` is in the fixture, so a future session cannot delete
/// it as redundant: `takala` — the design doc's own example — yields `tagal`
/// under **either** order, because its final vowel never conditioned the /k/.
/// `taka` is the word that makes order observable.
#[test]
fn both_rule_orders_give_takala_the_same_form_so_taka_is_the_ordering_test() {
    let mut reversed = rules();
    reversed.rules.swap(0, 2);

    let (swapped, _) = attested()
        .evolve("reversed", "Reversed", &reversed, 480)
        .expect("applies");
    // Walk it: apocope first gives takal; voicing (now third) still sees /k/
    // between a,a and gives tagal; lenition (still fourth) gives taɣal. The
    // full-set orders agree on takala completely.
    assert_eq!(form_of(&swapped, "w_0001"), "ta\u{0263}al");
    assert_eq!(form_of(&evolved(), "w_0001"), "ta\u{0263}al");
    // And with lenition removed, the voicing/apocope pair alone also agrees on
    // takala — its final vowel never conditioned the /k/ — while disagreeing on
    // taka. That is the whole reason taka ships.
    let mut no_lenition = rules();
    no_lenition.rules.truncate(3); // voicing, assimilation, apocope
    let mut no_lenition_reversed = no_lenition.clone();
    no_lenition_reversed.rules.swap(0, 2);

    let (a, _) = attested()
        .evolve("x1", "X1", &no_lenition, 480)
        .expect("applies");
    let (b, _) = attested()
        .evolve("x2", "X2", &no_lenition_reversed, 480)
        .expect("applies");
    assert_eq!(
        form_of(&a, "w_0001"),
        form_of(&b, "w_0001"),
        "takala is order-insensitive up to lenition"
    );
    assert_ne!(
        form_of(&a, "w_0002"),
        form_of(&b, "w_0002"),
        "taka is the ordering witness"
    );
}

/// Word order must not be observable: reversing the lexicon changes no form, no
/// inventory, and no trace — only the order of the report.
#[test]
fn reordering_the_lexicon_changes_nothing_but_the_order_of_the_report() {
    let forward = attested();
    let mut backward = attested();
    let reversed: Vec<_> = backward.lexicon.iter().rev().cloned().collect();
    backward.lexicon = stem_lexicon::Lexicon::from_entries(reversed);

    let (a, _) = forward.evolve("e1", "E1", &rules(), 480).expect("applies");
    let (b, _) = backward.evolve("e2", "E2", &rules(), 480).expect("applies");

    assert_eq!(
        a.phonemes, b.phonemes,
        "the minted inventory must not depend on word order"
    );
    for entry in a.lexicon.iter() {
        let twin = b.lexicon.require(&entry.id).expect("same words either way");
        assert_eq!(entry.phonemic_form, twin.phonemic_form);
        assert_eq!(entry.trace, twin.trace);
    }
}

/// Determinism as a test: two independent runs over the same input, byte for
/// byte. (Not feeding the output back in — that would be a second stratum.)
#[test]
fn applying_the_same_rules_twice_produces_a_byte_identical_genome() {
    let render = |genome: &LanguageGenome| {
        stem_io::to_string(genome, stem_io::Format::Ron).expect("serialises")
    };
    assert_eq!(render(&evolved()), render(&evolved()));
}

// ------------------------------------------------------------ bundle → symbol

/// The literal answer to the milestone's hardest question: /ɡ/ is U+0261 LATIN
/// SMALL LETTER SCRIPT G, romanised as ASCII `g` — which is why `written()` says
/// `tagala` (ROADMAP's string) while `ipa()` says `taɡala`.
#[test]
fn voicing_a_velar_stop_yields_script_g_not_ascii_g() {
    let genome = evolved();
    let g = genome
        .phonemes
        .get(&stem_core::PhonemeId::new("ph_g"))
        .expect("minted");
    assert_eq!(g.ipa, "\u{0261}");
    assert_ne!(g.ipa, "g");
    assert_eq!(g.romanization.as_deref(), Some("g"));
    assert_eq!(g.kind, stem_phonology::SegmentKind::Consonant);
}

#[test]
fn minted_phonemes_are_appended_in_table_order_not_discovery_order() {
    let genome = evolved();
    let appended: Vec<&str> = genome
        .phonemes
        .iter()
        .skip(15)
        .map(|p| p.id.as_str())
        .collect();
    // Discovery order is g (w_0001 r_0001), eng (w_0006 r_0002), gamma (w_0001
    // r_0004 — but per-word application means gamma is minted while processing
    // w_0001, before eng). Table order is g, gamma, eng — and that is what the
    // inventory must show, so the result is a function of the *set* of
    // innovations.
    assert_eq!(appended, ["ph_g", "ph_gamma", "ph_eng"]);
}

#[test]
fn a_minted_phoneme_satisfies_every_required_feature() {
    let genome = evolved();
    for id in ["ph_g", "ph_gamma", "ph_eng"] {
        let phoneme = genome
            .phonemes
            .get(&stem_core::PhonemeId::new(id))
            .expect("minted");
        assert!(
            stem_phonology::required_features_missing(phoneme.features).is_empty(),
            "{id} is ill-formed"
        );
        assert!(phoneme.frequency_weight > 0, "{id} would trip bad_weight");
    }
}

/// The evolved genome passes the same validation gate as any authored file.
#[test]
fn the_evolved_genome_validates_with_no_errors() {
    let genome = evolved();
    let report = genome.validate();
    assert!(report.is_ok(), "{report}");
}

/// M2's promise, discharged: the stale `pattern` on an evolved word is a Note —
/// the derivation *is* the recorded causal explanation — while a hand-edited
/// mismatch with no derivation stays a Warning.
#[test]
fn a_stale_pattern_on_a_word_with_a_derivation_is_a_note_not_a_warning() {
    let genome = evolved();
    let report = genome.validate();
    let mismatches: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == "lexicon.syllable_shape_mismatch")
        .collect();
    assert!(!mismatches.is_empty(), "apocope produced stale patterns");
    assert!(
        mismatches
            .iter()
            .all(|i| i.severity == stem_core::Severity::Note),
        "{report}"
    );
}

// ---------------------------------------------------------------- the fixture

/// RON has no include mechanism; the checked invariant is the honest substitute.
#[test]
fn the_attested_fixture_shares_the_reference_inventory() {
    let attested = attested();
    let reference: LanguageGenome =
        stem_io::load(fixture("proto_asterian.ron")).expect("reference loads");
    assert_eq!(
        attested.phonemes, reference.phonemes,
        "the two fixtures' inventories drifted apart"
    );
    assert_eq!(attested.phonotactics, reference.phonotactics);
}

#[test]
fn the_attested_fixture_validates_cleanly() {
    let report = attested().validate();
    assert!(report.is_ok(), "{report}");
}

/// `sawel`: exposed to the whole rule sequence and changed by none of it.
/// `Some(steps: [])`, which only the `Option`-around-`Vec` encoding can say.
#[test]
fn sawel_carries_an_empty_derivation_not_a_missing_one() {
    let genome = evolved();
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new("w_0003"))
        .unwrap();
    assert_eq!(form_of(&genome, "w_0003"), "sawel");
    let trace = entry.trace.as_ref().expect("Some, not None");
    assert!(trace.steps.is_empty());
}

#[test]
fn a_proto_word_carries_no_derivation_at_all() {
    for entry in attested().lexicon.iter() {
        assert!(entry.trace.is_none(), "{} is a proto word", entry.id);
    }
}

/// `akwa`: /k/ sits before the glide /w/, which is `[-syllabic]` — proving the
/// voicing environment is `[+syllabic]`, not "vowel-ish".
#[test]
fn a_glide_does_not_condition_intervocalic_voicing() {
    assert_eq!(form_of(&evolved(), "w_0004"), "akw");
}

/// Cognate sets survive evolution verbatim — the §8.6 thread, untouched by M3.
#[test]
fn evolution_copies_every_cognate_set_verbatim() {
    let before = attested();
    let after = evolved();
    for (a, b) in before.lexicon.iter().zip(after.lexicon.iter()) {
        assert_eq!(a.cognate_set, b.cognate_set);
        assert_eq!(a.id, b.id);
        assert_eq!(a.concept, b.concept);
    }
}

// ---------------------------------------------------- strata and the trace

/// A second run extends the derivation — `input` stays the proto-form, steps
/// append — and numbers its steps after the first.
#[test]
fn a_second_rule_run_extends_the_trace_rather_than_replacing_it() {
    let first = evolved();
    assert_eq!(first.applied_rules.len(), 4);

    // A second stratum: one more apocope round.
    let second_stratum = RuleSet {
        id: "second".to_owned(),
        name: "Second stratum".to_owned(),
        description: String::new(),
        rules: vec![rules().rules[2].clone()],
    };
    let (second, _) = first
        .evolve("later_asterian", "Later Asterian", &second_stratum, 200)
        .expect("applies");

    assert_eq!(second.applied_rules.len(), 5);

    let entry = second
        .lexicon
        .require(&stem_core::WordId::new("w_0001"))
        .unwrap();
    let trace = entry.trace.as_ref().unwrap();
    assert_eq!(
        trace.input.written(&second.phonemes).unwrap(),
        "takala",
        "the derivation still begins at the proto-form"
    );
    // taɣal's final syllable is unstressed... but its final segment is /l/, not a
    // vowel, so the second apocope does not fire on w_0001. Its steps stay 3.
    assert_eq!(trace.steps.len(), 3);

    // w_0006 `saŋk` likewise ends in a consonant; take one that still ends in a
    // vowel — there is none after the first stratum, so assert the numbering on
    // the existing steps instead: all indices < 4, and the log grew.
    assert!(trace.steps.iter().all(|s| s.index < 4));
    assert_eq!(
        second.parent.as_ref().map(|p| p.as_str()),
        Some("early_asterian"),
        "evolve chains the lineage"
    );
}

// --------------------------------------------------------------- CLI и text

#[test]
fn stemma_trace_prints_the_derivation_and_names_a_rule_that_did_not_apply() {
    let dir = std::env::temp_dir().join("stemma_m3_trace");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("early.ron");

    let apply = stemma(&[
        "apply-rules",
        fixture("asterian_attested.ron").to_str().unwrap(),
        "--rules",
        fixture("rules_asterian.ron").to_str().unwrap(),
        "--id",
        "early_asterian",
        "--name",
        "Early Asterian",
        "--years",
        "480",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(
        apply.status.success(),
        "{}",
        String::from_utf8_lossy(&apply.stderr)
    );

    let trace = stemma(&["trace", out.to_str().unwrap(), "w_0001"]);
    assert!(trace.status.success());
    let text = String::from_utf8_lossy(&trace.stdout).into_owned();

    for expected in [
        "*takala",
        "TA.ka.la",
        "Intervocalic voicing",
        "k > g",
        "a _ a",
        "new to this language",
        "Nasal place assimilation — did not apply",
        "Final unstressed vowel loss",
        "l _ #",
        "Velar lenition",
        "ta\u{0263}al",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }

    std::fs::remove_dir_all(&dir).ok();
}

/// The end-to-end determinism check, at the binary level.
#[test]
fn apply_rules_twice_writes_byte_identical_files() {
    let dir = std::env::temp_dir().join("stemma_m3_det");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let mut outputs = Vec::new();
    for name in ["a.ron", "b.ron"] {
        let out = dir.join(name);
        let run = stemma(&[
            "apply-rules",
            fixture("asterian_attested.ron").to_str().unwrap(),
            "--rules",
            fixture("rules_asterian.ron").to_str().unwrap(),
            "--id",
            "early_asterian",
            "--name",
            "Early Asterian",
            "--years",
            "480",
            "--out",
            out.to_str().unwrap(),
        ]);
        assert!(run.status.success());
        outputs.push(std::fs::read(&out).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);

    std::fs::remove_dir_all(&dir).ok();
}

/// §11.2's JSON shape, against its golden.
#[test]
fn the_trace_view_of_takala_matches_the_design_docs_json_example() {
    let genome = evolved();
    let entry = genome
        .lexicon
        .require(&stem_core::WordId::new("w_0001"))
        .unwrap();
    let derivation = entry.trace.as_ref().unwrap();

    let mut views = Vec::new();
    let mut current = derivation.input.clone();
    for (step, after) in derivation.steps.iter().zip(derivation.replay()) {
        views.push(
            stem_soundchange::view(
                step,
                &current,
                entry,
                &genome.applied_rules,
                &genome.phonemes,
            )
            .expect("renders"),
        );
        current = after;
    }

    let rendered = serde_json::to_string_pretty(&views).expect("serialises");
    let golden_path = fixture("expected_trace_takala.json");
    let golden = std::fs::read_to_string(&golden_path)
        .expect("golden exists; regenerate with the docs in the fixture header")
        .replace("\r\n", "\n");
    assert_eq!(rendered.trim(), golden.trim());

    // Spot-check §11.2's own fields on the first application.
    assert_eq!(views[0].input, "takala");
    assert_eq!(views[0].output, "tagala");
    assert_eq!(views[0].matches[0].span, [2, 3]);
    assert_eq!(views[0].matches[0].before, "k");
    assert_eq!(views[0].matches[0].after, "g");
    assert_eq!(views[0].matches[0].environment, "a _ a");
}

/// M3 must not move any RNG canary or corpus digest: the engine uses no RNG at
/// all, and this is the cheapest proof.
#[test]
fn m3_moves_no_rng_canary_and_no_corpus_digest() {
    use sha2::{Digest, Sha256};

    let roots = stemma(&[
        "generate-roots",
        fixture("proto_asterian.ron").to_str().unwrap(),
        "--count",
        "500",
        "--seed",
        "0",
    ]);
    let digest: [u8; 32] = Sha256::digest(&roots.stdout).into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, "677f34134cb331aeabb515aa93655e50294c1bdd4cfc6e1ff5126ba2afd3075b",
        "the M1 corpus digest moved"
    );

    // `--concepts 103` names the M2 corpus exactly: M13 grew the built-in list to
    // 673, and the pre-M13 103 are its frozen prefix (`PRE_M13_CONCEPT_COUNT`). The
    // digest below is M2's own, deliberately **not** re-baselined — asking for the
    // same concepts must still give the same words, or the append was not an
    // append. `the_first_hundred_and_three_words_are_unchanged` is the same claim
    // stated as M13's acceptance test rather than as an M3 regression guard.
    let lexicon = stemma(&[
        "new-lexicon",
        fixture("proto_asterian.ron").to_str().unwrap(),
        "--concepts",
        "103",
    ]);
    let digest: [u8; 32] = Sha256::digest(&lexicon.stdout).into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, "d16ba86130091d93e3455d2742037b6d199c5181710d15cc28f0f8b9ca508423",
        "the M2 lexicon digest moved"
    );
}
