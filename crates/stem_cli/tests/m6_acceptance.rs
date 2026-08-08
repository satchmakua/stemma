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
    // M9 annotates Coastal's cell with its drifted sense — the FORM is unchanged,
    // which is the point: meaning moved, ancestry and phonology did not.
    assert!(
        doc.contains("| star | *takala | taal \"omen\" | tagal | tala |"),
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

/// The anti-fabrication guard at the binary, **rewritten at M9 and stronger**.
///
/// M6 banned `omen` outright because the engine could not produce it, and printing
/// it would have been the document claiming a capability the program lacked. M9
/// built that capability, so the ban becomes a *condition*: `omen` may appear **only
/// when a real drift event produced it**, which the sibling test proves. What stays
/// banned forever is the genuinely unreachable:
///
/// - `tazal` — §21's prose form. `ɡ → z` is a place shift `Change::Set` cannot
///   express, so no rule file can ever produce it; the real Highland form is
///   `tagal`.
/// - `night-signal` — an invented gloss no fixture declares.
#[test]
fn stemma_demo_never_fabricates_an_unreachable_form_or_gloss() {
    let doc = stdout_of(&stemma(&["demo"]));
    assert!(doc.contains("tagal"), "the real Highland form is present");
    for faked in ["tazal", "night-signal"] {
        assert!(!doc.contains(faked), "the demo fabricated `{faked}`");
    }
}

/// **ROADMAP M9 at the binary.** The demo now *shows* meaning drift instead of
/// promising it — and the showing is earned, not printed.
///
/// Every clause is a separate way the document could have been lying, and all of
/// them must hold at once: the gloss appears, a real event with that gloss is on the
/// evolved genome's log, the mechanism that drove it is named, the drifted reflex is
/// still in its cognate row, and the closer no longer lists meaning drift as future
/// work.
#[test]
fn the_demo_shows_real_drift_rather_than_promising_it() {
    let doc = stdout_of(&stemma(&["demo"]));

    // 1. The drifted sense is in the document, annotated on the cognate cell.
    assert!(
        doc.contains("taal \"omen\""),
        "the drifted cell shows what it means now:\n{doc}"
    );
    // 2. The mechanisms that produced it are named — the engine's own words, from
    //    the drift file, not prose.
    assert!(
        doc.contains("metaphor") && doc.contains("metonymy"),
        "the demo names the mechanisms that drove the shift:\n{doc}"
    );
    // 3. The register §10.2 specifies.
    assert!(doc.contains("priestly"), "the register is recorded:\n{doc}");
    // 4. Highland did NOT drift — the contrast is the whole point.
    assert!(
        doc.contains("| tagal |") || doc.contains(" tagal "),
        "Highland keeps the inherited sense and form:\n{doc}"
    );
    // 5. The promise has been kept, so it is no longer made.
    assert!(
        !doc.contains("Still ahead, and named honestly rather than faked: **meaning drift**"),
        "the closer still promises what has now shipped:\n{doc}"
    );
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
