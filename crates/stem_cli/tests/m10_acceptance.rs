//! The ROADMAP M10 acceptance: **the M3 rule set expressed in the DSL produces
//! byte-identical output to the hand-built structs.**
//!
//! That sentence is the whole milestone. A parser that produced *equivalent* output
//! would be a second engine with a second set of behaviours to keep in sync; a
//! parser that produces the identical bytes is a front end, and can only ever be a
//! front end (§20.4).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_genome::LanguageGenome;
use stem_soundchange::{RuleSet, parse_rule_set};

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

/// The `.sc` file, parsed.
fn dsl_set() -> RuleSet {
    let path = fixture("rules_asterian.sc");
    let source = std::fs::read_to_string(&path).expect("the .sc fixture exists");
    parse_rule_set(&source, path.to_str().unwrap()).expect("the DSL fixture parses")
}

/// The `.ron` file, deserialised — the M3 hand-built structs.
fn ron_set() -> RuleSet {
    stem_io::load(fixture("rules_asterian.ron")).expect("the .ron fixture loads")
}

// ------------------------------------------------------------- the acceptance

/// **ROADMAP M10.** The two sources produce the *same structs*, field for field.
///
/// Asserted before the output comparison because it localises a failure: if this
/// passes and the output test fails, the engine is nondeterministic; if this fails,
/// the parser is wrong. Two different bugs, and the pair of tests tells them apart.
#[test]
fn the_dsl_parses_to_exactly_the_hand_built_structs() {
    let (dsl, ron) = (dsl_set(), ron_set());

    assert_eq!(dsl.id, ron.id);
    assert_eq!(dsl.name, ron.name);
    assert_eq!(dsl.description, ron.description);
    assert_eq!(
        dsl.rules.len(),
        ron.rules.len(),
        "the DSL file must express every rule, and no extra"
    );

    // Rule by rule, so a failure names which one rather than dumping both sets.
    for (a, b) in dsl.rules.iter().zip(ron.rules.iter()) {
        assert_eq!(a.id, b.id, "rule ids diverged");
        assert_eq!(a, b, "rule `{}` parsed to a different struct", a.id);
    }
    assert_eq!(dsl, ron, "the whole set is identical");
}

/// **ROADMAP M10, the headline.** Applying the parsed rules and applying the
/// hand-built rules give byte-identical evolved languages.
///
/// Compared as **serialised bytes**, not as `PartialEq` — the milestone's word is
/// "byte-identical", and a struct comparison would not catch a field that differs
/// only in what it serialises to.
#[test]
fn the_dsl_and_the_ron_set_produce_byte_identical_output() {
    let proto: LanguageGenome =
        stem_io::load(fixture("asterian_attested.ron")).expect("the fixture loads");

    let (from_dsl, _) = proto
        .evolve("early", "Early", &dsl_set(), 480)
        .expect("the DSL rules apply");
    let (from_ron, _) = proto
        .evolve("early", "Early", &ron_set(), 480)
        .expect("the RON rules apply");

    let dir = std::env::temp_dir().join(format!("stemma_m10_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let (a, b) = (dir.join("from_dsl.ron"), dir.join("from_ron.ron"));
    stem_io::save(&a, &from_dsl).expect("saves");
    stem_io::save(&b, &from_ron).expect("saves");

    let bytes_a = std::fs::read(&a).expect("reads");
    let bytes_b = std::fs::read(&b).expect("reads");
    assert_eq!(
        bytes_a, bytes_b,
        "the DSL is a front end over the same structs, so the evolved languages must \
         be byte-identical — not merely equivalent"
    );

    // And the derivations really did happen (a pair of empty lexicons would also be
    // byte-identical, and would prove nothing).
    let star = from_dsl
        .lexicon
        .by_meaning("star")
        .first()
        .copied()
        .expect("the star reflex exists")
        .clone();
    assert_eq!(
        star.written(&from_dsl.phonemes).unwrap(),
        "taɣal",
        "the DSL rules produced §10.2's worked chain"
    );
    assert!(
        star.trace.is_some_and(|t| !t.steps.is_empty()),
        "the reflex carries a real derivation"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The parser is deterministic: two parses of one file agree.
#[test]
fn parsing_the_same_source_twice_produces_identical_rules() {
    assert_eq!(dsl_set(), dsl_set());
}

// ------------------------------------------------------------------- the CLI

#[test]
fn the_rules_verb_reads_a_dsl_file() {
    let out = stemma(&["rules", fixture("rules_asterian.sc").to_str().unwrap()]);
    assert!(out.status.success(), "rules on a .sc file failed: {out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Early Asterian sound changes"), "{text}");
    assert!(text.contains("Intervocalic voicing"), "{text}");
}

/// `apply-rules` takes a `.sc` file wherever it takes a `.ron` one, and the CLI
/// pipeline reproduces the same forms.
#[test]
fn apply_rules_accepts_a_dsl_file_and_gives_the_same_forms() {
    let dir = std::env::temp_dir().join(format!("stemma_m10_cli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let out_file = dir.join("early.ron");

    let run = stemma(&[
        "apply-rules",
        fixture("asterian_attested.ron").to_str().unwrap(),
        "--rules",
        fixture("rules_asterian.sc").to_str().unwrap(),
        "--id",
        "early",
        "--name",
        "Early",
        "--years",
        "480",
        "--out",
        out_file.to_str().unwrap(),
    ]);
    assert!(
        run.status.success(),
        "apply-rules with a .sc failed: {run:?}"
    );

    let trace = stemma(&["trace", out_file.to_str().unwrap(), "w_0001"]);
    let text = String::from_utf8(trace.stdout).unwrap();
    assert!(
        text.contains("taɣal"),
        "the DSL-driven pipeline produced the worked chain:\n{text}"
    );
    assert!(
        text.contains("r_0001") && text.contains("Intervocalic voicing"),
        "the trace names the rules the DSL declared:\n{text}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A parse error names the file, the line, and the offending text — a failure the
/// user cannot locate is one they cannot fix.
#[test]
fn a_malformed_dsl_file_reports_where_it_broke() {
    let source = "rules s \"S\":\n\nrule r_1 \"R\":\n  target: [+nonsense]\n  change: delete\n";
    let e = parse_rule_set(source, "broken.sc").expect_err("a bad feature name");
    let text = e.to_string();
    assert!(text.contains("broken.sc"), "names the file: {text}");
    assert!(text.contains("line 4"), "names the line: {text}");
    assert!(text.contains("nonsense"), "names the text: {text}");
}
