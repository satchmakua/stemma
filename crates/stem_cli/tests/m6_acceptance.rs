//! The ROADMAP M6 acceptance tests: the `stemma demo` command over the real
//! binary. Byte-exactness is pinned by the `stem_export` golden
//! (`the_asterian_demo_matches_its_golden_file`); this file pins the *command's*
//! behaviour and the document landmarks, and the two share the same `fixtures/`
//! so they cannot silently disagree.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STEMMA: &str = env!("CARGO_BIN_EXE_stemma");

fn stemma(args: &[&str]) -> Output {
    Command::new(STEMMA)
        .args(args)
        .output()
        .expect("failed to run the stemma binary")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stemma_m6_{tag}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// **ROADMAP M6.** `stemma demo --out <file>` runs start to finish and writes a
/// document that reads as a language-family sketch.
#[test]
fn stemma_demo_runs_start_to_finish_and_has_the_expected_sections_and_forms() {
    let dest = tmp_dir("sections").join("demo.md");
    let out = stemma(&["demo", "--out", dest.to_str().unwrap()]);
    assert!(out.status.success(), "demo failed: {out:?}");

    let doc = std::fs::read_to_string(&dest).expect("the document was written");

    // The skeleton: title, three daughters, the cognate table, five etymologies.
    assert!(
        doc.starts_with("# Growing a Language Family in 90 Seconds\n"),
        "{doc}"
    );
    for daughter in ["Coastal Asterian", "Highland Asterian", "Riverine Asterian"] {
        assert!(
            doc.contains(&format!("### {daughter}")),
            "missing {daughter}"
        );
    }
    assert!(
        doc.contains("## Comparative cognate table"),
        "no table section"
    );
    assert!(
        doc.contains("## Five etymologies"),
        "no etymologies section"
    );

    // The exact star row of the cognate table (proto starred, real daughter forms).
    assert!(
        doc.contains("| star | *takala | taal | tagal | tala |"),
        "the star row drifted:\n{doc}"
    );
    // Five trace headings and their modern forms.
    for form in ["taal", "tala", "saŋk", "rean", "miala"] {
        assert!(doc.contains(form), "missing traced form `{form}`");
    }
}

/// **ROADMAP M6:** running it twice produces byte-identical output.
#[test]
fn stemma_demo_run_twice_produces_byte_identical_output() {
    let a = stdout_of(&stemma(&["demo"]));
    let b = stdout_of(&stemma(&["demo"]));
    assert_eq!(a, b, "two runs must be byte-identical");
    assert!(!a.is_empty(), "the demo produced a document");
}

/// `--out` and stdout carry the same bytes.
#[test]
fn stemma_demo_to_stdout_equals_the_out_file() {
    let dest = tmp_dir("passthrough").join("demo.md");
    stemma(&["demo", "--out", dest.to_str().unwrap()]);
    let file = std::fs::read_to_string(&dest).unwrap();
    let stdout = stdout_of(&stemma(&["demo"]));
    assert_eq!(file, stdout, "the file and stdout must match");
}

/// The anti-fabrication guard at the binary: real `tagal`, never the unreachable
/// `tazal` or an M9 drifted gloss.
#[test]
fn stemma_demo_never_fabricates_tazal_or_an_omen_reflex() {
    let doc = stdout_of(&stemma(&["demo"]));
    assert!(doc.contains("tagal"), "the real Highland form is present");
    for faked in ["tazal", "omen", "royal sign", "night-signal"] {
        assert!(!doc.contains(faked), "the demo fabricated `{faked}`");
    }
}

#[test]
fn the_demo_subcommand_is_wired_up() {
    let out = stemma(&["demo", "--help"]);
    assert!(out.status.success(), "demo --help failed");
    assert!(
        Path::new(STEMMA).exists(),
        "the binary is built for the acceptance run"
    );
}
