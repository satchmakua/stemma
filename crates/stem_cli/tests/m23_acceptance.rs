//! The ROADMAP M23 acceptance tests: **a profile with no vocal tract but a
//! bioluminescent channel validates, and the engine reports which existing machinery
//! does *not* apply to it rather than silently producing vowels.**
//!
//! # The clause that carries the milestone
//!
//! "rather than silently producing vowels". Almost every layer below `stem_genome`
//! assumes a mouth: a `PhonemeInventory` is a set of things a vocal tract can do, a
//! `CVC` template describes a syllable, stress is a property of a syllable. None of
//! that is wrong — it is about a body the Kethi do not have — and the failure mode is
//! the tool carrying on regardless and handing a species that communicates in light a
//! five-vowel system because its machinery had nowhere else to go.
//!
//! So this milestone has two halves and the tests attack both:
//!
//! - **It validates.** `PhonemeInventory::validate` errors with `empty` on a language
//!   with no phonemes, and that Error is what stands between the Kethi and a valid
//!   file. `LanguageGenome::validate` sets aside exactly the checks that assume a
//!   speaker makes sound, and **says which** — `the_set_aside_is_named_and_narrow`.
//! - **The set-aside is not a loophole.** A duplicate phoneme id is still an Error for
//!   a bioluminescent species, because that is a broken *record* rather than a claim
//!   about anatomy. `a_broken_record_cannot_hide_behind_a_claim_to_be_alien` is the
//!   test that stops "alien" becoming a way to silence inconvenient errors.
//!
//! And the guard that matters most for everything already built:
//! `silence_is_not_a_claim_to_be_alien`. Every language written before M23 has an empty
//! profile, and an empty profile must read as an ordinary speaker with an ordinary
//! mouth — otherwise M23 would have set aside the inventory checks for the entire
//! project.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::{Validate, ValidationReport};
use stem_embodiment::{
    Applies, ChannelKind, EmbodimentProfile, Persistence, Simultaneity, Subsystem,
    VOCAL_TRACT_CHECKS, applicability,
};
use stem_genome::{EmbodimentDependence, LanguageGenome};

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

/// The Kethi: no vocal tract, two non-vocal channels.
fn kethi() -> LanguageGenome {
    load("luminous_kethi.ron")
}

// -------------------------------------------------------- the acceptance itself

/// **ROADMAP M23's acceptance.** A body with no vocal tract and a bioluminescent
/// channel is a **valid** language.
#[test]
fn a_species_with_no_vocal_tract_but_a_light_channel_validates() {
    let genome = kethi();
    assert!(!genome.embodiment.has_vocal_tract());
    assert!(
        genome
            .embodiment
            .channels
            .iter()
            .any(|c| c.kind == ChannelKind::Bioluminescent),
        "the roadmap's own test case names a bioluminescent channel"
    );

    let report = genome.validate();
    assert!(
        report.is_ok(),
        "a creature that speaks in light is unusual, not broken: {report}"
    );

    let out = stemma(&[
        "validate",
        fixture("luminous_kethi.ron").to_str().expect("path"),
    ]);
    assert!(out.status.success(), "{}", stdout(&out));
    assert!(stdout(&out).contains("✓ valid"), "{}", stdout(&out));
}

/// And the engine **reports what does not apply**, subsystem by subsystem, with
/// reasons — the clause the whole milestone is written around.
#[test]
fn the_engine_says_which_machinery_does_not_apply() {
    let out = stdout(&stemma(&[
        "embodiment",
        fixture("luminous_kethi.ron").to_str().expect("path"),
    ]));

    assert!(out.contains("What Stemma can do with this body"), "{out}");
    for (subsystem, verdict) in [
        ("Phonology", "does not apply"),
        ("Phonotactics", "does not apply"),
        ("Prosody", "does not apply"),
        ("Sound change", "not built yet"),
    ] {
        let line = out
            .lines()
            .find(|l| l.trim_start().starts_with(subsystem))
            .unwrap_or_else(|| panic!("`{subsystem}` is missing: {out}"));
        assert!(
            line.contains(verdict),
            "`{subsystem}` should read `{verdict}`: {line}"
        );
        // A verdict with no reason leaves a reader guessing, which is what this table
        // exists to stop.
        assert!(
            line.len() > subsystem.len() + verdict.len() + 20,
            "`{subsystem}` gives no reason: {line}"
        );
    }

    // And the parts that are about meaning rather than anatomy still work. A blanket
    // "nothing applies" would be as useless as saying nothing.
    assert!(
        out.contains("Semantics") && out.contains("applies"),
        "{out}"
    );
    assert!(out.contains("Script") && out.contains("partly"), "{out}");
}

/// The set-aside is **named and narrow**. It is the only place anything is filtered out
/// of a sub-report, and the reason is printed rather than left to be discovered.
#[test]
fn the_set_aside_is_named_and_narrow() {
    let report = kethi().validate();
    let note = report
        .issues
        .iter()
        .find(|i| i.code == "embodiment.vocal_checks_set_aside")
        .unwrap_or_else(|| panic!("the set-aside must be reported: {report}"));

    assert!(note.message.contains("no vocal tract"), "{}", note.message);
    assert!(note.message.contains("`empty`"), "{}", note.message);
    assert!(
        note.message.contains("stemma embodiment"),
        "and points at where the whole answer is: {}",
        note.message
    );
    // Every code it names is on the list. A set-aside that could name anything else
    // would be a general suppression mechanism.
    for word in note.message.split('`').skip(1).step_by(2) {
        if word == "stemma embodiment" {
            continue;
        }
        assert!(
            VOCAL_TRACT_CHECKS.contains(&word),
            "`{word}` was set aside and is not on VOCAL_TRACT_CHECKS: {}",
            note.message
        );
    }
}

/// **The set-aside is not a loophole.** A duplicate phoneme id is a broken *record*
/// rather than a claim about anatomy, and stays an Error whatever the speaker is made
/// of. Without this, declaring `embodiment:` would be a way to silence anything.
#[test]
fn a_broken_record_cannot_hide_behind_a_claim_to_be_alien() {
    let mut genome = kethi();
    genome.phonemes = stem_phonology::PhonemeInventory::from_phonemes([
        stem_phonology::Phoneme::new("ph_x", "x", stem_phonology::SegmentKind::Consonant),
        stem_phonology::Phoneme::new("ph_x", "y", stem_phonology::SegmentKind::Consonant),
    ]);

    let report = genome.validate();
    assert!(
        report.errors().any(|i| i.code == "phonology.duplicate_id"),
        "a duplicate id is broken data, not a fact about a mouth: {report}"
    );
    assert!(!report.is_ok());
}

/// **"rather than silently producing vowels."** A non-vocal species that has been given
/// a vocal inventory anyway is told so, and the vowels are counted.
///
/// Reported, never refused: an author converting a language gives it a body before
/// rewriting its inventory, and refusing to load the intermediate state would make the
/// conversion impossible to do a step at a time.
#[test]
fn a_non_vocal_language_holding_vowels_is_told_about_them() {
    let mut genome = load("proto_asterian.ron");
    genome.embodiment = kethi().embodiment;

    let report = genome.validate();
    let warning = report
        .warnings()
        .find(|i| i.code == "embodiment.vocal_inventory_without_a_vocal_tract")
        .unwrap_or_else(|| panic!("{report}"));
    assert!(
        warning.message.contains("5 vowel(s)"),
        "the vowels are counted, not implied: {}",
        warning.message
    );
    assert!(
        report
            .warnings()
            .any(|i| i.code == "embodiment.syllable_templates_without_syllables"),
        "and the syllable templates too: {report}"
    );
    assert!(
        report.is_ok(),
        "still reported rather than refused: {report}"
    );

    // And `stemma embodiment` leads with it rather than burying it under the table.
    let text = stem_genome::render_embodiment(&genome).expect("renders");
    assert!(
        text.contains("But this language has used it anyway:"),
        "{text}"
    );
}

// ------------------------------------------------- the guard for everything else

/// **Silence is not a claim to be alien.** Every language written before M23 has an
/// empty profile, and every one of them has speakers with mouths.
///
/// If an empty profile read as non-vocal, M23 would have set aside the inventory checks
/// for the entire project — including the `no_nucleus` Error that catches a genuinely
/// broken human language.
#[test]
fn silence_is_not_a_claim_to_be_alien() {
    for name in [
        "proto_asterian.ron",
        "asterian_attested.ron",
        "invalid_no_vowels.ron",
    ] {
        let genome = load(name);
        assert!(genome.embodiment.is_empty(), "`{name}` predates M23");
        assert!(
            genome.embodiment.has_vocal_tract(),
            "`{name}`: silence must read as an ordinary speaker"
        );
        assert!(
            applicability(&genome.embodiment)
                .iter()
                .all(|a| a.verdict == Applies::Yes),
            "`{name}`: every subsystem applies"
        );
    }

    // The proof that nothing was set aside: the deliberately broken fixture is still
    // broken, and still for the reason it always was.
    let broken = load("invalid_no_vowels.ron").validate();
    assert!(
        broken.errors().any(|i| i.code == "phonology.no_nucleus"),
        "a human language with no vowels is still an Error: {broken}"
    );
    assert!(!broken.is_ok());
}

/// A pre-M23 file gains no bytes. A user's saved language is their work.
#[test]
fn a_pre_m23_file_saves_without_gaining_an_embodiment_block() {
    for name in ["proto_asterian.ron", "grammar_asterian.ron"] {
        let genome = load(name);
        let path = std::env::temp_dir().join(format!("stemma_m23_{name}"));
        stem_io::save(&path, &genome).expect("saves");
        let text = std::fs::read_to_string(&path).expect("readable");

        for token in ["embodiment", "channels", "vocal_tract", "manipulators"] {
            assert!(
                !text.contains(token),
                "`{name}` gained `{token}`; an empty profile must not reach the file"
            );
        }
        assert_eq!(
            stem_io::load::<LanguageGenome>(&path).expect("reloads"),
            genome
        );
    }
}

/// A language that says nothing about its speakers is told what Stemma assumes, rather
/// than being shown an empty sketch — the `render_grammar` precedent.
#[test]
fn a_language_with_no_profile_is_told_what_stemma_assumes() {
    let out = stdout(&stemma(&[
        "embodiment",
        fixture("proto_asterian.ron").to_str().expect("path"),
    ]));
    assert!(
        out.contains("Nothing is declared about these speakers"),
        "{out}"
    );
    assert!(out.contains("a vocal tract, moving air"), "{out}");
    assert!(
        out.contains("Declare `embodiment:` in the genome"),
        "and told what would change it: {out}"
    );
}

// ------------------------------------------------------- §18.3's consequences

/// §18.3's claims are **tendencies**, reported and never enforced — M17's discipline.
/// A simultaneous channel gets a Note about simultaneous morphology; nothing goes
/// looking for it in the grammar, and the language stays valid without it.
#[test]
fn channel_consequences_are_reported_and_never_enforced() {
    let report = kethi().validate();

    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "embodiment.simultaneous_channel"),
        "the mantle bands carry four parameters at once: {report}"
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "embodiment.persistent_channel"),
        "and the ink register lasts: {report}"
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "embodiment.many_manipulators"),
        "and there are eight arms: {report}"
    );

    // Notes, every one. The Kethi have no morphology at all and are still valid.
    assert!(report.warnings().next().is_none(), "{report}");
    assert!(report.is_ok(), "{report}");
    assert!(
        kethi().morphology.is_empty(),
        "nothing went looking for the morphology §18.3 predicts"
    );
}

/// **No consequence message quotes a frequency** — M17's rule, with more force: there
/// is no sample of alien species to have counted. Section citations are stripped first,
/// because `CLAUDE.md` asks for them and `§18.3` is not a statistic.
#[test]
fn no_consequence_message_claims_a_statistic() {
    let report = kethi().validate();
    for issue in report
        .issues
        .iter()
        .filter(|i| i.code.starts_with("embodiment."))
    {
        // The counts this project does state are counts of the author's own
        // declarations — eight arms, two ecologies — never a claimed rate.
        if issue.code.contains("manipulators")
            || issue.code.contains("speakers")
            || issue.code.contains("vocal_inventory")
        {
            continue;
        }
        let claims: String = strip_citations(&issue.message);
        assert!(
            !claims.chars().any(|c| c.is_ascii_digit()),
            "`{}` quotes a number: {}",
            issue.code,
            issue.message
        );
        for weasel in ["%", "percent", "most species", "usually", "tend to"] {
            assert!(
                !issue.message.to_lowercase().contains(weasel),
                "`{}` claims a statistic: {}",
                issue.code,
                issue.message
            );
        }
    }
}

fn strip_citations(message: &str) -> String {
    let mut out = String::new();
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '§' {
            out.push(c);
            continue;
        }
        while chars
            .peek()
            .is_some_and(|n| n.is_ascii_digit() || *n == '.')
        {
            chars.next();
        }
    }
    out
}

// ------------------------------------------------------------ §17's last row

/// §17's alien-embodiment row scores, and the deferred list is **empty** — the first
/// time since M7 that every dimension the design names has a band.
#[test]
fn the_last_deferred_dimension_finally_scores() {
    let out = stdout(&stemma(&[
        "profile",
        fixture("luminous_kethi.ron").to_str().expect("path"),
    ]));

    assert!(out.contains("Embodiment"), "{out}");
    assert!(out.contains("non-vocal"), "{out}");
    assert!(
        out.contains("Phonology, Phonotactics, Prosody, Sound change"),
        "the raw basis rides along so the band can be checked: {out}"
    );
    assert!(
        out.contains("every dimension §17 names is scored"),
        "and the empty deferred block says it is empty rather than vanishing: {out}"
    );
    assert!(stem_genome::NOT_MODELLED.is_empty());

    // An ordinary language reads `vocal`, which is a fact rather than a high score.
    let ordinary = load("proto_asterian.ron").plausibility_profile();
    assert_eq!(ordinary.embodiment_dependence, EmbodimentDependence::Vocal);
    assert!(ordinary.unavailable.is_empty());
}

/// The band separates "this body has no use for some of Stemma" from "this body has
/// been given machinery it has no use for anyway". The second is a finding about the
/// *file*, and reads differently.
#[test]
fn a_mismatched_language_is_banded_apart_from_a_merely_alien_one() {
    assert_eq!(
        kethi().plausibility_profile().embodiment_dependence,
        EmbodimentDependence::NonVocal,
        "the Kethi have no vowels to be mismatched about"
    );

    let mut mismatched = load("proto_asterian.ron");
    mismatched.embodiment = kethi().embodiment;
    assert_eq!(
        mismatched.plausibility_profile().embodiment_dependence,
        EmbodimentDependence::Mismatched
    );
}

// ------------------------------------------------------------- the invariants

/// §18.1 makes `environment` a field **of** the embodiment profile, and M15 put it on
/// the genome two milestones earlier. Both files load, and `ecology()` is the one place
/// either is read — otherwise following the design document would silently get your
/// ecology ignored.
#[test]
fn an_ecology_declared_the_way_the_design_says_is_actually_read() {
    let genome = kethi();
    assert!(
        genome.environment.is_empty(),
        "the Kethi nest it, as §18.1 says"
    );
    assert!(!genome.embodiment.environment.is_empty());
    assert!(!genome.ecology().is_empty(), "and `ecology()` finds it");

    // It reaches the vocabulary machinery, not just the struct.
    let out = stdout(&stemma(&[
        "culture",
        fixture("luminous_kethi.ron").to_str().expect("path"),
    ]));
    assert!(out.contains("The lightless shelf"), "{out}");
    assert!(out.contains("No Kethi has seen it"), "{out}");
    assert!(
        out.contains("2 uncoined"),
        "and the shaping counts read the nested profile: {out}"
    );
}

/// Declaring both is legal and resolvable — and must never be resolved *silently*, or
/// an author editing the one that loses would watch their change do nothing.
#[test]
fn declaring_an_ecology_in_both_places_is_reported() {
    let mut genome = kethi();
    genome.environment = load("desert_asterian.ron").environment;
    assert!(!genome.environment.is_empty() && !genome.embodiment.environment.is_empty());

    let report = genome.validate();
    assert!(
        report
            .warnings()
            .any(|i| i.code == "embodiment.two_ecologies_declared"),
        "{report}"
    );
    assert_eq!(
        genome.ecology(),
        &genome.embodiment.environment,
        "the nested one wins, as documented"
    );
}

/// A daughter species has its parents' body. `fork` and `evolve` carry the profile
/// verbatim, exactly as they carry the ecology and the scripts.
#[test]
fn a_daughter_inherits_its_parents_body() {
    let genome = kethi();
    let daughter = genome.fork("kethi_deep", "Deep Kethi", 400);
    assert_eq!(daughter.embodiment, genome.embodiment);
}

/// Two runs, byte for byte (§9.4).
#[test]
fn the_sketch_is_deterministic() {
    let genome = kethi();
    assert_eq!(
        stem_genome::render_embodiment(&genome).expect("renders"),
        stem_genome::render_embodiment(&genome).expect("renders")
    );

    let path = fixture("luminous_kethi.ron");
    let path = path.to_str().expect("path");
    assert_eq!(
        stemma(&["embodiment", path]).stdout,
        stemma(&["embodiment", path]).stdout
    );
}

/// The applicability table covers every subsystem, always. A row that could go missing
/// would let a body quietly acquire machinery nobody checked it against.
#[test]
fn no_subsystem_is_ever_left_out_of_the_table() {
    for profile in [EmbodimentProfile::default(), kethi().embodiment] {
        let table = applicability(&profile);
        assert_eq!(table.len(), Subsystem::ALL.len());
        for subsystem in Subsystem::ALL {
            assert!(
                table.iter().any(|a| a.subsystem == subsystem),
                "{subsystem:?} is missing"
            );
        }
    }
}

/// The profile round-trips, and a misspelled field fails to load rather than silently
/// defaulting — the `deny_unknown_fields` rule, which matters more here than usual
/// because a dropped `vocal_tract` would flip every applicability answer.
#[test]
fn a_profile_round_trips_and_refuses_a_misspelled_field() {
    let genome = kethi();
    let path = std::env::temp_dir().join("stemma_m23_roundtrip.ron");
    stem_io::save(&path, &genome).expect("saves");
    let back: LanguageGenome = stem_io::load(&path).expect("reloads");
    assert_eq!(back.embodiment, genome.embodiment);

    let text = std::fs::read_to_string(&path).expect("readable");
    assert!(
        stem_io::load::<LanguageGenome>(&write_temp(
            "stemma_m23_typo.ron",
            &text.replace("channels:", "chanels:")
        ))
        .is_err(),
        "a misspelled field must fail to load"
    );
}

fn write_temp(name: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, text).expect("writes");
    path
}

/// `check_against_language` is quiet for an ordinary language, so nothing M23 added can
/// have moved any existing command's output.
#[test]
fn the_finding_is_silent_for_every_vocal_language() {
    for name in [
        "proto_asterian.ron",
        "asterian_attested.ron",
        "desert_asterian.ron",
        "grammar_asterian.ron",
        "written_asterian.ron",
    ] {
        let report: ValidationReport =
            stem_genome::embodiment_view::check_against_language(&load(name));
        assert!(report.issues.is_empty(), "`{name}`: {report}");
    }
}

/// The two consequence enums that drive the Kethi's Notes are the ones the fixture
/// declares — so the test above is checking the fixture rather than a default.
#[test]
fn the_fixture_declares_the_constraints_its_notes_are_drawn_from() {
    let genome = kethi();
    let mantle = genome
        .embodiment
        .channels
        .iter()
        .find(|c| c.id == "mantle")
        .expect("the light channel");
    assert_eq!(mantle.simultaneity, Simultaneity::Simultaneous);
    assert_eq!(mantle.persistence, Persistence::Fleeting);

    let ink = genome
        .embodiment
        .channels
        .iter()
        .find(|c| c.id == "ink")
        .expect("the chemical channel");
    assert_eq!(ink.persistence, Persistence::Persistent);
}
