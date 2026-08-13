//! The ROADMAP M14 acceptance tests: **a language of a few hundred roots yields a
//! several-thousand-word lexicon in which every derived word traces to its parts;
//! and a sound change applied afterwards makes a compound opaque — its parts no
//! longer recoverable by eye, but still recorded.**
//!
//! The load-bearing test is [`a_sound_change_makes_a_compound_opaque_without_losing_its_parts`]:
//! it derives with the real `derive`, evolves with the real engine
//! (`LanguageGenome::evolve`), and asserts on a compound whose surface genuinely no
//! longer contains either part. Everything else — the scale, the CLI wiring, the
//! renderer — trusts that mechanism.
//!
//! Lives here because loading a fixture needs `stem_io` + `stem_genome`, which sit
//! above `stem_lexicon` — the same reason `reference_phonology.rs` is here.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::Validate;
use stem_genome::{LanguageGenome, render_word_history};
use stem_lexicon::{Lexicon, WordEntry, WordSource, derive};
use stem_soundchange::RuleSet;

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

/// The M14 fixture with its 673 roots coined — the base lexicon, before derivation.
fn seeded_proto() -> LanguageGenome {
    let genome: LanguageGenome =
        stem_io::load(fixture("derivation_asterian.ron")).expect("the fixture loads");
    let lexicon = stem_lexicon::build_proto_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        &stem_lexicon::meanings(&genome.concepts),
        genome.seed,
    )
    .expect("seeds");
    genome.with_lexicon(lexicon)
}

/// The same, with every derivation pattern run over it.
fn derived() -> LanguageGenome {
    let proto = seeded_proto();
    let coined = derive(
        &proto.morphology.derivations,
        &proto.morphology.morphemes,
        &proto.lexicon,
        &proto.id,
    )
    .expect("derives");
    let mut entries: Vec<WordEntry> = proto.lexicon.iter().cloned().collect();
    entries.extend(coined);
    proto.with_lexicon(Lexicon::from_entries(entries))
}

// --------------------------------------------------------------- half one: scale

/// **ROADMAP M14, half one.** A few hundred roots become several thousand words.
#[test]
fn a_few_hundred_roots_yield_a_several_thousand_word_lexicon() {
    let proto = seeded_proto();
    let full = derived();

    assert_eq!(proto.lexicon.len(), 673, "the M13 concept list, coined");
    assert!(
        full.lexicon.len() > 3000,
        "673 roots over 14 patterns should be several thousand words, got {}",
        full.lexicon.len()
    );
    // The lexicon is now mostly made of itself, which is the point of the milestone.
    let derived_count = full
        .lexicon
        .iter()
        .filter(|e| e.source == WordSource::Derived)
        .count();
    assert!(
        derived_count > proto.lexicon.len() * 3,
        "most of the lexicon should be built from the rest of it, got {derived_count} \
         derived against {} roots",
        proto.lexicon.len()
    );
}

/// **ROADMAP M14, half one's real claim.** Not "there are many words" but "every one
/// of them traces to its parts" — a derived word with no record would be the §3.3
/// violation this milestone exists to avoid.
#[test]
fn every_derived_word_records_the_words_it_was_built_from() {
    let full = derived();
    let mut checked = 0usize;

    for entry in full.lexicon.iter() {
        if entry.source != WordSource::Derived {
            continue;
        }
        checked += 1;
        assert!(
            !entry.bases.is_empty(),
            "`{}` is derived and records no base — a composed form with no recorded \
             composition is the §3.3 bug",
            entry.id
        );
        for base in &entry.bases {
            let source = full.lexicon.get(&base.word).unwrap_or_else(|| {
                panic!(
                    "`{}` names base `{}`, which is not in the lexicon",
                    entry.id, base.word
                )
            });
            assert_eq!(
                base.cognate_set, source.cognate_set,
                "`{}`'s echo of base `{}` must match its source, or the etymology \
                 points nowhere",
                entry.id, base.word
            );
            assert!(
                base.end > base.start,
                "`{}` records a zero-width span for base `{}`",
                entry.id,
                base.word
            );
        }
        // Every span must land inside the composition form.
        let length = entry.phonemic_form.len() as u32;
        for base in &entry.bases {
            assert!(
                base.end <= length,
                "`{}`'s base span {}..{} runs past its {length}-segment form",
                entry.id,
                base.start,
                base.end
            );
        }
    }
    assert!(
        checked > 3000,
        "the scan must actually have scanned: {checked}"
    );
}

#[test]
fn a_derived_lexicon_validates_cleanly() {
    let full = derived();
    let report = full.validate();
    assert!(
        report.errors().next().is_none(),
        "deriving must not break the language: {report}"
    );
    // Specifically: no dangling or stale base anywhere in 3,978 words.
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.code == "dangling_base" || i.code == "stale_base"),
        "{report}"
    );
}

#[test]
fn deriving_twice_produces_an_identical_lexicon() {
    assert_eq!(
        derived().lexicon,
        derived().lexicon,
        "derivation has no RNG, so two runs are byte-identical"
    );
}

/// Ids and cognate sets must not collide with the roots they were built from — the
/// invariant `Lexicon::validate` calls an Error, checked here at scale.
#[test]
fn derived_words_collide_with_no_root_on_id_or_cognate_set() {
    let full = derived();
    let ids: std::collections::BTreeSet<&str> =
        full.lexicon.iter().map(|e| e.id.as_str()).collect();
    let sets: std::collections::BTreeSet<&str> = full
        .lexicon
        .iter()
        .map(|e| e.cognate_set.as_str())
        .collect();
    assert_eq!(ids.len(), full.lexicon.len(), "a word id is duplicated");
    assert_eq!(
        sets.len(),
        full.lexicon.len(),
        "a cognate set is duplicated"
    );
}

// ------------------------------------------------------------ half two: opacity

/// **ROADMAP M14, half two, and the load-bearing test.**
///
/// `wiki` "blood" + `kippa` "debt" compounds to `wikikippa`. Run the Coastal rule
/// set over it — velar lenition, then intervocalic fricative loss, then apocope —
/// and it becomes **`wiiipp`**, which contains neither part. That is what
/// lexicalisation looks like from the outside.
///
/// From the inside, both parts are still exactly addressable: the stored spans walk
/// through the word's own trace and say `wiki` became `wii` and `kippa` became
/// `ipp`. A tool that recomputed the decomposition by matching strings would lose
/// this word at precisely the moment it got interesting.
#[test]
fn a_sound_change_makes_a_compound_opaque_without_losing_its_parts() {
    let rules: RuleSet = stem_io::load(fixture("rules_coastal.ron")).expect("rules load");
    // `evolve` hands back the report alongside the language; the acceptance is about
    // the forms, and `a_derived_lexicon_validates_cleanly` covers the report.
    let (late, _report) = derived()
        .evolve("late", "Late Asterian", &rules, 500)
        .expect("evolves");

    let compound = late
        .lexicon
        .iter()
        .find(|e| e.glosses.first().is_some_and(|g| g == "blood-debt"))
        .expect("the BLOOD+DEBT compound is in the fixture");

    let surface = compound.written(&late.phonemes).expect("renders");
    assert_eq!(surface, "wiiipp", "the eroded form");

    // Opaque by eye: neither part survives as a substring.
    let composition = compound
        .trace
        .as_ref()
        .expect("it went through the rules")
        .input
        .written(&late.phonemes)
        .expect("renders");
    assert_eq!(composition, "wikikippa");
    for part in ["wiki", "kippa"] {
        assert!(
            !surface.contains(part),
            "`{part}` is still visible in `{surface}`, so this word is not opaque and \
             the test proves nothing"
        );
    }

    // Recorded all the same: each span, walked through this word's own trace.
    assert_eq!(compound.bases.len(), 2);
    let recovered = |i: usize| -> String {
        let base = &compound.bases[i];
        let segments = compound.morpheme_surface(base.start as usize, base.end as usize);
        segments
            .iter()
            .map(|id| {
                late.phonemes
                    .require(id)
                    .expect("in the inventory")
                    .written()
            })
            .collect()
    };
    assert_eq!(recovered(0), "wii", "what `wiki` became");
    assert_eq!(recovered(1), "ipp", "what `kippa` became");
    assert_eq!(compound.bases[0].gloss, "blood");
    assert_eq!(compound.bases[1].gloss, "debt");

    // And the parts are still cognate-visible: each set resolves to a live word.
    for base in &compound.bases {
        assert!(
            late.lexicon.by_cognate_set(&base.cognate_set).is_some(),
            "base {} lost its thread",
            base.word
        );
    }

    // The rendered story says all of it in one place.
    let story = render_word_history(&late, compound).expect("renders");
    assert!(story.contains("Formation:"), "{story}");
    assert!(
        story.contains("\"blood\"") && story.contains("\"debt\""),
        "{story}"
    );
    assert!(story.contains("the seam has eroded"), "{story}");
}

/// The compound is a **new lexeme**, not a reflex of its left base: it must not
/// share a cognate set with either part, or M5's table would put them on one row.
#[test]
fn a_compound_is_a_new_lexeme_and_not_a_reflex_of_either_part() {
    let full = derived();
    let compound = full
        .lexicon
        .iter()
        .find(|e| e.glosses.first().is_some_and(|g| g == "blood-debt"))
        .expect("present");
    for base in &compound.bases {
        assert_ne!(
            compound.cognate_set, base.cognate_set,
            "a compound that shared its base's descent class would claim to *be* it"
        );
    }
}

// --------------------------------------------------------------------- the CLI

#[test]
fn the_derive_command_coins_and_saves() {
    let dir = std::env::temp_dir().join("stemma_m14_cli");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let proto = dir.join("proto.ron");
    let out = dir.join("derived.ron");

    assert!(
        stemma(&[
            "new-lexicon",
            fixture("derivation_asterian.ron").to_str().unwrap(),
            "--out",
            proto.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let output = stemma(&[
        "derive",
        proto.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let genome: LanguageGenome = stem_io::load(&out).expect("loads");
    assert!(genome.lexicon.len() > 3000, "{}", genome.lexicon.len());
    assert!(genome.validate().errors().next().is_none());

    std::fs::remove_dir_all(&dir).ok();
}

/// **Replace, never append.** Running `derive` on an already-derived file must
/// re-coin rather than double the lexicon — the `new-lexicon` rule, which is what
/// makes the command idempotent instead of destructive.
#[test]
fn deriving_a_second_time_replaces_rather_than_appends() {
    let dir = std::env::temp_dir().join("stemma_m14_idempotent");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let proto = dir.join("proto.ron");
    let once = dir.join("once.ron");
    let twice = dir.join("twice.ron");

    assert!(
        stemma(&[
            "new-lexicon",
            fixture("derivation_asterian.ron").to_str().unwrap(),
            "--out",
            proto.to_str().unwrap(),
        ])
        .status
        .success()
    );
    assert!(
        stemma(&[
            "derive",
            proto.to_str().unwrap(),
            "--out",
            once.to_str().unwrap()
        ])
        .status
        .success()
    );
    let output = stemma(&[
        "derive",
        once.to_str().unwrap(),
        "--out",
        twice.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("replacing"),
        "a replacement should say so"
    );

    let a: LanguageGenome = stem_io::load(&once).expect("loads");
    let b: LanguageGenome = stem_io::load(&twice).expect("loads");
    assert_eq!(a.lexicon, b.lexicon, "re-deriving must be idempotent");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_derivations_command_lists_every_pattern() {
    let output = stemma(&[
        "derivations",
        fixture("derivation_asterian.ron").to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("14 derivation pattern(s)"), "{text}");
    for id in ["AGENT", "COMPOUND", "DIMINUTIVE"] {
        assert!(text.contains(id), "`{id}` is missing from:\n{text}");
    }
}

/// `--limit` tightens, never loosens. A flag typed at the prompt must not overrun a
/// bound the file asked for.
#[test]
fn the_limit_flag_caps_every_pattern() {
    let dir = std::env::temp_dir().join("stemma_m14_limit");
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).expect("temp dir");
    let proto = dir.join("proto.ron");
    assert!(
        stemma(&[
            "new-lexicon",
            fixture("derivation_asterian.ron").to_str().unwrap(),
            "--out",
            proto.to_str().unwrap(),
        ])
        .status
        .success()
    );

    let output = stemma(&["derive", proto.to_str().unwrap(), "--limit", "2"]);
    assert!(output.status.success());
    let text = stdout(&output);
    // 14 patterns × at most 2 each.
    let coined = text.lines().count();
    assert!(coined <= 28, "the cap was not applied: {coined} words");
    assert!(coined > 0, "the cap must not silence the command");

    // **Every** pattern must still fire. An upper bound alone passes trivially when
    // a pattern coins nothing, which is exactly what a shared-budget cap did to the
    // compounds — the last pattern in the file, and the silent casualty.
    assert_eq!(
        coined, 28,
        "all 14 patterns should coin their 2, but only {coined} words appeared:\n{text}"
    );
    assert_eq!(
        text.lines()
            .filter(|l| l.contains("star + stone") || l.contains("sun + road"))
            .count(),
        2,
        "the compound pattern is last in the file and must not be starved by the \
         thirteen before it:\n{text}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A pre-M14 file has no `derivations`, and must say so rather than failing.
#[test]
fn a_language_with_no_patterns_says_so_and_succeeds() {
    let output = stemma(&["derive", fixture("proto_asterian.ron").to_str().unwrap()]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no derivation patterns"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Adding `bases` must not strand a file that never had one.
#[test]
fn a_pre_m14_fixture_round_trips_with_no_new_bytes() {
    for name in ["proto_asterian.ron", "asterian_attested.ron"] {
        let genome: LanguageGenome = stem_io::load(fixture(name)).expect("loads");
        let text = stem_io::to_string(&genome, stem_io::Format::Ron).expect("serialise");
        assert!(
            !text.contains("bases"),
            "an empty `bases` must not be written into `{name}`"
        );
        assert!(
            !text.contains("derivations"),
            "an empty `derivations` must not be written into `{name}`"
        );
    }
}
