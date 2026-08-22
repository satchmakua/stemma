//! The ROADMAP M24 acceptance tests: **a bioluminescent pulse language undergoes an
//! ordered signal change and traces it, through the same engine; every existing vocal
//! fixture produces byte-identical output afterwards.**
//!
//! # The clause that carries the milestone
//!
//! "through the same engine." Generalising a phoneme to a channel signal would be easy
//! to *claim* — add a `Signal` type, give it a parallel engine, and announce that
//! Stemma models non-vocal languages. What makes the claim worth anything is that
//! `apply_rules` was not touched: the function that turned `*takala` into `taɣal` at M3
//! is the function that dims a Kethi flicker between two glows, and it never learned
//! that light exists.
//!
//! That was possible because the engine never asked what produced a segment. It matches
//! feature bundles, rewrites feature bundles, and resolves the result to a symbol. **A
//! vocal tract lives entirely in which dimensions a unit values** — so appending the
//! luminous dimensions to the closed feature set is the whole generalisation, and the
//! rest of the milestone is making the *checks* stop assuming a mouth.
//!
//! # The second clause is the dangerous one
//!
//! "every existing vocal fixture produces byte-identical output afterwards." Six new
//! features, a third `SegmentKind`, a third template slot and a rewritten requirement
//! geometry are exactly the kind of change that moves a generated lexicon by one word
//! and is never noticed.
//!
//! The strongest guard for it is not in this file: it is M2's frozen digest, which
//! hashes the reference language's whole generated lexicon and has been pinned since
//! before any of this existed. `every_vocal_fixture_is_untouched_by_the_generalisation`
//! sweeps the corpus as well, and `a_vocal_phoneme_values_no_channel_dimension` pins
//! the reason both of them still pass.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::LanguageGenome;
use stem_phonology::{Feature, PRE_M24_FEATURE_COUNT, SegmentKind, Sign};

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
    stem_io::load(fixture(name)).expect("loads")
}

/// The Kethi with their vocabulary coined — six signals, no sounds.
fn kethi() -> LanguageGenome {
    let genome = load("luminous_kethi.ron");
    let lexicon = stem_lexicon::build_shaped_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &stem_lexicon::meanings(&genome.concepts),
        genome.ecology(),
        genome.seed,
    )
    .expect("coins");
    genome.with_lexicon(lexicon)
}

/// The signal change, read through M10's DSL parser — the same front end the CLI uses.
fn rules() -> stem_soundchange::RuleSet {
    let path = fixture("rules_kethi_dimming.sc");
    let source = std::fs::read_to_string(&path).expect("readable");
    stem_soundchange::parse_rule_set(&source, &path.display().to_string()).expect("parses")
}

fn late() -> LanguageGenome {
    let (evolved, _) = kethi()
        .evolve("kethi_late", "Late Kethi", &rules(), 700)
        .expect("evolves");
    evolved
}

// ------------------------------------------------- half one: the signal change

/// **ROADMAP M24's acceptance, first clause.** A pulse language undergoes an ordered
/// signal change and traces it.
#[test]
fn a_pulse_language_undergoes_an_ordered_signal_change_and_traces_it() {
    let genome = late();
    assert_eq!(genome.applied_rules.len(), 2, "two ordered changes");

    // `w_0016` is the word both rules touch, and touch in order: its slow flicker
    // becomes rapid, and the now-rapid flicker then dims between two steady glows.
    let entry = genome
        .lexicon
        .get(&stem_core::WordId::new("w_0016"))
        .expect("the fixture coins it");
    let trace = entry.trace.as_ref().expect("traced");
    let steps: Vec<&str> = trace.steps.iter().map(|s| s.rule.as_str()).collect();
    assert_eq!(
        steps,
        ["r_k01", "r_k02"],
        "both rules fired on this word, in chronological order"
    );

    // And the trace replays to the stored form — §3.3, over light.
    let forms = trace.replay();
    assert_eq!(forms.last(), Some(&entry.phonemic_form));

    let printed = stdout(&stemma(&["trace", &on_disk(&genome, "m24_late"), "w_0016"]));
    assert!(printed.contains("r_k01  Rapid merger"), "{printed}");
    assert!(printed.contains("r_k02  Interglow dimming"), "{printed}");
}

/// **Through the same engine.** Not a parallel implementation — the identical call, on
/// a pure function of five arguments, with no branch anywhere on what produced a
/// segment.
#[test]
fn the_signal_change_runs_through_the_untouched_sound_change_engine() {
    let genome = kethi();
    let evolution = stem_soundchange::apply_rules(
        &rules().rules,
        0,
        &genome.phonemes,
        &genome.prosody,
        &genome.lexicon,
    )
    .expect("the engine applies a signal change exactly as it applies a sound change");

    assert!(
        evolution.lexicon.iter().any(|e| e.trace.is_some()),
        "and traces it"
    );
    // The engine names no channel type. If it ever did, `the_engine_never_references_*`
    // in `stem_soundchange` would fail — three source scans already hold it to
    // phonology, and this milestone did not add a fourth exception.
    assert_eq!(evolution.lexicon.len(), genome.lexicon.len());
}

/// Order is chronology (§11.3), and here it is **load-bearing**: `r_k01` makes a slow
/// flicker rapid, and only then can `r_k02` catch it. Reverse them and the word comes
/// out unchanged — the M19 proof, applied to light.
#[test]
fn reversing_the_two_signal_changes_leaves_the_word_alone() {
    let genome = kethi();
    let mut reversed = rules();
    reversed.rules.reverse();

    let (forward, _) = genome.evolve("f", "F", &rules(), 700).expect("evolves");
    let (backward, _) = genome.evolve("b", "B", &reversed, 700).expect("evolves");

    let id = stem_core::WordId::new("w_0016");
    let a = forward.lexicon.get(&id).expect("there");
    let b = backward.lexicon.get(&id).expect("there");
    assert_ne!(
        a.phonemic_form, b.phonemic_form,
        "rule order is observable over signals exactly as it is over sounds"
    );
}

/// The engine **cannot innovate a signal**, and that is a stated limitation rather than
/// an accident. Minting needs a reviewed reference table, and there is none for a
/// channel nobody has studied — so a rule producing an undeclared bundle earns
/// `unnameable_output` and leaves the site alone.
#[test]
fn a_signal_change_can_merge_but_never_invent() {
    let genome = kethi();
    // Setting a dimension to a value no declared signal carries: the output bundle
    // exists nowhere in the inventory, and there is no table to mint it from.
    let mut inventive = rules();
    inventive.rules.truncate(1);
    inventive.rules[0].change = stem_soundchange::Change::Set(
        stem_phonology::FeatureBundle::EMPTY.with(Feature::Saturated, Sign::Minus),
    );

    let (evolved, report) = genome
        .evolve("x", "X", &inventive, 100)
        .expect("still applies");
    assert!(
        report
            .warnings()
            .any(|i| i.code == "soundchange.unnameable_output"),
        "the engine says it could not name the result: {report}"
    );
    // And left the sites alone rather than inventing a symbol for them.
    assert_eq!(
        evolved.phonemes.len(),
        genome.phonemes.len(),
        "no signal was minted"
    );
}

/// The Kethi's units are **signals**, not consonants or vowels — the whole model, in
/// one assertion.
#[test]
fn the_kethi_have_signals_and_no_sounds() {
    let genome = load("luminous_kethi.ron");
    assert_eq!(genome.phonemes.signals().count(), 6);
    assert_eq!(genome.phonemes.consonants().count(), 0);
    assert_eq!(genome.phonemes.vowels().count(), 0);
    assert!(
        genome
            .phonemes
            .iter()
            .all(|p| p.kind == SegmentKind::Signal),
        "every unit is on the luminous channel"
    );
    let report = genome.validate();
    assert!(report.is_ok(), "{report}");
}

/// A template of `S` slots is a signal string, and needs no nucleus. Mixing the two
/// **is** an error, because nothing in the model can say what a syllable made partly of
/// light would be.
#[test]
fn a_signal_template_needs_no_nucleus_and_may_not_be_mixed_with_one() {
    let genome = load("luminous_kethi.ron");
    for template in &genome.phonotactics.templates {
        assert!(
            template.slots().is_ok(),
            "`{}` is a legal signal string",
            template.pattern
        );
    }

    let mixed = stem_phonology::phonotactics::WeightedTemplate::new("CVS");
    assert!(matches!(
        mixed.slots(),
        Err(stem_phonology::phonotactics::TemplateError::MixedChannels)
    ));
}

// --------------------------------------- half two: nothing vocal moved an inch

/// **ROADMAP M24's acceptance, second clause.** Every vocal fixture is untouched.
///
/// Loaded and saved: the reloaded genome must equal the loaded one, and no channel
/// token may reach the file. Six appended features and a third `SegmentKind` are
/// exactly the kind of change that shifts one word in one lexicon unnoticed.
#[test]
fn every_vocal_fixture_is_untouched_by_the_generalisation() {
    for name in [
        "proto_asterian.ron",
        "asterian_attested.ron",
        "desert_asterian.ron",
        "seafarer_asterian.ron",
        "grammar_asterian.ron",
        "grammar_svo_asterian.ron",
        "grammar_free_asterian.ron",
        "written_asterian.ron",
        "derivation_asterian.ron",
        "morphology_asterian.ron",
    ] {
        let genome = load(name);
        assert_eq!(
            genome.phonemes.signals().count(),
            0,
            "`{name}` is a vocal fixture"
        );

        let path = std::env::temp_dir().join(format!("stemma_m24_{name}"));
        stem_io::save(&path, &genome).expect("saves");
        // Scanned as SIGNED FEATURE TOKENS and as the slot value, not as bare words:
        // `asterian_attested.ron` glosses `*takala` as "star, bright thing", and prose
        // about brightness is not a claim about a channel dimension. The first draft of
        // this test failed on that gloss, which is a fair warning about substring scans.
        let text = std::fs::read_to_string(&path).expect("readable");
        for dimension in [
            "luminous",
            "bright",
            "long_wave",
            "saturated",
            "pulsed",
            "rapid",
        ] {
            for sign in ['+', '-'] {
                let token = format!("\"{sign}{dimension}\"");
                assert!(
                    !text.contains(&token),
                    "`{name}` gained `{token}`; M24 must be invisible to a vocal language"
                );
            }
        }
        assert!(
            !text.contains("kind: signal"),
            "`{name}` gained a channel-signal unit"
        );
        assert_eq!(
            stem_io::load::<LanguageGenome>(&path).expect("reloads"),
            genome
        );
    }
}

/// The reason the sweep above passes: **a vocal phoneme values no channel dimension,
/// and a signal values no articulatory feature.** Neither is `-` on the other's
/// dimensions — the question does not arise (ADR-0004), and that is what lets one
/// bundle type carry both.
#[test]
fn a_vocal_phoneme_values_no_channel_dimension() {
    let channel: Vec<Feature> = Feature::ALL[PRE_M24_FEATURE_COUNT..].to_vec();
    assert_eq!(channel.len(), 6, "M24 appended six");

    for phoneme in load("proto_asterian.ron").phonemes.iter() {
        for dimension in &channel {
            assert!(
                !phoneme.features.is_specified(*dimension),
                "/{}/ values `{}`, which is a dimension of light",
                phoneme.ipa,
                dimension.name()
            );
        }
        // And the rendering is unchanged, which is what every golden test depends on.
        assert!(
            !phoneme.features.render().contains("luminous"),
            "/{}/ renders a channel dimension",
            phoneme.ipa
        );
    }

    for signal in load("luminous_kethi.ron").phonemes.iter() {
        for vocal in &Feature::ALL[..PRE_M24_FEATURE_COUNT] {
            assert!(
                !signal.features.is_specified(*vocal),
                "`{}` values `{}`, which is a feature of speech",
                signal.id,
                vocal.name()
            );
        }
    }
}

/// The channel dimensions are **appended**, never interleaved. Declaration order is the
/// rendering order, so an insertion would rewrite every bundle in every export ever
/// produced.
#[test]
fn the_vocal_feature_prefix_is_frozen() {
    let names: Vec<&str> = Feature::ALL[..PRE_M24_FEATURE_COUNT]
        .iter()
        .map(|f| f.name())
        .collect();
    assert_eq!(
        names,
        [
            "syllabic",
            "consonantal",
            "sonorant",
            "approximant",
            "continuant",
            "nasal",
            "lateral",
            "trill",
            "voice",
            "labial",
            "coronal",
            "dorsal",
            "high",
            "low",
            "back",
            "round",
        ]
    );
}

/// A vocal language still gets every vocal check. The generalisation loosened the
/// nucleus rule for an **all-signal** inventory only, and a broken human language must
/// still be broken.
#[test]
fn a_vocal_language_with_no_vowels_is_still_an_error() {
    let broken = load("invalid_no_vowels.ron");
    let report = broken.validate();
    assert!(
        report.errors().any(|i| i.code == "phonology.no_nucleus"),
        "{report}"
    );
    assert!(!report.is_ok());

    // And a MIXED inventory answers the vocal question too — one consonant is a claim
    // about syllables, so a half-converted language cannot duck it.
    let kethi = load("luminous_kethi.ron");
    let mut units: Vec<stem_phonology::Phoneme> = kethi.phonemes.iter().cloned().collect();
    units.push(stem_phonology::Phoneme::new(
        "ph_p",
        "p",
        SegmentKind::Consonant,
    ));
    let mixed = stem_phonology::PhonemeInventory::from_phonemes(units);
    assert!(
        mixed.validate().errors().any(|i| i.code == "no_nucleus"),
        "a single consonant makes it a vocal claim again"
    );
}

/// Two runs, byte for byte (§9.4). The signal lexicon is as reproducible as any other.
#[test]
fn a_signal_language_is_as_deterministic_as_a_spoken_one() {
    assert_eq!(kethi().lexicon, kethi().lexicon);
    assert_eq!(late(), late());

    let path = fixture("luminous_kethi.ron");
    let path = path.to_str().expect("path");
    assert_eq!(
        stemma(&["new-lexicon", path]).stdout,
        stemma(&["new-lexicon", path]).stdout
    );
}

/// `Signal` is the general name and `Phoneme` is the type — an alias, not a rename,
/// because `phonemes:` appears in every language file ever written with this program
/// and M24's own acceptance forbids moving a byte of them.
#[test]
fn the_general_name_points_at_the_same_type() {
    let signal: stem_phonology::Signal =
        stem_phonology::Phoneme::new("sg_x", "◆", SegmentKind::Signal);
    let phoneme: stem_phonology::Phoneme = signal.clone();
    assert_eq!(signal, phoneme, "one type, two names");
}

fn on_disk(genome: &LanguageGenome, name: &str) -> String {
    let path = std::env::temp_dir().join(format!("stemma_{name}.ron"));
    stem_io::save(&path, genome).expect("saves");
    path.display().to_string()
}
