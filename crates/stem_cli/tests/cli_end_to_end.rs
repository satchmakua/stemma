//! End-to-end tests: run the real `stemma` binary against the real fixtures.
//!
//! These are the M0 acceptance tests. Unit tests prove the pieces; this proves
//! the wiring — a file on disk is parsed, validated, and reported on, by the
//! binary a user would actually invoke.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The `stemma` binary cargo just built for this test run.
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn help_lists_the_subcommands() {
    let output = stemma(&["--help"]);
    assert!(output.status.success(), "`stemma --help` should exit 0");

    let text = stdout(&output);
    for subcommand in ["validate", "info", "convert"] {
        assert!(
            text.contains(subcommand),
            "`--help` should mention `{subcommand}`:\n{text}"
        );
    }
}

#[test]
fn version_is_reported() {
    let output = stemma(&["--version"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn the_reference_proto_language_loads_and_validates_cleanly() {
    let path = fixture("proto_asterian.ron");
    let output = stemma(&["validate", path.to_str().unwrap()]);
    let text = stdout(&output);

    assert!(
        output.status.success(),
        "the reference fixture must stay valid:\n{text}"
    );
    assert!(text.contains("Proto-Asterian"), "{text}");
    assert!(
        text.contains("10C/5V"),
        "expected the inventory split:\n{text}"
    );
}

#[test]
fn a_broken_language_fails_and_reports_every_fault_at_once() {
    let path = fixture("invalid_no_vowels.ron");
    let output = stemma(&["validate", path.to_str().unwrap()]);
    let text = stdout(&output);

    assert!(
        !output.status.success(),
        "a broken language must exit non-zero:\n{text}"
    );
    for code in [
        "phonology.no_nucleus",
        "phonology.duplicate_ipa",
        "phonology.bad_weight",
    ] {
        assert!(
            text.contains(code),
            "expected `{code}` in one pass:\n{text}"
        );
    }
}

#[test]
fn info_prints_the_inventory() {
    let path = fixture("proto_asterian.ron");
    let output = stemma(&["info", path.to_str().unwrap()]);
    let text = stdout(&output);

    assert!(output.status.success(), "{text}");
    assert!(text.contains("Proto-Asterian"), "{text}");
    assert!(text.contains("/t/"), "{text}");
    assert!(text.contains("/a/"), "{text}");
    assert!(text.contains("seed"), "{text}");
}

#[test]
fn a_language_survives_a_round_trip_through_json() {
    // RON is the authored format and JSON the interchange format; a project must
    // come back unchanged from the trip (`DESIGN.md` §19.2).
    let dir = std::env::temp_dir().join("stemma_cli_roundtrip");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();

    let source = fixture("proto_asterian.ron");
    let as_json = dir.join("proto.json");
    let back_to_ron = dir.join("proto.ron");

    assert!(
        stemma(&[
            "convert",
            source.to_str().unwrap(),
            as_json.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(
        stemma(&[
            "convert",
            as_json.to_str().unwrap(),
            back_to_ron.to_str().unwrap()
        ])
        .status
        .success()
    );

    // Compare the parsed values, not the bytes: the fixture has comments and
    // hand-authored spacing that no serialiser would reproduce.
    let original: stem_genome::LanguageGenome = stem_io::load(&source).unwrap();
    let round_tripped: stem_genome::LanguageGenome = stem_io::load(&back_to_ron).unwrap();
    assert_eq!(original, round_tripped);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unreadable_file_fails_with_a_message_naming_it() {
    let output = stemma(&["validate", "definitely/not/here.ron"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("definitely/not/here.ron"), "{stderr}");
}

#[test]
fn an_unsupported_extension_is_rejected() {
    let output = stemma(&["validate", "language.txt"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ron"),
        "the error should say what is supported:\n{stderr}"
    );
}
