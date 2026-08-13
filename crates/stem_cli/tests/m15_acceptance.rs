//! The ROADMAP M15 acceptance tests: **two languages over one phonology and one
//! concept list, given different profiles, differ in *which* meanings are elaborated
//! and which are missing — and each gap is explained in the report rather than
//! silently empty.**
//!
//! The two halves are load-bearing in different ways.
//!
//! `two_ecologies_over_one_phonology_differ_only_where_their_cultures_differ` is the
//! comparison: it coins both fixtures with the real builder and asserts that a
//! meaning both peoples have comes out as **the same word**, so every difference
//! between the two dictionaries is attributable to the culture profile and nothing
//! else. That property is what makes the milestone a demonstration instead of two
//! unrelated languages, and it is bought by the draw contract on
//! `build_shaped_lexicon` — absent concepts still draw and discard, elaborations
//! draw from their own stream.
//!
//! `every_gap_is_explained_rather_than_silently_empty` is the honesty half. A missing
//! word is invisible by construction, so a gap that nothing prints is
//! indistinguishable from an accident of which wordlist shipped — which is the exact
//! failure M13 chose breadth to avoid and M15 exists to replace with a claim.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, VocabularyShaping, absences, render_culture};
use stem_lexicon::{
    CONCEPT_COUNT, ConceptKey, LARGE_VOCABULARY_GAP, Lexicon, build_shaped_lexicon, meanings,
};

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

/// Coins a fixture's lexicon through the real shaped builder.
fn coin(genome: &LanguageGenome) -> Lexicon {
    build_shaped_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &meanings(&genome.concepts),
        &genome.environment,
        genome.seed,
    )
    .expect("coins")
}

/// Every word this lexicon has for `key`, as written forms in stored order.
fn words_for(genome: &LanguageGenome, lexicon: &Lexicon, key: &str) -> Vec<String> {
    lexicon
        .by_concept(&ConceptKey::new(key))
        .iter()
        .map(|e| e.written(&genome.phonemes).expect("renders"))
        .collect()
}

// ------------------------------------------------------- half one: the comparison

/// **ROADMAP M15, half one.** One phonology, one concept list, one seed, two
/// ecologies — and the difference between the dictionaries is *entirely* the
/// culture profile's doing.
#[test]
fn two_ecologies_over_one_phonology_differ_only_where_their_cultures_differ() {
    let desert = load("desert_asterian.ron");
    let sea = load("seafarer_asterian.ron");

    // The controlled variables: everything except the profile.
    assert_eq!(desert.phonemes, sea.phonemes, "one phonology");
    assert_eq!(
        desert.phonotactics, sea.phonotactics,
        "one set of root shapes"
    );
    assert_eq!(desert.seed, sea.seed, "one seed");
    assert_ne!(
        desert.environment, sea.environment,
        "the profile is the only thing that differs, so it had better differ"
    );

    let desert_lexicon = coin(&desert);
    let sea_lexicon = coin(&sea);

    // A meaning both peoples have comes out as THE SAME WORD. This is what makes
    // the comparison legible rather than two unrelated languages, and it holds only
    // because an absent concept still draws and discards its root.
    for shared in ["STAR", "SUN", "BLOOD", "MOTHER", "STONE"] {
        let a = words_for(&desert, &desert_lexicon, shared);
        let b = words_for(&sea, &sea_lexicon, shared);
        assert_eq!(a.len(), 1, "{shared} is ordinary in the desert language");
        assert_eq!(
            a, b,
            "`{shared}` must be one word in both, or nothing is controlled"
        );
    }
}

/// **ROADMAP M15, half one's real claim.** They differ in *which* meanings are
/// elaborated and which are missing — and the two lists are near-inversions, which
/// is the point: neither language is more complete, they are differently complete.
#[test]
fn the_two_cultures_elaborate_and_lack_opposite_meanings() {
    let desert = load("desert_asterian.ron");
    let sea = load("seafarer_asterian.ron");
    let desert_lexicon = coin(&desert);
    let sea_lexicon = coin(&sea);

    let counts = |g: &LanguageGenome, l: &Lexicon, k: &str| words_for(g, l, k).len();

    // The sea: four words for one people, none at all for the other.
    assert_eq!(counts(&sea, &sea_lexicon, "SEA"), 4);
    assert_eq!(counts(&desert, &desert_lexicon, "SEA"), 0);

    // And the reverse, for sand.
    assert_eq!(counts(&desert, &desert_lexicon, "SAND"), 4);
    assert_eq!(
        counts(&sea, &sea_lexicon, "SAND"),
        1,
        "ordinary, not absent"
    );

    // Livestock and fish, each way.
    assert_eq!(counts(&desert, &desert_lexicon, "CATTLE"), 4);
    assert_eq!(counts(&sea, &sea_lexicon, "CATTLE"), 0);
    assert_eq!(counts(&sea, &sea_lexicon, "FISH"), 3);
    assert_eq!(counts(&desert, &desert_lexicon, "FISH"), 0);

    // **The inversion that answers DESIGN §7.5.** That section worried about a
    // desert language being forced to coin a word for ice. Here the desert language
    // *has* one — its speakers trade north, which M12 recorded as the user's
    // counter-argument — and the tropical island language does not. The ecology
    // decides, and the decision is on the record either way.
    assert_eq!(
        counts(&desert, &desert_lexicon, "ICE"),
        1,
        "they trade north; that is exactly why the word exists"
    );
    assert_eq!(
        counts(&sea, &sea_lexicon, "ICE"),
        0,
        "no snow and no northern trade — and `stemma culture` says so"
    );
}

/// An elaborated meaning is coined once per **named** sense, with the author's
/// distinction on each word — not N copies of one gloss.
#[test]
fn an_elaborated_meaning_carries_the_distinctions_the_author_named() {
    let desert = load("desert_asterian.ron");
    let lexicon = coin(&desert);
    let sands = lexicon.by_concept(&ConceptKey::new("SAND"));
    assert_eq!(sands.len(), 4);

    let glosses: Vec<&str> = sands.iter().filter_map(|e| e.display_gloss()).collect();
    assert!(
        glosses.iter().any(|g| g.contains("drifting")),
        "{glosses:?}"
    );
    assert!(glosses.iter().any(|g| g.contains("blinds")), "{glosses:?}");
    assert_eq!(
        glosses
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "four words, four distinct meanings — not four rows all glossed `sand`"
    );
    // Each is its own lexeme with its own descent class.
    let sets: std::collections::BTreeSet<&str> =
        sands.iter().map(|e| e.cognate_set.as_str()).collect();
    assert_eq!(sets.len(), 4);
}

// ------------------------------------------------------ half two: the explanation

/// **ROADMAP M15, half two.** Each gap is explained *in the report*, not silently
/// empty. A missing word cannot be noticed; only a printed claim can be checked.
#[test]
fn every_gap_is_explained_rather_than_silently_empty() {
    for name in ["desert_asterian.ron", "seafarer_asterian.ron"] {
        let genome = load(name);
        let text = render_culture(&genome).expect("renders");
        let gaps = absences(&genome);

        assert!(!gaps.is_empty(), "{name} declares no gaps to explain");
        for (concept, culture_trait, reason) in &gaps {
            assert!(
                text.contains(concept),
                "{name}: `{concept}` goes uncoined and the report never names it"
            );
            assert!(
                !reason.trim().is_empty(),
                "{name}: `{concept}` is a gap with no reason — the accident M15 replaces"
            );
            assert!(
                text.contains(reason),
                "{name}: `{concept}`'s reason is not printed:\n{text}"
            );
            assert!(
                text.contains(culture_trait),
                "{name}: the trait explaining `{concept}` is not named"
            );
        }
    }
}

/// The gap list and the coined lexicon must agree: every meaning the report says is
/// absent really is uncoined, and every uncoined meaning is in the report. Two
/// sources of truth that disagreed would make the explanation worthless.
#[test]
fn the_reported_gaps_are_exactly_the_uncoined_meanings() {
    for name in ["desert_asterian.ron", "seafarer_asterian.ron"] {
        let genome = load(name);
        let lexicon = coin(&genome);
        let reported: std::collections::BTreeSet<&str> =
            absences(&genome).into_iter().map(|(c, _, _)| c).collect();

        for meaning in meanings(&genome.concepts) {
            let coined = !lexicon.by_concept(&ConceptKey::new(meaning.key)).is_empty();
            let explained = reported.contains(meaning.key);
            assert_ne!(
                coined,
                explained,
                "{name}: `{}` is {} but the report says {}",
                meaning.key,
                if coined { "coined" } else { "uncoined" },
                if explained { "it is absent" } else { "nothing" }
            );
        }
    }
}

// ------------------------------------------------- the contract that makes it work

/// The draw contract's first clause: an absent concept still **draws and discards**,
/// so removing a meaning cannot move a word it did not remove.
#[test]
fn removing_a_meaning_does_not_move_any_other_word() {
    let genome = load("desert_asterian.ron");
    let bare = LanguageGenome {
        environment: Default::default(),
        ..genome.clone()
    };

    let shaped = coin(&genome);
    let unshaped = coin(&bare);
    assert!(
        unshaped.len() > shaped.len(),
        "the profile removes meanings"
    );

    // Every meaning the desert people do have is the same word in both.
    let mut compared = 0usize;
    for meaning in meanings(&genome.concepts) {
        let key = ConceptKey::new(meaning.key);
        let a = shaped.by_concept(&key);
        let b = unshaped.by_concept(&key);
        if a.len() != 1 || b.len() != 1 {
            continue; // absent, or elaborated — compared elsewhere
        }
        compared += 1;
        assert_eq!(
            a[0].phonemic_form, b[0].phonemic_form,
            "`{}` moved when an unrelated meaning was removed",
            meaning.key
        );
    }
    assert!(compared > 600, "the scan must have scanned: {compared}");
}

/// The second clause: elaboration draws its extra words from `RngDomain::Elaboration`,
/// so adding a distinction cannot move a word either.
#[test]
fn elaborating_a_meaning_does_not_move_any_other_word() {
    let genome = load("desert_asterian.ron");
    let mut more = genome.clone();
    more.environment.traits[0].elaborates[0]
        .senses
        .push("sand nobody has a use for".to_owned());

    let before = coin(&genome);
    let after = coin(&more);
    assert_eq!(after.len(), before.len() + 1, "exactly one more word");

    for meaning in meanings(&genome.concepts) {
        let key = ConceptKey::new(meaning.key);
        if key.as_str() == "SAND" {
            continue;
        }
        let a = before.by_concept(&key);
        let b = after.by_concept(&key);
        assert_eq!(
            a.iter().map(|e| &e.phonemic_form).collect::<Vec<_>>(),
            b.iter().map(|e| &e.phonemic_form).collect::<Vec<_>>(),
            "`{}` moved when SAND gained a distinction",
            meaning.key
        );
    }
}

#[test]
fn coining_a_shaped_lexicon_twice_is_byte_identical() {
    let genome = load("seafarer_asterian.ron");
    assert_eq!(coin(&genome), coin(&genome));
}

/// A pre-M15 file has no `environment`, coins exactly what it always did, and must
/// not gain a byte.
#[test]
fn a_pre_m15_fixture_round_trips_with_no_new_bytes_and_coins_the_same_lexicon() {
    let genome = load("proto_asterian.ron");
    assert!(genome.environment.is_empty());

    let text = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
    assert!(
        !text.contains("environment"),
        "an empty profile must not be written into a file that had none"
    );

    // The shaped builder with an empty profile is the unshaped builder.
    let shaped = coin(&genome);
    let plain = stem_lexicon::build_proto_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &meanings(&genome.concepts),
        genome.seed,
    )
    .expect("coins");
    assert_eq!(shaped, plain);
    assert_eq!(shaped.len(), CONCEPT_COUNT);
}

// ------------------------------------------------------- the profile band and CLI

/// The ADR-0009 projection, M15's instance: the `HeavilyShaped` band holds **exactly**
/// when the `large_vocabulary_gap` Note fires, because both read the one shared
/// `LARGE_VOCABULARY_GAP`.
#[test]
fn the_shaping_band_and_the_validation_note_cannot_disagree() {
    let genome = load("desert_asterian.ron");
    let profile = genome.plausibility_profile();
    let report = genome.validate();
    let note_fired = report
        .issues
        .iter()
        .any(|i| i.code == "large_vocabulary_gap");

    assert_eq!(
        profile.vocabulary_shaping == VocabularyShaping::HeavilyShaped,
        note_fired,
        "band {:?} against note {note_fired}, at {} uncoined and a threshold of \
         {LARGE_VOCABULARY_GAP}",
        profile.vocabulary_shaping,
        profile.shaping_counts.0
    );
    // This fixture is deliberately below the threshold — a real culture, not a
    // stress test — so it reads `Shaped`.
    assert_eq!(profile.vocabulary_shaping, VocabularyShaping::Shaped);
    assert!(report.errors().next().is_none(), "{report}");
}

/// A language with no profile reads `Unshaped`, which is a different fact from a
/// profile that removes nothing.
#[test]
fn a_language_with_no_profile_reads_unshaped() {
    let genome = load("proto_asterian.ron");
    assert_eq!(
        genome.plausibility_profile().vocabulary_shaping,
        VocabularyShaping::Unshaped
    );
    let text = stem_genome::render_profile(&genome.plausibility_profile(), &genome.name);
    assert!(text.contains("Vocabulary shaping"), "{text}");
    assert!(text.contains("no culture profile"), "{text}");
}

#[test]
fn the_culture_command_prints_the_profile_and_succeeds() {
    let output = stemma(&["culture", fixture("desert_asterian.ron").to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);
    assert!(text.contains("Environment & culture"), "{text}");
    assert!(text.contains("High desert"), "{text}");
    assert!(
        text.contains("no living speaker has seen open water"),
        "{text}"
    );
    assert!(
        text.contains("Vocabulary"),
        "the arithmetic is stated: {text}"
    );
}

/// The command must be useful on a language that has no profile, rather than
/// printing an empty section.
#[test]
fn the_culture_command_names_what_an_absent_profile_silently_asserts() {
    let output = stemma(&["culture", fixture("proto_asterian.ron").to_str().unwrap()]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("No culture profile"), "{text}");
    assert!(text.contains("not a deliberate one"), "{text}");
}

#[test]
fn new_lexicon_says_how_the_culture_shaped_the_result() {
    let output = stemma(&[
        "new-lexicon",
        fixture("desert_asterian.ron").to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("culture profile") && stderr.contains("uncoined"),
        "coining must say what the profile did: {stderr}"
    );
    // And the count is short of the full list by exactly the shaping.
    let (absent, _, extra) =
        stem_lexicon::shaping_counts(&load("desert_asterian.ron").environment, &meanings(&[]));
    assert_eq!(
        stdout(&output).lines().count(),
        CONCEPT_COUNT - absent + extra
    );
}
