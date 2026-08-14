//! The ROADMAP M17 acceptance tests: **`stemma grammar <lang>` prints a readable
//! sketch from stored parameters; a harmonically odd combination earns a Warning and
//! still validates; two runs byte-identical.**
//!
//! # The milestone's real content is the third clause of its own description
//!
//! *Typological implications are reported, not enforced (§17) — a VO language with
//! postpositions is rare, not forbidden, and Stemma says which and why.*
//!
//! That is the hard part. A grammar profile is a dozen enums and an afternoon; a
//! grammar profile that tells an author their combination is uncommon **without
//! refusing it, and without inventing a statistic to sound authoritative** is the
//! thing worth testing. So the tests below check three properties in particular:
//! that no combination is ever an Error, that the odd one is reported, and that no
//! message quotes a number this program cannot support.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, render_grammar};
use stem_syntax::{AdpositionOrder, Headedness, SyntaxProfile, WordOrder};

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
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

fn load(name: &str) -> LanguageGenome {
    stem_io::load(fixture(name)).unwrap_or_else(|e| panic!("{name} loads: {e:?}"))
}

// --------------------------------------------------------- half one: the sketch

/// **ROADMAP M17, half one.** The command prints a readable sketch from the stored
/// parameters — every one of §7.4's, with a plain-English gloss.
#[test]
fn the_grammar_command_prints_a_readable_sketch_from_stored_parameters() {
    let output = stemma(&["grammar", fixture("grammar_asterian.ron").to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);

    for label in [
        "Word order",
        "Headedness",
        "Adpositions",
        "Genitive",
        "Adjective",
        "Alignment",
        "Relative clause",
        "Negation",
        "Questions",
        "Pro-drop",
        "Evidentiality",
        "Switch-reference",
    ] {
        assert!(text.contains(label), "`{label}` is missing:\n{text}");
    }

    // The values, and the glosses that make them legible to a reader who does not
    // already know what "ergative-absolutive" means.
    assert!(text.contains("SOV"), "{text}");
    assert!(text.contains("ergative-absolutive"), "{text}");
    assert!(
        text.contains("the one who walks patterns with the one who is hit"),
        "a sketch that printed enum names would be a data dump: {text}"
    );
}

/// Headedness is **derived** from the orders, never stored — so the fixture cannot
/// state one that disagrees with its own parameters, and the sketch says as much.
#[test]
fn headedness_is_derived_from_the_parameters_and_labelled_as_such() {
    let genome = load("grammar_asterian.ron");
    assert_eq!(genome.syntax.headedness(), Headedness::HeadFinal);

    let text = render_grammar(&genome).expect("renders");
    assert!(text.contains("head-final"), "{text}");
    assert!(
        text.contains("never stored"),
        "a reader must not mistake the summary for a thirteenth editable parameter: \
         {text}"
    );
}

/// **ROADMAP M17, half three.** Two runs byte-identical.
#[test]
fn two_runs_of_the_grammar_command_are_byte_identical() {
    let path = fixture("grammar_asterian.ron");
    let run = || stdout(&stemma(&["grammar", path.to_str().unwrap()]));
    assert_eq!(run(), run());
}

/// A language with no grammar says so, rather than printing an empty table or —
/// worse — a plausible-looking default.
#[test]
fn a_language_with_no_syntax_profile_says_so() {
    let output = stemma(&["grammar", fixture("proto_asterian.ron").to_str().unwrap()]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("No syntax profile"), "{text}");
    assert!(
        !text.contains("SVO"),
        "an undecided language must not be given the commonest answer: {text}"
    );
}

/// An unstated parameter is **printed**, not skipped. The same argument M15 made
/// about a missing word: a gap nothing shows is indistinguishable from a decision.
#[test]
fn an_unstated_parameter_still_gets_a_row() {
    let mut genome = load("grammar_asterian.ron");
    genome.syntax.evidentiality = stem_syntax::Evidentiality::Unspecified;
    let text = render_grammar(&genome).expect("renders");
    assert!(
        text.lines()
            .any(|l| l.contains("Evidentiality") && l.contains('—')),
        "{text}"
    );
}

// ---------------------------------------------- half two: reported, not enforced

/// **ROADMAP M17, half two.** A harmonically odd combination earns a Warning and
/// still validates — and the message says which way the tendency runs.
#[test]
fn a_disharmonic_combination_warns_and_still_validates() {
    let mut genome = load("grammar_asterian.ron");
    // Asterian is object–verb; prepositions are the rarer pairing.
    genome.syntax.adpositions = AdpositionOrder::Prepositions;

    let report = genome.syntax.validate();
    let issue = report
        .warnings()
        .find(|i| i.code == "ov_with_prepositions")
        .unwrap_or_else(|| panic!("{report}"));
    assert!(
        issue.message.contains("postpositions"),
        "the message must name the tendency, not just flag the fact: {}",
        issue.message
    );
    assert!(
        issue.message.contains("uncommon"),
        "and say it is rare rather than wrong: {}",
        issue.message
    );

    // Still a valid language, by the whole-genome gate.
    assert!(
        genome.validate().errors().next().is_none(),
        "§17: unusual designs get warnings, only structurally broken ones get errors"
    );

    // And the sketch prints it, with the posture stated in as many words.
    let text = render_grammar(&genome).expect("renders");
    assert!(text.contains("Notes on harmony"), "{text}");
    assert!(text.contains("does not refuse it"), "{text}");
}

/// The reference fixture is harmonic, so the sketch says so rather than staying
/// silent — a report that only ever speaks up is one a reader learns to ignore.
#[test]
fn the_harmonic_fixture_is_told_that_it_is() {
    let text = stdout(&stemma(&[
        "grammar",
        fixture("grammar_asterian.ron").to_str().unwrap(),
    ]));
    assert!(text.contains("Typologically harmonic"), "{text}");
}

/// **The constraint that governs this whole milestone.** `CLAUDE.md`: "Unusual
/// designs get warnings; only structurally broken ones get errors… Resist the urge
/// to reject a weird language."
///
/// Swept over every combination of the four parameters the harmony checks read.
#[test]
fn no_combination_of_syntactic_parameters_is_ever_an_error() {
    use stem_syntax::{Alignment, GenitiveOrder, RelativeClause};

    let mut checked = 0usize;
    for word_order in [
        WordOrder::Sov,
        WordOrder::Svo,
        WordOrder::Vso,
        WordOrder::Vos,
        WordOrder::Ovs,
        WordOrder::Osv,
        WordOrder::Free,
        WordOrder::Unspecified,
    ] {
        for adpositions in [
            AdpositionOrder::Prepositions,
            AdpositionOrder::Postpositions,
            AdpositionOrder::None,
            AdpositionOrder::Unspecified,
        ] {
            for genitive in [
                GenitiveOrder::GenitiveNoun,
                GenitiveOrder::NounGenitive,
                GenitiveOrder::Unspecified,
            ] {
                for relative_clause in [
                    RelativeClause::Prenominal,
                    RelativeClause::Postnominal,
                    RelativeClause::InternallyHeaded,
                    RelativeClause::Correlative,
                    RelativeClause::Unspecified,
                ] {
                    for alignment in [Alignment::Neutral, Alignment::ErgativeAbsolutive] {
                        let profile = SyntaxProfile {
                            word_order,
                            adpositions,
                            genitive,
                            relative_clause,
                            alignment,
                            ..SyntaxProfile::default()
                        };
                        let report = profile.validate();
                        assert!(
                            report.is_ok(),
                            "{word_order:?}/{adpositions:?}/{genitive:?}/\
                             {relative_clause:?} was refused:\n{report}"
                        );
                        // And every one of them renders, so no combination can crash
                        // the sketch either.
                        let genome = LanguageGenome {
                            syntax: profile,
                            ..LanguageGenome::proto("x", "X")
                        };
                        assert!(render_grammar(&genome).is_ok());
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 8 * 4 * 3 * 5 * 2);
}

/// **No fabricated statistics.** It would be easy to write "only 4% of languages"
/// into a harmony message and impossible to verify from inside this program — the
/// same rule that keeps invented Concepticon ids out of the concept list. A digit in
/// one of these messages is almost certainly a number nobody checked.
#[test]
fn no_harmony_message_quotes_a_frequency() {
    use stem_syntax::{GenitiveOrder, RelativeClause};

    let profile = SyntaxProfile {
        word_order: WordOrder::Vso,
        adpositions: AdpositionOrder::Postpositions,
        genitive: GenitiveOrder::GenitiveNoun,
        relative_clause: RelativeClause::Prenominal,
        ..SyntaxProfile::default()
    };
    let report = profile.validate();
    assert!(
        report.issues.len() >= 3,
        "this profile should trip several checks: {report}"
    );
    for issue in &report.issues {
        assert!(
            !issue.message.chars().any(|c| c.is_ascii_digit()),
            "`{}` quotes a number this program cannot verify: {}",
            issue.code,
            issue.message
        );
    }
}

// ------------------------------------------------------------- the additive rule

/// Adding `syntax` must not strand a file that never had one.
#[test]
fn a_pre_m17_fixture_round_trips_with_no_new_bytes() {
    for name in [
        "proto_asterian.ron",
        "asterian_attested.ron",
        "desert_asterian.ron",
    ] {
        let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
        assert!(genome.syntax.is_empty(), "{name} has no grammar yet");
        let text = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
        assert!(
            !text.contains("syntax"),
            "an empty profile must not be written into `{name}`"
        );
    }
}

/// A daughter inherits its parent's grammar. That is the truthful default — a
/// language does not lose its word order by being forked — and it is what M19's
/// syntactic change will act on.
#[test]
fn a_fork_carries_the_syntax_profile_verbatim() {
    let parent = load("grammar_asterian.ron");
    let daughter = parent.fork("d", "Daughter", 100);
    assert_eq!(daughter.syntax, parent.syntax);
}

/// The sketch and `stemma validate` read the *same* report, so they cannot disagree
/// about what is unusual in this language.
#[test]
fn the_sketch_and_the_validator_agree_about_what_is_unusual() {
    let mut genome = load("grammar_asterian.ron");
    genome.syntax.adpositions = AdpositionOrder::Prepositions;

    let sketch = render_grammar(&genome).expect("renders");
    for issue in genome.syntax.validate().issues {
        assert!(
            sketch.contains(&issue.message),
            "`{}` is in the report but not in the sketch:\n{sketch}",
            issue.code
        );
    }
}
