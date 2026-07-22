//! The ROADMAP M5 acceptance tests, over the real fixtures and the real binary.
//!
//! Lives here for the same reason `m3`/`m4_acceptance.rs` do: loading a fixture
//! needs `stem_io` + `stem_genome`, above the engine crates.
//!
//! ROADMAP M5: "stemma cognates --meanings water sun star king mother prints the
//! §10.3 table across all three daughters; stemma trace-word coastal star prints
//! the full derivation, rule by rule, from proto-form to modern form." The
//! `coastal star` phrasing maps to `out/coastal.ron star` — the file-native
//! deviation already accepted for `trace`/`family` (docs/adr/0008).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// Fork the three daughters into a per-test temp dir (named by `tag`) and return
/// its path. Each M5 test that needs daughters builds them through the real
/// `fork` pipeline, so the acceptance exercises the whole chain end to end. The
/// `tag` isolates concurrent tests — cargo runs them in parallel, and a shared
/// dir would let one test's fork truncate a file another is reading.
fn build_family(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stemma_m5_{tag}"));
    std::fs::create_dir_all(&dir).unwrap();
    for (id, rules, years) in [
        ("coastal", "rules_coastal.ron", "470"),
        ("highland", "rules_highland.ron", "460"),
        ("riverine", "rules_riverine.ron", "420"),
    ] {
        let out = stemma(&[
            "fork",
            fixture("asterian_attested.ron").to_str().unwrap(),
            "--rules",
            fixture(rules).to_str().unwrap(),
            "--id",
            id,
            "--name",
            &format!("{id} Asterian"),
            "--years",
            years,
            "--out",
            dir.join(format!("{id}.ron")).to_str().unwrap(),
        ]);
        assert!(out.status.success(), "fork {id} failed: {out:?}");
    }
    dir
}

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

// ------------------------------------------------------ §10.3 cognate table

/// **ROADMAP M5, view one.** The exact §10.3 table across the whole family.
/// Baselined from the real renderer; M6 will script against these bytes.
#[test]
fn the_cognates_command_prints_the_section_10_3_table_across_the_family() {
    let dir = build_family("table");
    let out = stemma(&[
        "cognates",
        fixture("asterian_attested.ron").to_str().unwrap(),
        dir.join("coastal.ron").to_str().unwrap(),
        dir.join("highland.ron").to_str().unwrap(),
        dir.join("riverine.ron").to_str().unwrap(),
        "--meanings",
        "water",
        "sun",
        "star",
        "king",
        "mother",
    ]);
    assert!(out.status.success(), "cognates failed: {out:?}");

    let expected = "\
meaning  asterian_attested  coastal  highland  riverine
water    *akwa              akw      akw       akwa
sun      *sawel             sawel    sawel     sawel
star     *takala            taal     tagal     tala
king     *rekan             rean     regan     rean
mother   *mikala            mial     migal     miala
";
    assert_eq!(stdout_of(&out), expected, "the §10.3 table drifted");
}

/// The crux: `king` resolves through `*rekan`'s gloss override (concept MAN),
/// not the word-less KING concept, and the row is filled by cognate set.
#[test]
fn stemma_cognates_joins_king_through_the_gloss_override() {
    let dir = build_family("king");
    let out = stemma(&[
        "cognates",
        fixture("asterian_attested.ron").to_str().unwrap(),
        dir.join("coastal.ron").to_str().unwrap(),
        dir.join("highland.ron").to_str().unwrap(),
        dir.join("riverine.ron").to_str().unwrap(),
        "--meanings",
        "king",
    ]);
    let text = stdout_of(&out);
    let king = text
        .lines()
        .find(|l| l.starts_with("king"))
        .expect("a king row");
    for reflex in ["*rekan", "rean", "regan"] {
        assert!(king.contains(reflex), "king row missing `{reflex}`: {king}");
    }
}

#[test]
fn stemma_cognates_stdout_is_byte_identical_across_two_runs() {
    let dir = build_family("twice");
    let args = [
        "cognates",
        fixture("asterian_attested.ron").to_str().unwrap(),
        dir.join("coastal.ron").to_str().unwrap(),
        "--meanings",
        "star",
        "stone",
    ]
    .map(String::from);
    let a = stdout_of(&stemma(
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    let b = stdout_of(&stemma(
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    ));
    assert_eq!(a, b, "two runs must be byte-identical");
}

/// The `ŋ` alignment case (STONE `sank`/`saŋk`/`saŋka`): char-count padding, not
/// byte length, keeps the columns aligned.
#[test]
fn stemma_cognates_aligns_columns_with_wide_glyphs() {
    let dir = build_family("align");
    let out = stemma(&[
        "cognates",
        fixture("asterian_attested.ron").to_str().unwrap(),
        dir.join("highland.ron").to_str().unwrap(),
        "--meanings",
        "stone",
    ]);
    let text = stdout_of(&out);
    assert!(text.contains("saŋk"), "the ŋ reflex is present:\n{text}");
    for line in text.lines() {
        assert_eq!(line, line.trim_end(), "no trailing whitespace: {line:?}");
    }
}

// ------------------------------------------------------ §10.2 trace by meaning

/// **ROADMAP M5, view two.** `trace-word coastal star` prints the full
/// derivation from proto to modern.
#[test]
fn trace_word_coastal_star_prints_the_derivation_from_proto_to_modern() {
    let dir = build_family("twstar");
    let out = stemma(&[
        "trace-word",
        dir.join("coastal.ron").to_str().unwrap(),
        "star",
    ]);
    let text = stdout_of(&out);
    assert!(text.contains("takala"), "proto form: {text}");
    assert!(text.contains("taal"), "modern form: {text}");
    let idx = |r: &str| {
        text.find(r)
            .unwrap_or_else(|| panic!("missing {r}:\n{text}"))
    };
    assert!(
        idx("r_ivv") < idx("r_velar_lenition")
            && idx("r_velar_lenition") < idx("r_gamma_loss")
            && idx("r_gamma_loss") < idx("r_apocope"),
        "rule blocks in chronological order:\n{text}"
    );
}

/// `trace-word` is `trace` with meaning-addressing — same renderer, so the two
/// must be byte-identical for a meaning that resolves to one word.
#[test]
fn stemma_trace_word_by_meaning_equals_trace_by_id() {
    let dir = build_family("eq");
    let coastal = dir.join("coastal.ron");
    let by_meaning = stdout_of(&stemma(&["trace-word", coastal.to_str().unwrap(), "star"]));
    let by_id = stdout_of(&stemma(&["trace", coastal.to_str().unwrap(), "w_0001"]));
    assert_eq!(by_meaning, by_id, "trace-word star must equal trace w_0001");
}

/// `king` addresses `w_0005` via its gloss override — so the two views agree.
#[test]
fn stemma_trace_word_addresses_king_by_its_gloss_override() {
    let dir = build_family("twking");
    let coastal = dir.join("coastal.ron");
    let by_meaning = stdout_of(&stemma(&["trace-word", coastal.to_str().unwrap(), "king"]));
    let by_id = stdout_of(&stemma(&["trace", coastal.to_str().unwrap(), "w_0005"]));
    assert_eq!(by_meaning, by_id, "king resolves to w_0005 (rean)");
    assert!(
        by_meaning.contains("rekan"),
        "traces from *rekan: {by_meaning}"
    );
}

#[test]
fn stemma_trace_word_of_an_unknown_meaning_fails_naming_it() {
    let dir = build_family("unknown");
    let out = stemma(&[
        "trace-word",
        dir.join("coastal.ron").to_str().unwrap(),
        "dragon",
    ]);
    assert!(!out.status.success(), "an unknown meaning is a usage error");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("dragon"),
        "the error names the meaning: {stderr}"
    );
}

// ------------------------------------------------------ the resolvers agree

#[test]
fn the_attested_fixture_resolves_mother_through_the_concept_gloss() {
    // Documents why w_0009 was added: MOTHER has no gloss override, so it
    // resolves via the concept gloss — the concept-path counterpart to king.
    let genome: stem_genome::LanguageGenome =
        stem_io::load(fixture("asterian_attested.ron")).unwrap();
    let matches = genome.lexicon.by_meaning("mother");
    assert_eq!(matches.len(), 1, "exactly one MOTHER word");
    assert_eq!(matches[0].id.as_str(), "w_0009");
    assert!(
        matches[0].glosses.is_empty(),
        "MOTHER carries no override — it resolves by its concept gloss"
    );
}

#[test]
fn the_cognates_and_trace_word_subcommands_are_wired_up() {
    assert!(stemma(&["cognates", "--help"]).status.success());
    assert!(stemma(&["trace-word", "--help"]).status.success());
}
