//! The ROADMAP M16 acceptance tests: **edit a gloss in the window, save, and
//! `stemma validate` on the file agrees; a rejected edit is reported and never
//! written; a file saved from the UI is byte-identical to one the equivalent CLI
//! command produces.**
//!
//! # What "from the UI" means in a test, precisely
//!
//! These tests do not drive an `egui` window — there is no headless harness for one
//! here, and a test that claimed to click a button while calling a function would be
//! worse than no test, because it would read as covering something it does not.
//!
//! What they drive is the **whole path the window uses**: build a
//! [`stem_genome::Edit`] value, hand it to `apply_edit`, write the result with
//! `stem_io::save`. That is the entirety of `App::run` and `App::save_selected` —
//! everything else in those two functions is a text box and a banner. So the
//! byte-identity claim is tested exactly where it can fail (two different code paths
//! producing two different files) and not where it cannot (whether a button was
//! pressed). `the_ui_computes_nothing_it_could_instead_ask_a_library_for` is what
//! keeps that equivalence honest: it fails if the window ever grows a second way to
//! change a language.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::{Validate, WordId};
use stem_genome::{Edit, LanguageGenome, apply_edit, move_rule};
use stem_lexicon::{ConceptKey, PartOfSpeech};

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

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn load(path: &Path) -> LanguageGenome {
    stem_io::load(path).unwrap_or_else(|e| panic!("{} loads: {e:?}", path.display()))
}

/// Exactly what `App::run` then `App::save_selected` do: one `Edit` through
/// `apply_edit`, then `stem_io::save` to the file it came from.
fn edit_as_the_window_would(source: &Path, destination: &Path, edit: &Edit) {
    let genome = load(source);
    let outcome = apply_edit(&genome, edit).expect("the window would have accepted this");
    stem_io::save(destination, &outcome.genome).expect("saves");
}

// ------------------------------------------------- half one: edit, save, validate

/// **ROADMAP M16, half one.** Edit a gloss, save, and the file is still a language
/// `stemma validate` is happy with — and the edit is actually in it.
#[test]
fn a_gloss_edited_and_saved_survives_the_round_trip_and_validates() {
    let dir = temp_dir("stemma_m16_gloss");
    let saved = dir.join("edited.ron");

    // `w_0002` MOON, not `w_0001`: the star word holds a modelled sense, and a
    // sense outranks an authored override in `display_gloss` (M9, deliberately —
    // otherwise a drifted meaning would be invisible). That interaction has its own
    // test below; this one is about the plain case.
    edit_as_the_window_would(
        &fixture("asterian_attested.ron"),
        &saved,
        &Edit::SetGloss {
            word: WordId::new("w_0002"),
            gloss: "the wandering light".to_owned(),
        },
    );

    // The edit is on disk, not merely in memory.
    let reloaded = load(&saved);
    assert_eq!(
        reloaded
            .lexicon
            .require(&WordId::new("w_0002"))
            .unwrap()
            .display_gloss(),
        Some("the wandering light")
    );

    // And the file is a language the ordinary gate accepts.
    let output = stemma(&["validate", saved.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "validate refused a file the editor produced:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(reloaded.validate().errors().next().is_none());

    std::fs::remove_dir_all(&dir).ok();
}

/// Nothing else moved. An editor that quietly renormalised the rest of the file
/// would make every edit an unreviewable diff.
#[test]
fn editing_one_word_leaves_every_other_byte_alone() {
    let dir = temp_dir("stemma_m16_minimal");
    let untouched = dir.join("copy.ron");
    let edited = dir.join("edited.ron");

    let source = fixture("asterian_attested.ron");
    let genome = load(&source);
    stem_io::save(&untouched, &genome).expect("saves");
    edit_as_the_window_would(
        &source,
        &edited,
        &Edit::SetGloss {
            word: WordId::new("w_0001"),
            gloss: "changed".to_owned(),
        },
    );

    // Compared as line **multisets**, not zipped positions: inserting three lines
    // shifts every line after them, so a positional diff would report the whole file
    // and prove nothing. What is actually claimed is that the edit *added* the gloss
    // block and *removed* nothing.
    let before = std::fs::read_to_string(&untouched).expect("read");
    let after = std::fs::read_to_string(&edited).expect("read");

    let mut removed: Vec<&str> = before.lines().collect();
    let mut added: Vec<&str> = Vec::new();
    for line in after.lines() {
        match removed.iter().position(|old| *old == line) {
            Some(i) => {
                removed.swap_remove(i);
            }
            None => added.push(line),
        }
    }
    assert!(
        removed.is_empty(),
        "a gloss edit removed lines it had no business touching: {removed:?}"
    );
    // Sorted, because a multiset difference has no meaningful order: the file
    // already contains `glosses: [` (on `w_0005`, glossed "king"), so the greedy
    // match pairs the new block's opener with the old one and reports the *old*
    // one as added. Which of two identical lines is "the new one" is not a question
    // with an answer, and asserting on it would be asserting on the matcher.
    let mut added: Vec<&str> = added.iter().map(|l| l.trim()).collect();
    added.sort_unstable();
    assert_eq!(
        added,
        ["\"changed\",", "],", "glosses: ["],
        "the only new lines should be the gloss block itself"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// **The trap M16 has to be honest about.** `display_gloss` prefers a modelled sense
/// over an authored override (M9, so a drifted meaning cannot be hidden by an
/// inherited label). So glossing a word that holds a sense *stores* the label and
/// *shows* nothing — which from a text box is indistinguishable from a failed edit.
///
/// The edit is accepted, the override really is written, and `gloss_shadowed_by_sense`
/// comes back in `introduced` so the window can say why nothing appeared to happen.
#[test]
fn glossing_a_word_that_holds_a_sense_is_accepted_and_says_the_sense_still_shows() {
    let genome = load(&fixture("asterian_attested.ron"));
    let outcome = apply_edit(
        &genome,
        &Edit::SetGloss {
            word: WordId::new("w_0001"),
            gloss: "the wandering light".to_owned(),
        },
    )
    .expect("accepted — the override is legitimate, it is just not what displays");

    let edited = outcome
        .genome
        .lexicon
        .require(&WordId::new("w_0001"))
        .unwrap();
    assert_eq!(
        edited.glosses,
        ["the wandering light"],
        "the override is stored"
    );
    assert_eq!(
        edited.display_gloss(),
        Some("star"),
        "and the sense is still what displays — M9's rule, unchanged"
    );
    assert!(
        outcome
            .introduced
            .iter()
            .any(|i| i.code.ends_with("gloss_shadowed_by_sense")),
        "the editor must explain the invisible change: {:?}",
        outcome.introduced
    );
}

// ------------------------------------------------- half two: refusal, never written

/// **ROADMAP M16, half two: an id collision.** The edit is refused, and — the part
/// that matters — the language it was applied to is unchanged, so a window holding it
/// cannot be showing a change the library said no to.
#[test]
fn an_edit_that_would_collide_an_id_is_refused_and_nothing_is_written() {
    let dir = temp_dir("stemma_m16_collision");
    let saved = dir.join("out.ron");

    // A lexicon whose ids are irregular, so `len + 1` lands on a taken id.
    let mut genome = load(&fixture("asterian_attested.ron"));
    let entries: Vec<stem_lexicon::WordEntry> = genome
        .lexicon
        .iter()
        .take(3)
        .cloned()
        .map(|mut e| {
            if e.id.as_str() == "w_0003" {
                e.id = WordId::new("w_0004");
            }
            e
        })
        .collect();
    genome.lexicon = stem_lexicon::Lexicon::from_entries(entries);
    let before = genome.clone();

    let err = apply_edit(
        &genome,
        &Edit::AddWord {
            form: "taka".to_owned(),
            gloss: "a new word".to_owned(),
            concept: None,
            part_of_speech: PartOfSpeech::Noun,
        },
    )
    .expect_err("w_0004 is taken");

    assert!(err.to_string().contains("duplicate_word_id"), "{err}");
    assert_eq!(
        genome, before,
        "a refused edit must not mutate the language"
    );
    assert!(!saved.exists(), "and nothing was written");

    // The CLI refuses it the same way, with a non-zero exit.
    let source = dir.join("irregular.ron");
    stem_io::save(&source, &genome).expect("saves");
    let output = stemma(&[
        "add-word",
        source.to_str().unwrap(),
        "--form",
        "taka",
        "--gloss",
        "a new word",
        "--out",
        saved.to_str().unwrap(),
    ]);
    assert!(!output.status.success(), "the CLI must refuse it too");
    assert!(!saved.exists(), "a refused edit writes no file");

    std::fs::remove_dir_all(&dir).ok();
}

/// **The other half of "a rejected edit": a bad form.** A word cannot be made of
/// sounds this language does not have, and the refusal names the offending text
/// rather than saying "invalid".
#[test]
fn a_word_using_a_sound_this_language_lacks_is_refused_and_says_which() {
    let genome = load(&fixture("asterian_attested.ron"));
    let err = apply_edit(
        &genome,
        &Edit::AddWord {
            // /z/ and /b/ are not in the Asterian inventory.
            form: "tazbo".to_owned(),
            gloss: "impossible".to_owned(),
            concept: None,
            part_of_speech: PartOfSpeech::Noun,
        },
    )
    .expect_err("no /z/ in Asterian");
    assert!(
        err.to_string().contains("zbo"),
        "the message must name what could not be read: {err}"
    );
}

#[test]
fn editing_a_word_that_is_not_there_names_the_id() {
    let genome = load(&fixture("asterian_attested.ron"));
    let err = apply_edit(
        &genome,
        &Edit::SetGloss {
            word: WordId::new("w_9999"),
            gloss: "x".to_owned(),
        },
    )
    .expect_err("no such word");
    assert!(err.to_string().contains("w_9999"), "{err}");
}

// --------------------------------------------- half three: the two front ends agree

/// **ROADMAP M16, half three.** A file saved through the window's path and a file
/// saved by the equivalent CLI command are byte-identical.
///
/// This is the claim the whole design is for: one `Edit` value, one `apply_edit`, so
/// there is no second implementation of what an edit means and therefore nothing for
/// the two front ends to disagree about.
#[test]
fn a_file_saved_from_the_ui_matches_the_equivalent_cli_command() {
    let dir = temp_dir("stemma_m16_parity");
    let source = fixture("asterian_attested.ron");

    let cases: Vec<(&str, Edit, Vec<String>)> = vec![
        (
            "gloss",
            Edit::SetGloss {
                word: WordId::new("w_0001"),
                gloss: "the wandering light".to_owned(),
            },
            vec![
                "set-gloss".into(),
                "{src}".into(),
                "w_0001".into(),
                "the wandering light".into(),
            ],
        ),
        (
            "add",
            Edit::AddWord {
                form: "takan".to_owned(),
                gloss: "a coined thing".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
            vec![
                "add-word".into(),
                "{src}".into(),
                "--form".into(),
                "takan".into(),
                "--gloss".into(),
                "a coined thing".into(),
            ],
        ),
        (
            "remove",
            Edit::RemoveWord {
                word: WordId::new("w_0002"),
            },
            vec!["remove-word".into(), "{src}".into(), "w_0002".into()],
        ),
        (
            "concept",
            Edit::DeclareConcept {
                key: ConceptKey::new("OBSIDIAN"),
                gloss: "black glass".to_owned(),
                part_of_speech: PartOfSpeech::Noun,
                note: String::new(),
            },
            vec![
                "declare-concept".into(),
                "{src}".into(),
                "--key".into(),
                "OBSIDIAN".into(),
                "--gloss".into(),
                "black glass".into(),
            ],
        ),
    ];

    for (name, edit, argv) in cases {
        let from_ui = dir.join(format!("{name}_ui.ron"));
        let from_cli = dir.join(format!("{name}_cli.ron"));

        edit_as_the_window_would(&source, &from_ui, &edit);

        let src = source.to_str().unwrap().to_owned();
        let out = from_cli.to_str().unwrap().to_owned();
        let mut args: Vec<String> = argv
            .into_iter()
            .map(|a| if a == "{src}" { src.clone() } else { a })
            .collect();
        args.push("--out".into());
        args.push(out);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = stemma(&borrowed);
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        assert_eq!(
            std::fs::read_to_string(&from_ui).expect("read"),
            std::fs::read_to_string(&from_cli).expect("read"),
            "`{name}`: the window and the CLI produced different files, which means \
             there are two implementations of what this edit does"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}

// ----------------------------------------------------------------- reordering rules

/// Rule order is chronology and it is observable, so reordering is a real edit —
/// and here is the proof that it changes the language it is applied to.
#[test]
fn reordering_a_rule_set_changes_what_the_engine_produces() {
    let dir = temp_dir("stemma_m16_reorder");
    let reordered = dir.join("reordered.ron");

    let rules: stem_soundchange::RuleSet =
        stem_io::load(fixture("rules_asterian.ron")).expect("rules load");
    assert!(rules.rules.len() >= 2, "need two rules to reorder");

    // Move the last rule first — the sharpest possible reordering.
    let last = rules.rules.len() - 1;
    let moved = move_rule(&rules, last, 0).expect("in range");
    assert_eq!(
        moved.rules[0].id, rules.rules[last].id,
        "a rotation puts it first"
    );
    assert_eq!(
        moved.rules.len(),
        rules.rules.len(),
        "and loses nothing on the way"
    );
    stem_io::save(&reordered, &moved).expect("saves");

    // Both orders applied to the same language give different lexicons — which is
    // exactly why this is an edit worth having, and why M3 made order observable.
    let proto = load(&fixture("asterian_attested.ron"));
    let (original, _) = proto
        .clone()
        .evolve("a", "A", &rules, 100)
        .expect("evolves");
    let (swapped, _) = proto.evolve("b", "B", &moved, 100).expect("evolves");
    assert_ne!(
        original.lexicon, swapped.lexicon,
        "if reordering changed nothing, the acceptance would be vacuous"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_reorder_rule_command_reports_the_new_order_and_refuses_a_bad_index() {
    let path = fixture("rules_asterian.ron");
    let output = stemma(&[
        "reorder-rule",
        path.to_str().unwrap(),
        "--from",
        "3",
        "--to",
        "0",
    ]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("moved rule 3 to 0"), "{text}");
    assert!(
        text.starts_with("moved") || text.contains("  0  "),
        "{text}"
    );

    let bad = stemma(&[
        "reorder-rule",
        path.to_str().unwrap(),
        "--from",
        "0",
        "--to",
        "99",
    ]);
    assert!(!bad.status.success(), "an out-of-range index must fail");
}

// ------------------------------------------------------------- undo is the file

/// **Nothing autosaves.** Without `--out` the CLI writes nothing at all, which is the
/// same contract the window's Save button has: an edit is held, not committed.
#[test]
fn an_edit_without_an_output_writes_nothing() {
    let dir = temp_dir("stemma_m16_dryrun");
    let source = dir.join("source.ron");
    let genome = load(&fixture("asterian_attested.ron"));
    stem_io::save(&source, &genome).expect("saves");
    let before = std::fs::read_to_string(&source).expect("read");

    let output = stemma(&[
        "set-gloss",
        source.to_str().unwrap(),
        "w_0001",
        "not written",
    ]);
    assert!(output.status.success());
    assert!(
        stdout(&output).contains("glossed `w_0001`"),
        "it should still say what it would do"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("nothing written"),
        "and say that it did not do it"
    );
    assert_eq!(
        std::fs::read_to_string(&source).expect("read"),
        before,
        "the source file must be untouched"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A warning must not refuse an edit — §17's posture, applied to editing. Declaring
/// a concept that shadows a built-in is odd, reported, and allowed.
#[test]
fn a_warning_is_shown_and_the_edit_is_still_accepted() {
    let genome = load(&fixture("asterian_attested.ron"));
    let outcome = apply_edit(
        &genome,
        &Edit::DeclareConcept {
            key: ConceptKey::new("STAR"),
            gloss: "a different star".to_owned(),
            part_of_speech: PartOfSpeech::Noun,
            note: String::new(),
        },
    )
    .expect("a warning must not refuse an edit");
    assert!(
        outcome
            .introduced
            .iter()
            .any(|i| i.code.ends_with("shadows_builtin")),
        "{:?}",
        outcome.introduced
    );
}

/// Adding a word to a real fixture and then tracing it must work: a hand-authored
/// word is an ordinary `WordEntry`, so every downstream command takes it.
#[test]
fn a_hand_added_word_flows_through_the_ordinary_commands() {
    let dir = temp_dir("stemma_m16_downstream");
    let saved = dir.join("edited.ron");

    edit_as_the_window_would(
        &fixture("asterian_attested.ron"),
        &saved,
        &Edit::AddWord {
            form: "kalan".to_owned(),
            gloss: "a hand-made word".to_owned(),
            concept: None,
            part_of_speech: PartOfSpeech::Noun,
        },
    );
    let genome = load(&saved);
    let added = genome.lexicon.iter().last().expect("appended");
    assert_eq!(added.written(&genome.phonemes).unwrap(), "kalan");

    // `export-md` renders it, and `trace-word` finds it by meaning.
    let md = stemma(&["export-md", saved.to_str().unwrap()]);
    assert!(md.status.success());
    assert!(stdout(&md).contains("a hand-made word"), "{}", stdout(&md));

    let traced = stemma(&["trace-word", saved.to_str().unwrap(), "a hand-made word"]);
    assert!(
        traced.status.success(),
        "{}",
        String::from_utf8_lossy(&traced.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}
