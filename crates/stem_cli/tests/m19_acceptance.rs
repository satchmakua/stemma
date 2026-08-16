//! The ROADMAP M19 acceptance tests: **a daughter whose case suffixes were eroded by
//! an *ordered sound change* shows stricter word order in its profile, and the causal
//! chain from the rule to the syntactic shift is on the record — not asserted by the
//! author.**
//!
//! # The last clause is the milestone
//!
//! Everything before "not asserted by the author" is easy to fake: an author could
//! write "at year 640, order becomes SOV" and the profile would change. What makes
//! this grammaticalization rather than bookkeeping is that the program **checked**.
//!
//! So the tests below attack that clause from three sides:
//!
//! - `the_shift_is_refused_while_the_case_marker_still_survives` — the same change
//!   set applied to the *un-eroded* language does nothing, and says which word still
//!   carries the ending. If the engine were not really measuring, this would pass.
//! - `the_rule_that_caused_the_shift_is_named_by_the_engine` — the recorded `RuleId`
//!   appears nowhere in the change file, and is one of the rules that actually ran.
//! - `reordering_the_sound_changes_stops_the_shift_from_firing` — swap the two rules
//!   and the ergative *survives*, so the shift is refused. The causal chain is real
//!   enough to break when its cause is removed.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, SyntacticChangeSet, Trigger, apply_shifts, check};
use stem_syntax::{Alignment, WordOrder};

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

fn changes() -> SyntacticChangeSet {
    stem_io::load(fixture("shift_asterian.ron")).expect("the change set loads")
}

/// The erosion rules, read through M10's DSL parser — the `.sc` file is the fixture,
/// so the test exercises the same front end the CLI does.
fn rules() -> stem_soundchange::RuleSet {
    let path = fixture("rules_case_erosion.sc");
    let source = std::fs::read_to_string(&path).expect("the rule file is readable");
    stem_soundchange::parse_rule_set(&source, &path.display().to_string())
        .expect("the rule set parses")
}

/// Old Asterian with its vocabulary coined — free order, ergative case, intact.
fn old() -> LanguageGenome {
    let genome: LanguageGenome =
        stem_io::load(fixture("grammar_free_asterian.ron")).expect("loads");
    let lexicon = stem_lexicon::build_shaped_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &stem_lexicon::meanings(&genome.concepts),
        &genome.environment,
        genome.seed,
    )
    .expect("coins");
    genome.with_lexicon(lexicon)
}

/// The same language after the two ordered sound changes have run.
fn middle() -> LanguageGenome {
    let (evolved, _) = old()
        .evolve("middle", "Middle Asterian", &rules(), 600)
        .expect("evolves");
    evolved
}

// ------------------------------------------------ half one: the shift does happen

/// **ROADMAP M19's acceptance.** After the erosion, the daughter's profile has
/// stricter word order — and it got there through the ordinary pipeline.
#[test]
fn erosion_by_an_ordered_sound_change_fixes_the_word_order() {
    let before = old();
    assert_eq!(before.syntax.word_order, WordOrder::Free);
    assert_eq!(before.syntax.alignment, Alignment::ErgativeAbsolutive);

    let (after, report) =
        apply_shifts(&middle(), &changes(), "modern", "Modern Asterian", 60).expect("shifts");

    assert_eq!(
        after.syntax.word_order,
        WordOrder::Sov,
        "free order is no longer recoverable, so position took the load"
    );
    assert_eq!(after.syntax.alignment, Alignment::Neutral);
    assert!(
        report.issues.iter().any(|i| i.code == "shift_applied"),
        "{report}"
    );
    assert!(after.validate().errors().next().is_none());
}

/// The shift is a lineage step like any other: a daughter with a parent edge, and the
/// record carried forward.
#[test]
fn a_shift_produces_an_ordinary_daughter() {
    let parent = middle();
    let (daughter, _) =
        apply_shifts(&parent, &changes(), "modern", "Modern Asterian", 60).expect("shifts");

    assert_eq!(daughter.parent.as_ref(), Some(&parent.id));
    assert_eq!(
        daughter.lineage_depth_years,
        parent.lineage_depth_years + 60
    );
    // Forms and identities are untouched — a syntactic change is not a sound change.
    assert_eq!(daughter.lexicon, parent.lexicon);
    assert_eq!(daughter.applied_rules, parent.applied_rules);
}

// ------------------------------------- half two: the chain is a finding, not a claim

/// **The clause the milestone is about.** The rule named in the record appears
/// nowhere in the change file — the engine found it by replaying the language's own
/// derivations.
#[test]
fn the_rule_that_caused_the_shift_is_named_by_the_engine() {
    let (after, _) =
        apply_shifts(&middle(), &changes(), "modern", "Modern Asterian", 60).expect("shifts");

    assert_eq!(after.applied_shifts.len(), 2);
    let shift = &after.applied_shifts[0];
    let rule = shift
        .caused_by
        .as_ref()
        .expect("the cause must be recorded, or this is bookkeeping not grammaticalization");

    // It is one of the rules that actually ran.
    assert!(
        after.applied_rules.iter().any(|r| &r.id == rule),
        "`{rule}` is not in this language's recorded history"
    );

    // And it is *not* in the change file: the author never wrote it down.
    let change_file = std::fs::read_to_string(fixture("shift_asterian.ron")).expect("readable");
    assert!(
        !change_file.contains(rule.as_str()),
        "`{rule}` appears in the change set, so the chain was asserted rather than found"
    );
}

/// **The test that would pass if the engine were faking it.** Applied to the
/// *un-eroded* language, the very same change set must do nothing — and say which
/// word still carries the ending.
#[test]
fn the_shift_is_refused_while_the_case_marker_still_survives() {
    let (after, report) = apply_shifts(&old(), &changes(), "x", "Unchanged", 0).expect("runs");

    assert!(after.applied_shifts.is_empty(), "nothing may have applied");
    assert_eq!(
        after.syntax.word_order,
        WordOrder::Free,
        "the profile must be untouched"
    );

    let refusal = report
        .issues
        .iter()
        .find(|i| i.code == "shift_did_not_apply")
        .unwrap_or_else(|| panic!("{report}"));
    assert!(
        refusal.message.contains("still surfaces as `-ir`"),
        "the refusal must show the evidence: {}",
        refusal.message
    );
    // And the run as a whole is flagged, so a silent no-op is impossible to miss.
    assert!(
        report.warnings().any(|i| i.code == "no_shift_applied"),
        "{report}"
    );
}

/// **The sharpest version of the same claim.** Rule *order* is what destroys the
/// ergative: `-ir` ends in a rhotic, so the rhotic must go before word-final vowel
/// loss can reach the vowel behind it. Swap the two rules and the marker survives —
/// so the shift is refused.
///
/// This is M3's "rule order is observable" (`tag` vs `tak`) reaching all the way up
/// into the grammar.
#[test]
fn reordering_the_sound_changes_stops_the_shift_from_firing() {
    let forward = rules();
    assert_eq!(forward.rules.len(), 2);
    let swapped = stem_genome::move_rule(&forward, 1, 0).expect("in range");

    let (wrong_order, _) = old()
        .evolve("swapped", "Swapped", &swapped, 600)
        .expect("evolves");

    let finding = check(
        &wrong_order,
        &Trigger::CaseMarkerLost {
            morpheme: stem_core::MorphemeId::new("m_erg"),
        },
    )
    .expect("checks");
    assert!(
        !finding.holds,
        "with vowel loss first, the rhotic protects the ergative: {}",
        finding.evidence
    );

    // Which is exactly the difference: in the right order it *is* lost.
    let right = check(
        &middle(),
        &Trigger::CaseMarkerLost {
            morpheme: stem_core::MorphemeId::new("m_erg"),
        },
    )
    .expect("checks");
    assert!(right.holds, "{}", right.evidence);
}

/// The evidence is printed whether or not the condition held. "Why did this *not*
/// apply?" is M10's diagnostic discipline, and it is the question an author actually
/// has when nothing happened.
#[test]
fn a_finding_explains_itself_either_way() {
    let held = check(
        &middle(),
        &Trigger::CaseMarkerLost {
            morpheme: stem_core::MorphemeId::new("m_abs"),
        },
    )
    .expect("checks");
    assert!(held.holds);
    assert!(
        held.evidence.contains("surfaces on no word"),
        "{}",
        held.evidence
    );

    let refused = check(
        &old(),
        &Trigger::CaseMarkerLost {
            morpheme: stem_core::MorphemeId::new("m_abs"),
        },
    )
    .expect("checks");
    assert!(!refused.holds);
    assert!(
        refused.evidence.contains("still surfaces"),
        "{}",
        refused.evidence
    );
}

// ------------------------------------------------------------ the visible result

/// The point of a grammar changing is that the language changes. Same proposition,
/// two stages, two sentences — and this time the difference is *diachronic* rather
/// than a comparison of two designs (M18's pair).
#[test]
fn the_same_proposition_comes_out_differently_after_the_shift() {
    let before = old();
    let (after, _) =
        apply_shifts(&middle(), &changes(), "modern", "Modern Asterian", 60).expect("shifts");

    let utter = |g: &LanguageGenome| {
        let proposition = stem_syntax::Proposition::parse("SEE(KING, STAR)").expect("parses");
        stem_genome::say(g, &proposition)
            .expect("generates")
            .written(&g.phonemes)
            .expect("renders")
    };

    let old_sentence = utter(&before);
    let new_sentence = utter(&after);
    assert_ne!(old_sentence, new_sentence);
    assert!(
        old_sentence.contains("ir"),
        "the old stage marks its agent: {old_sentence}"
    );
    assert!(
        !new_sentence.contains("ir"),
        "the new one has nothing left to mark it with: {new_sentence}"
    );
}

// --------------------------------------------------------------------- the CLI

#[test]
fn the_shift_command_prints_the_causal_chain() {
    let dir = std::env::temp_dir().join("stemma_m19_cli");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let old_path = dir.join("old.ron");
    let middle_path = dir.join("middle.ron");
    let modern_path = dir.join("modern.ron");

    assert!(
        stemma(&[
            "new-lexicon",
            fixture("grammar_free_asterian.ron").to_str().unwrap(),
            "--out",
            old_path.to_str().unwrap(),
        ])
        .status
        .success()
    );
    assert!(
        stemma(&[
            "apply-rules",
            old_path.to_str().unwrap(),
            "--rules",
            fixture("rules_case_erosion.sc").to_str().unwrap(),
            "--id",
            "middle",
            "--name",
            "Middle Asterian",
            "--years",
            "600",
            "--out",
            middle_path.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let output = stemma(&[
        "shift",
        middle_path.to_str().unwrap(),
        "--changes",
        fixture("shift_asterian.ron").to_str().unwrap(),
        "--id",
        "modern",
        "--name",
        "Modern Asterian",
        "--years",
        "60",
        "--out",
        modern_path.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);
    assert!(text.contains("word order became SOV"), "{text}");
    assert!(text.contains("the cause is on the record"), "{text}");

    // And `stemma shifts` reads the record back off the saved file.
    let read_back = stdout(&stemma(&["shifts", modern_path.to_str().unwrap()]));
    assert!(read_back.contains("Syntactic history"), "{read_back}");
    assert!(read_back.contains("sound change `r_e02`"), "{read_back}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_language_with_no_recorded_shift_says_so() {
    let text = stdout(&stemma(&[
        "shifts",
        fixture("grammar_free_asterian.ron").to_str().unwrap(),
    ]));
    assert!(text.contains("No recorded syntactic change"), "{text}");
}

// ------------------------------------------------------------- the additive rule

/// Adding `applied_shifts` must not strand a file that never had one.
#[test]
fn a_pre_m19_fixture_round_trips_with_no_new_bytes() {
    for name in [
        "proto_asterian.ron",
        "asterian_attested.ron",
        "grammar_asterian.ron",
    ] {
        let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
        assert!(genome.applied_shifts.is_empty());
        let text = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
        assert!(
            !text.contains("applied_shifts"),
            "an empty history must not be written into `{name}`"
        );
    }
}

/// A shifted language carries its record through a fork, because that record *is*
/// its grammar's history.
#[test]
fn a_fork_carries_the_syntactic_history_verbatim() {
    let (modern, _) =
        apply_shifts(&middle(), &changes(), "modern", "Modern Asterian", 60).expect("shifts");
    let daughter = modern.fork("d", "Daughter", 50);
    assert_eq!(daughter.applied_shifts, modern.applied_shifts);
    assert_eq!(daughter.syntax, modern.syntax);
}

#[test]
fn shifting_twice_produces_an_identical_language() {
    let parent = middle();
    let a = apply_shifts(&parent, &changes(), "m", "M", 60)
        .expect("shifts")
        .0;
    let b = apply_shifts(&parent, &changes(), "m", "M", 60)
        .expect("shifts")
        .0;
    assert_eq!(a, b, "no RNG anywhere, so two runs are byte-identical");
}
