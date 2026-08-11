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
    for subcommand in ["validate", "info", "convert", "generate-roots", "features"] {
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

// ---------------------------------------------------------------------------
// M1 — root generation. These are the ROADMAP M1 acceptance tests.
// ---------------------------------------------------------------------------

/// stdout must be *only* the roots, one per line. Everything explanatory goes to
/// stderr, which is what makes a byte comparison of stdout an honest determinism
/// check rather than a check of the banner text.
fn generate(args: &[&str]) -> Output {
    let path = fixture("proto_asterian.ron");
    let mut full = vec!["generate-roots", path.to_str().unwrap()];
    full.extend_from_slice(args);
    stemma(&full)
}

#[test]
fn generate_roots_produces_the_requested_count() {
    let output = generate(&["--count", "100"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output).lines().count(), 100);
}

/// **The ROADMAP M1 acceptance test.** Runs the real binary twice and compares
/// raw stdout bytes.
#[test]
fn generate_roots_twice_with_the_same_seed_is_byte_identical() {
    let first = generate(&["--count", "100", "--seed", "42"]);
    let second = generate(&["--count", "100", "--seed", "42"]);
    assert!(first.status.success());
    assert_eq!(
        first.stdout, second.stdout,
        "the same seed must reproduce the same language byte for byte (DESIGN.md §9.4)"
    );
}

#[test]
fn generate_roots_with_a_different_seed_differs() {
    let a = generate(&["--count", "100", "--seed", "1"]);
    let b = generate(&["--count", "100", "--seed", "2"]);
    assert_ne!(a.stdout, b.stdout);
}

/// Seed precedence, pinned: with no `--seed`, the genome's own seed is used, so a
/// language always reproduces itself from the file alone.
#[test]
fn omitting_the_seed_flag_uses_the_seed_stored_in_the_language() {
    let implicit = generate(&["--count", "50"]);
    let explicit = generate(&["--count", "50", "--seed", "42"]);
    assert_eq!(
        implicit.stdout, explicit.stdout,
        "the fixture declares `seed: 42`, so these must agree"
    );

    let stderr = String::from_utf8_lossy(&implicit.stderr);
    assert!(stderr.contains("from the genome"), "{stderr}");
}

/// Pins the draw-order contract at the binary level: the generator is never
/// re-seeded per root, so asking for fewer is a byte-prefix of asking for more.
#[test]
fn generating_fewer_roots_is_a_prefix_of_generating_more() {
    let few = generate(&["--count", "20", "--seed", "5"]);
    let many = generate(&["--count", "100", "--seed", "5"]);

    let few_text = stdout(&few);
    let many_text = stdout(&many);
    let prefix: String = many_text
        .lines()
        .take(20)
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(few_text, prefix);
}

#[test]
fn every_generated_root_is_non_empty_and_uses_only_romanised_inventory_letters() {
    let output = generate(&["--count", "200", "--seed", "3"]);
    // The fixture's romanisations: /j/ writes as `y`, everything else as its IPA.
    let allowed: std::collections::BTreeSet<char> = "ptkmnslrwyaeiou".chars().collect();
    for root in stdout(&output).lines() {
        assert!(!root.is_empty(), "a generated root must not be empty");
        for c in root.chars() {
            assert!(allowed.contains(&c), "unexpected letter `{c}` in `{root}`");
        }
    }
}

#[test]
fn the_ipa_flag_switches_the_rendering() {
    // /j/ is the only segment whose romanisation differs from its IPA, so a corpus
    // large enough to contain one must render differently under `--ipa`.
    let romanised = stdout(&generate(&["--count", "300", "--seed", "0"]));
    let ipa = stdout(&generate(&["--count", "300", "--seed", "0", "--ipa"]));
    assert_ne!(romanised, ipa);
    assert!(romanised.contains('y'), "expected the romanisation of /j/");
    assert!(!ipa.contains('y'), "IPA output should use `j`, not `y`");
}

/// The corpus golden digest.
///
/// This ratifies already-verified behaviour: every behavioural test above was
/// green before this constant was filled in. Re-baselining it is a legitimate act
/// after a deliberate *fixture content* edit, and a red alarm after a dependency
/// change — the two are distinguishable because the data-free canaries in
/// `stem_core::rng` and `stem_phonology::generate` cannot be moved by any fixture
/// edit at all.
#[test]
fn the_reference_root_corpus_hashes_to_a_frozen_digest() {
    use sha2::{Digest, Sha256};

    let output = generate(&["--count", "500", "--seed", "0"]);
    assert!(output.status.success());

    let digest: [u8; 32] = Sha256::digest(&output.stdout).into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, "677f34134cb331aeabb515aa93655e50294c1bdd4cfc6e1ff5126ba2afd3075b",
        "the reference corpus changed; see the doc comment before re-baselining"
    );
}

#[test]
fn a_language_with_no_phonotactics_explains_why_it_cannot_generate() {
    let path = fixture("invalid_no_vowels.ron");
    let output = stemma(&["generate-roots", path.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no_templates"), "{stderr}");
}

#[test]
fn a_file_with_a_malformed_feature_list_fails_to_load_naming_the_token() {
    let path = fixture("invalid_features.ron");
    let output = stemma(&["validate", path.to_str().unwrap()]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("+voicee"),
        "the token must be named:\n{stderr}"
    );
    assert!(
        stderr.contains("voice"),
        "the suggestion must appear:\n{stderr}"
    );
}

#[test]
fn features_prints_the_resolved_matrix_for_every_segment() {
    let path = fixture("proto_asterian.ron");
    let output = stemma(&["features", path.to_str().unwrap()]);
    assert!(output.status.success());

    let text = stdout(&output);
    assert_eq!(text.lines().filter(|l| l.starts_with('/')).count(), 15);
    assert!(text.contains("+syllabic"), "{text}");
    // The glide case, visible in the tool a conlanger actually reads.
    assert!(
        text.lines()
            .any(|l| l.starts_with("/w/") && l.contains("-consonantal")),
        "{text}"
    );
}

#[test]
fn features_can_select_one_segment() {
    let path = fixture("proto_asterian.ron");
    let output = stemma(&["features", path.to_str().unwrap(), "--ipa", "k"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert_eq!(text.lines().filter(|l| l.starts_with('/')).count(), 1);
    assert!(text.contains("+dorsal"), "{text}");
}

/// Round-tripping the fixture through the serialiser must reach a fixpoint.
///
/// Compared as save(load(f)) vs save(load(save(load(f)))), **not** against the file
/// on disk: `stem_io::save` strips the fixture's comment header and hand-authored
/// grouping and prepends the RON extension header, none of which any serialiser
/// would reproduce. What matters is that canonical output is stable — which is
/// what proves feature ordering does not churn.
#[test]
fn the_reference_fixture_is_a_serialisation_fixpoint() {
    use stem_genome::LanguageGenome;
    use stem_io::Format;

    let once: LanguageGenome = stem_io::load(fixture("proto_asterian.ron")).expect("load");
    let text = stem_io::to_string(&once, Format::Ron).expect("serialise");

    let twice: LanguageGenome = stem_io::load_str(&text, Format::Ron).expect("reload");
    let text_again = stem_io::to_string(&twice, Format::Ron).expect("re-serialise");

    assert_eq!(once, twice, "a round trip must not change the value");
    assert_eq!(text, text_again, "canonical output must be stable");
}

// ---------------------------------------------------------------------------
// Regressions found by M1's adversarial review. Each of these reproduced a real
// defect before it was fixed; they exist so it cannot come back.
// ---------------------------------------------------------------------------

/// A vowel-only inventory is typologically unattested, so `validate` *warns* —
/// and CLAUDE.md's rule is that unusual designs are flagged, not rejected.
/// Generation must therefore succeed. It previously failed with a raw `rand`
/// string, because the consonant distribution was built even with no `C` slot.
#[test]
fn a_language_that_validate_only_warns_about_can_still_generate() {
    let dir = std::env::temp_dir().join("stemma_vowel_only");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vowels.ron");
    std::fs::write(
        &path,
        // Fully featured: `features_unspecified` is an Error since M3, and this
        // test's point is that a language `validate` merely WARNS about
        // (`no_consonants`) must still generate.
        r#"(
            id: "vowelish", name: "Vowelish", seed: 7,
            phonemes: [
                (id: "ph_a", ipa: "a", kind: vowel, frequency_weight: 60,
                 features: ["+syllabic", "-consonantal", "+sonorant", "+approximant", "+continuant", "-nasal", "-lateral", "-trill", "+voice", "-labial", "-coronal", "+dorsal", "-high", "+low", "+back", "-round"]),
                (id: "ph_i", ipa: "i", kind: vowel, frequency_weight: 40,
                 features: ["+syllabic", "-consonantal", "+sonorant", "+approximant", "+continuant", "-nasal", "-lateral", "-trill", "+voice", "-labial", "-coronal", "+dorsal", "+high", "-low", "-back", "-round"]),
            ],
            phonotactics: (
                templates: [(pattern: "V", weight: 60), (pattern: "VV", weight: 40)],
                syllables_per_root: [(count: 2, weight: 10)],
            ),
        )"#,
    )
    .unwrap();

    let validated = stemma(&["validate", path.to_str().unwrap()]);
    assert!(
        validated.status.success(),
        "warnings must not make a language invalid:\n{}",
        stdout(&validated)
    );

    let generated = stemma(&["generate-roots", path.to_str().unwrap(), "--count", "10"]);
    assert!(
        generated.status.success(),
        "a language `validate` accepts must generate:\nstdout: {}\nstderr: {}",
        stdout(&generated),
        String::from_utf8_lossy(&generated.stderr)
    );
    assert_eq!(stdout(&generated).lines().count(), 10);

    std::fs::remove_dir_all(&dir).ok();
}

/// A one-character typo in a field name previously took the field's default in
/// silence — measured, `frequency_wieght` on the most-frequent vowel changed 17 of
/// 20 generated roots with no diagnostic at any severity. That is exactly the
/// failure DESIGN.md §9.4 exists to prevent, so it must now refuse to load.
#[test]
fn a_misspelled_field_name_is_refused_rather_than_silently_defaulted() {
    let dir = std::env::temp_dir().join("stemma_field_typo");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("typo.ron");
    std::fs::write(
        &path,
        r#"(
            id: "typo", name: "Typo", seed: 1,
            phonemes: [
                (id: "ph_a", ipa: "a", kind: vowel, frequency_wieght: 60),
            ],
        )"#,
    )
    .unwrap();

    let output = stemma(&["validate", path.to_str().unwrap()]);
    assert!(!output.status.success(), "{}", stdout(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("frequency_wieght"),
        "the offending field must be named:\n{stderr}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Guards the `inventory.rs` convention at the *load* path: a bad template is a
/// reportable issue, not a parse error that aborts the load and hides everything
/// else wrong with the file.
#[test]
fn a_bad_template_loads_and_is_reported_rather_than_failing_the_load() {
    let path = fixture("invalid_template.ron");
    let output = stemma(&["validate", path.to_str().unwrap()]);
    let text = stdout(&output);

    assert!(!output.status.success());
    assert!(
        text.contains("phonotactics.bad_template"),
        "expected a validation issue, not a load failure:\n{text}"
    );
    // The rest of the file was still inspected — that is the whole point.
    assert!(text.contains("phonology.features_unspecified"), "{text}");
    assert!(
        text.contains("CV, CVC, V, VC"),
        "the message should point at the explicit form:\n{text}"
    );
}

/// An unbounded `--count` previously panicked with a capacity overflow (exit 101)
/// or aborted on allocation, because `Vec` pre-reserves from the size hint.
#[test]
fn an_absurd_count_is_a_message_rather_than_a_panic() {
    let output = generate(&["--count", "18446744073709551615"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("count"),
        "the error should name the offending flag:\n{stderr}"
    );
    assert_ne!(
        output.status.code(),
        Some(101),
        "101 means a panic; this should be a clean argument error"
    );
}

/// Every phoneme must be reachable, checked on the *reference* inventory rather
/// than a toy one — /j/ carries only 10 of 305 consonant weight, and a toy
/// inventory has no analogue of a share that small.
#[test]
fn every_reference_phoneme_appears_in_a_large_enough_sample() {
    let output = generate(&["--count", "20000", "--seed", "0", "--ipa"]);
    assert!(output.status.success());

    let corpus = stdout(&output);
    let seen: std::collections::BTreeSet<char> = corpus.chars().filter(|c| *c != '\n').collect();
    for symbol in "ptkmnslrwjaeiou".chars() {
        assert!(
            seen.contains(&symbol),
            "/{symbol}/ never appeared in 20,000 roots — it may be excluded from a \
             candidate vector"
        );
    }
}

// ---------------------------------------------------------------------------
// M2 — lexicon. These are the ROADMAP M2 acceptance tests.
// ---------------------------------------------------------------------------

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// **ROADMAP M2, half two.** Save a lexicon, load it back, compare.
#[test]
fn reloading_a_saved_lexicon_yields_an_identical_lexicon() {
    let dir = temp_dir("stemma_m2_reload");
    let saved = dir.join("proto.ron");

    let path = fixture("proto_asterian.ron");
    let output = stemma(&[
        "new-lexicon",
        path.to_str().unwrap(),
        "--out",
        saved.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let genome: stem_genome::LanguageGenome = stem_io::load(&saved).expect("load");
    assert_eq!(genome.lexicon.len(), stem_lexicon::CONCEPT_COUNT);

    // **ROADMAP M2, half one**: every entry has a stable id and cognate-set id.
    for entry in genome.lexicon.iter() {
        assert!(!entry.id.is_empty(), "every entry needs a WordId");
        assert!(
            !entry.cognate_set.is_empty(),
            "every entry needs a CognateSetId"
        );
    }

    // Round-trip through the serialiser and back.
    let text = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
    let back: stem_genome::LanguageGenome = stem_io::load_str(&text, stem_io::Format::Ron).unwrap();
    assert_eq!(back.lexicon, genome.lexicon);

    std::fs::remove_dir_all(&dir).ok();
}

/// The saved file must regenerate its own lexicon from its own stored seed. This
/// is what catches a `--seed`/`--out` divergence, where the file would say
/// `seed: 42` while holding words drawn from stream 7.
#[test]
fn a_saved_lexicon_regenerates_from_its_own_file() {
    let dir = temp_dir("stemma_m2_selfseed");
    let saved = dir.join("proto.ron");

    let path = fixture("proto_asterian.ron");
    assert!(
        stemma(&[
            "new-lexicon",
            path.to_str().unwrap(),
            "--seed",
            "7",
            "--out",
            saved.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let genome: stem_genome::LanguageGenome = stem_io::load(&saved).expect("load");
    assert_eq!(genome.seed, 7, "the effective seed must be written back");

    // Rebuild using ONLY what the file says.
    let rebuilt = stem_lexicon::build_proto_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &stem_lexicon::meanings(&genome.concepts),
        genome.seed,
    )
    .expect("rebuild");
    assert_eq!(rebuilt, genome.lexicon, "the file must reproduce itself");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn running_new_lexicon_twice_produces_byte_identical_stdout() {
    let path = fixture("proto_asterian.ron");
    let run = || stemma(&["new-lexicon", path.to_str().unwrap()]).stdout;
    assert_eq!(run(), run());
}

#[test]
fn regenerating_a_lexicon_replaces_rather_than_appends() {
    let dir = temp_dir("stemma_m2_replace");
    let once = dir.join("a.ron");
    let twice = dir.join("b.ron");
    let path = fixture("proto_asterian.ron");

    assert!(
        stemma(&[
            "new-lexicon",
            path.to_str().unwrap(),
            "--out",
            once.to_str().unwrap()
        ])
        .status
        .success()
    );
    let output = stemma(&[
        "new-lexicon",
        once.to_str().unwrap(),
        "--out",
        twice.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("replacing"),
        "a replacement should say so"
    );

    let genome: stem_genome::LanguageGenome = stem_io::load(&twice).expect("load");
    assert_eq!(
        genome.lexicon.len(),
        stem_lexicon::CONCEPT_COUNT,
        "appending would have doubled it"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_reference_fixture_still_loads_with_no_lexicon() {
    let path = fixture("proto_asterian.ron");
    let genome: stem_genome::LanguageGenome = stem_io::load(&path).expect("load");
    assert!(
        genome.lexicon.is_empty(),
        "the fixture is a proto definition, not a generated language"
    );
    assert!(
        stemma(&["validate", path.to_str().unwrap()])
            .status
            .success()
    );
}

/// Adding the `lexicon` field must not strand a file that never had one:
/// `skip_serializing_if` keeps the round trip byte-identical.
#[test]
fn converting_the_reference_fixture_stays_byte_identical() {
    let genome: stem_genome::LanguageGenome =
        stem_io::load(fixture("proto_asterian.ron")).expect("load");
    let once = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
    assert!(
        !once.contains("lexicon"),
        "an empty lexicon must not be written into a file that had none:\n{once}"
    );

    let reloaded: stem_genome::LanguageGenome =
        stem_io::load_str(&once, stem_io::Format::Ron).expect("reload");
    let twice = stem_io::to_string(&reloaded, stem_io::Format::Ron).expect("re-serialise");
    assert_eq!(once, twice);
}

#[test]
fn export_md_writes_a_readable_dictionary() {
    let dir = temp_dir("stemma_m2_md");
    let saved = dir.join("proto.ron");
    let path = fixture("proto_asterian.ron");
    assert!(
        stemma(&[
            "new-lexicon",
            path.to_str().unwrap(),
            "--out",
            saved.to_str().unwrap()
        ])
        .status
        .success()
    );

    let output = stemma(&["export-md", saved.to_str().unwrap()]);
    assert!(output.status.success());
    let document = stdout(&output);

    assert!(
        document.starts_with("# Proto-Asterian — lexicon"),
        "{document}"
    );
    assert!(document.contains("| Form | IPA | Gloss | POS | Concept | Word | Cognate set |"));
    assert!(document.contains("## Etymology"));
    // A known headword from the concept list.
    assert!(
        document.contains("| star |"),
        "the dictionary should gloss `star`"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_csv_writes_one_row_per_entry_plus_a_header() {
    let dir = temp_dir("stemma_m2_csv");
    let saved = dir.join("proto.ron");
    let path = fixture("proto_asterian.ron");
    assert!(
        stemma(&[
            "new-lexicon",
            path.to_str().unwrap(),
            "--out",
            saved.to_str().unwrap()
        ])
        .status
        .success()
    );

    let output = stemma(&["export-csv", saved.to_str().unwrap()]);
    assert!(output.status.success());
    let table = stdout(&output);
    assert_eq!(table.lines().count(), stem_lexicon::CONCEPT_COUNT + 1);
    assert!(
        table.starts_with("\"ID\",\"Language_ID\",\"Parameter_ID\""),
        "{table}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Mirrors M1's `677f3413…` corpus canary: a frozen digest over the whole
/// generated lexicon, distinguishable from the data-free canaries in
/// `stem_export` because only a fixture edit can move this one.
#[test]
fn the_reference_lexicon_hashes_to_a_frozen_digest() {
    use sha2::{Digest, Sha256};

    let path = fixture("proto_asterian.ron");
    let output = stemma(&["new-lexicon", path.to_str().unwrap()]);
    assert!(output.status.success());

    let digest: [u8; 32] = Sha256::digest(&output.stdout).into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, "d16ba86130091d93e3455d2742037b6d199c5181710d15cc28f0f8b9ca508423",
        "the reference lexicon changed; see the doc comment before re-baselining"
    );
}
