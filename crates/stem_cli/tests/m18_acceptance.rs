//! The ROADMAP M18 acceptance tests: **`stemma say <lang> '<proposition>'` produces
//! a sentence; the same proposition through two daughters differs *because* their
//! profiles differ; the trace names every construction applied.**
//!
//! # The middle clause is the one that can be faked, so it is tested hardest
//!
//! "Two languages produce two sentences" is trivially satisfiable by two languages
//! with different words. The claim worth proving is that the difference is
//! **grammatical**: the fixtures share a phonology, a concept list and a seed, so
//! they coin *the same words*, and every difference in the output has to come from
//! the syntax profile. `the_same_words_come_out_in_a_different_order` pins that
//! directly by comparing the multisets of lexicon entries used.
//!
//! # And §3.3, which is the constraint the milestone is really about
//!
//! *Every sentence carries a record of the constructions that built it, exactly as a
//! word carries its derivation.* A sentence that came out in the right order with no
//! `clause` construction recorded is a bug even when the string is correct — the same
//! rule that makes an untraced sound change a bug.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, render_sentence, say};
use stem_syntax::Proposition;

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

/// A fixture with its lexicon coined — what `stemma new-lexicon` writes.
fn coined(name: &str) -> LanguageGenome {
    let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
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

fn head_final() -> LanguageGenome {
    coined("grammar_asterian.ron")
}

fn head_initial() -> LanguageGenome {
    coined("grammar_svo_asterian.ron")
}

fn utter(genome: &LanguageGenome, text: &str) -> String {
    let proposition = Proposition::parse(text).expect("parses");
    let sentence = say(genome, &proposition).expect("generates");
    sentence.written(&genome.phonemes).expect("renders")
}

// ---------------------------------------------------- half one: a sentence exists

/// **ROADMAP M18, half one.** There is a sentence.
#[test]
fn the_say_command_produces_a_sentence() {
    let dir = std::env::temp_dir().join("stemma_m18_say");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let language = dir.join("sov.ron");

    assert!(
        stemma(&[
            "new-lexicon",
            fixture("grammar_asterian.ron").to_str().unwrap(),
            "--out",
            language.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let output = stemma(&["say", language.to_str().unwrap(), "SEE(KING, STAR)"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);

    // The sentence itself, on the first line, so a pipeline can take it alone.
    let sentence = text.lines().next().expect("a first line");
    assert_eq!(
        sentence.split_whitespace().count(),
        3,
        "agent, patient, verb: {sentence}"
    );
    // And it is made of real words with real ids, not invented forms.
    assert!(text.contains("w_0"), "{text}");
    assert!(text.contains("cog_asterian_grammar_"), "{text}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn saying_the_same_thing_twice_is_byte_identical() {
    let genome = head_final();
    assert_eq!(
        utter(&genome, "SEE(KING, STAR)"),
        utter(&genome, "SEE(KING, STAR)")
    );
}

/// The words are the language's own, with their own histories — the point of
/// generating from a lexicon rather than from thin air.
#[test]
fn every_word_of_a_sentence_is_a_real_lexicon_entry() {
    let genome = head_final();
    let proposition = Proposition::parse("SEE(KING:BIG/PRIEST, STAR)").expect("parses");
    let sentence = say(&genome, &proposition).expect("generates");

    assert_eq!(sentence.slots.len(), 5);
    for slot in &sentence.slots {
        let entry = genome
            .lexicon
            .get(&slot.word)
            .unwrap_or_else(|| panic!("`{}` is not in the lexicon", slot.word));
        assert_eq!(
            slot.cognate_set, entry.cognate_set,
            "a slot's echoed descent class must match its source"
        );
    }
}

// -------------------------------- half two: two profiles, two sentences, one cause

/// **ROADMAP M18, half two.** The same proposition through two languages differs.
#[test]
fn one_proposition_through_two_languages_gives_two_sentences() {
    let a = head_final();
    let b = head_initial();
    assert_ne!(
        utter(&a, "SEE(KING:BIG, STAR/PRIEST)"),
        utter(&b, "SEE(KING:BIG, STAR/PRIEST)")
    );
}

/// **…and the difference is the *profiles'*.** The two fixtures share a phonology, a
/// concept list and a seed, so they coin identical words — every difference in the
/// output is therefore grammatical, which is the claim the milestone actually makes.
#[test]
fn the_same_words_come_out_in_a_different_order() {
    let a = head_final();
    let b = head_initial();

    // The controlled variables.
    assert_eq!(a.phonemes, b.phonemes, "one phonology");
    assert_eq!(a.seed, b.seed, "one seed");
    assert_ne!(a.syntax, b.syntax, "the grammars had better differ");

    let proposition = Proposition::parse("SEE(KING:BIG, STAR/PRIEST)").expect("parses");
    let first = say(&a, &proposition).expect("generates");
    let second = say(&b, &proposition).expect("generates");

    // The same five lexicon entries, in both — compared as a sorted multiset, since
    // the whole point is that their *order* is what changed.
    let ids = |s: &stem_syntax::Sentence| {
        let mut ids: Vec<String> = s.slots.iter().map(|x| x.word.to_string()).collect();
        ids.sort();
        ids
    };
    assert_eq!(
        ids(&first),
        ids(&second),
        "the two languages must be using the same words"
    );

    // But in a different order, with the modifiers on different sides.
    let roles = |s: &stem_syntax::Sentence| s.slots.iter().map(|x| x.role).collect::<Vec<_>>();
    assert_eq!(
        roles(&first),
        ["agent", "adjective", "possessor", "patient", "predicate"],
        "SOV, noun-adjective, genitive-noun"
    );
    assert_eq!(
        roles(&second),
        ["adjective", "agent", "predicate", "patient", "possessor"],
        "SVO, adjective-noun, noun-genitive"
    );
}

/// **The sharpest of the four differences, because it is not a reordering.**
///
/// An ergative language marks the agent of a *transitive* clause and leaves the
/// argument of an intransitive one absolutive; a nominative language marks both the
/// same. So the very same word takes two different suffixes in one language and one
/// suffix in the other — a difference about grammar, not about order.
#[test]
fn alignment_makes_one_word_take_two_endings_in_one_language_and_one_in_the_other() {
    let ergative = head_final();
    let nominative = head_initial();

    let transitive_agent = |g: &LanguageGenome| {
        utter(g, "SEE(KING, STAR)")
            .split_whitespace()
            .next()
            .expect("a first word")
            .to_owned()
    };
    let intransitive_agent = |g: &LanguageGenome| {
        let sentence = utter(g, "SEE(KING)");
        sentence
            .split_whitespace()
            .find(|w| w != &"ponti")
            .expect("the argument")
            .to_owned()
    };

    assert_ne!(
        transitive_agent(&ergative),
        intransitive_agent(&ergative),
        "an ergative language marks the transitive agent differently from the \
         intransitive argument — that is what `ergative` means"
    );
    assert_eq!(
        transitive_agent(&nominative),
        intransitive_agent(&nominative),
        "a nominative language marks them the same — that is what `nominative` means"
    );
}

// -------------------------------------------- half three: §3.3, the record it left

/// **ROADMAP M18, half three.** The trace names every construction applied, and each
/// one names the syntax parameter that decided it.
#[test]
fn the_trace_names_every_construction_and_the_parameter_behind_it() {
    let genome = head_final();
    let proposition = Proposition::parse("SEE(KING:BIG/PRIEST, STAR)").expect("parses");
    let sentence = say(&genome, &proposition).expect("generates");

    let ids: Vec<&str> = sentence.constructions.iter().map(|c| c.id).collect();
    assert!(ids.contains(&"noun_phrase"), "{ids:?}");
    assert!(ids.contains(&"case_marking"), "{ids:?}");
    assert!(ids.contains(&"clause"), "{ids:?}");

    for construction in &sentence.constructions {
        assert!(
            !construction.effect.is_empty() && !construction.because.is_empty(),
            "`{}` recorded no reason",
            construction.id
        );
    }

    // The rendered form of the record — the same shape §10.2's word trace has, one
    // level up: a line per step, each saying what and why.
    let text = render_sentence(&genome, &sentence).expect("renders");
    assert!(text.contains("Constructions:"), "{text}");
    assert!(text.contains("because word_order = SOV"), "{text}");
    assert!(
        text.contains("because alignment = ergative-absolutive"),
        "{text}"
    );
}

/// A construction record that could be reconstructed from the output would be
/// decoration. This one names decisions the string does not show — *which parameter*
/// put the verb last, not merely that it is last.
#[test]
fn the_record_names_parameters_the_string_alone_does_not_reveal() {
    let genome = head_final();
    let sentence = say(
        &genome,
        &Proposition::parse("SEE(KING, STAR)").expect("parses"),
    )
    .expect("generates");
    let clause = sentence
        .constructions
        .iter()
        .find(|c| c.id == "clause")
        .expect("a clause construction");
    assert!(
        clause.because.starts_with("word_order ="),
        "{}",
        clause.because
    );
}

// ------------------------------------------------------------------ honest gaps

/// A language with no case morphemes still gets a sentence — unmarked, with the gap
/// **stated**. Faking an affix would be the fabrication this project exists to avoid.
#[test]
fn a_missing_case_marker_is_reported_and_the_sentence_still_comes_out() {
    let mut genome = head_final();
    genome.morphology.morphemes.clear();

    let sentence = say(
        &genome,
        &Proposition::parse("SEE(KING, STAR)").expect("parses"),
    )
    .expect("still generates");

    assert_eq!(sentence.slots.len(), 3, "nothing was dropped");
    assert!(!sentence.gaps.is_empty(), "the gap must be stated");
    assert!(
        sentence.gaps.iter().any(|g| g.contains("ERG")),
        "{:?}",
        sentence.gaps
    );

    let text = render_sentence(&genome, &sentence).expect("renders");
    assert!(text.contains("Not done:"), "{text}");
}

/// A concept the lexicon has no word for is an error naming it. There is no sentence
/// to be had, and coining one on the spot would be inventing vocabulary.
#[test]
fn a_concept_with_no_word_is_an_error_that_names_it() {
    let genome = head_final();
    let output = stemma(&["say", "-", "SEE(KING, STAR)"]);
    let _ = output; // the path check below is the real one

    let err = say(
        &genome,
        &Proposition::parse("SEE(KING, DRAGON)").expect("parses"),
    )
    .expect_err("no word for DRAGON");
    assert!(err.to_string().contains("DRAGON"), "{err}");
}

/// A malformed proposition is refused by the command, with what was expected.
#[test]
fn a_malformed_proposition_fails_the_command_and_says_why() {
    let dir = std::env::temp_dir().join("stemma_m18_bad");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let language = dir.join("sov.ron");
    assert!(
        stemma(&[
            "new-lexicon",
            fixture("grammar_asterian.ron").to_str().unwrap(),
            "--out",
            language.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let output = stemma(&["say", language.to_str().unwrap(), "SEE KING STAR"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no `(` found"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}

// ------------------------------------------------------------ nothing else moved

/// A language that has no grammar can still be asked to speak — it falls back
/// visibly, rather than refusing or pretending.
#[test]
fn a_language_with_no_syntax_profile_still_says_something_and_admits_the_fallback() {
    let mut genome = head_final();
    genome.syntax = Default::default();

    let sentence = say(
        &genome,
        &Proposition::parse("SEE(KING, STAR)").expect("parses"),
    )
    .expect("generates");
    assert_eq!(sentence.slots.len(), 3);

    let clause = sentence
        .constructions
        .iter()
        .find(|c| c.id == "clause")
        .expect("a clause construction");
    assert!(
        clause.because.contains("not stated"),
        "the fallback must be visible: {}",
        clause.because
    );
}

/// M18 added no genome field, so nothing on disk changed. The pair validates and the
/// pre-M18 fixtures are untouched.
#[test]
fn the_fixture_pair_validates_and_nothing_else_moved() {
    for name in ["grammar_asterian.ron", "grammar_svo_asterian.ron"] {
        let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
        let report = genome.validate();
        assert!(report.errors().next().is_none(), "{name}: {report}");
    }

    let proto: LanguageGenome = stem_io::load(fixture("proto_asterian.ron")).expect("loads");
    let text = stem_io::to_string(&proto, stem_io::Format::Ron).expect("serialise");
    assert!(!text.contains("syntax"), "M18 stores nothing new");
}
