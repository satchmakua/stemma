//! The ROADMAP M21 acceptance tests: **`stemma glyph-trace <lang> <glyph>` walks a
//! glyph back to its pictogram; a language whose spelling froze while its pronunciation
//! moved shows the resulting gap, and §17's script-history row — M7's last deferred
//! dimension — finally scores.**
//!
//! # The clause that carries the milestone
//!
//! §7.6: *a glyph should have ancestry just like a word* — and the two are
//! **independent**. The second half is the whole claim. A biography attached to a sign
//! is a `Vec` with prose in it, and any file format can hold one; what makes this
//! script *evolution* is that the sign's history and the language's history are
//! measured separately and observed to come apart.
//!
//! So the fixture does it the only honest way: `rules_glide_loss.sc` contains two
//! ordinary sound changes that **have never heard of the script**. They name feature
//! bundles, they would run identically on an unwritten language, and nothing in them
//! mentions a glyph. What they do to the Kirran alphabet is then *found*:
//!
//! - `the_sign_outlived_its_sound_and_the_engine_found_it` — glide loss deletes every
//!   /w/, and the letter `w` is left writing a sound no word contains. The finding
//!   comes from the **lexicon**, and a test below proves that by checking the phoneme
//!   is still sitting in the inventory while the finding fires anyway.
//! - `the_spelling_could_not_keep_up_with_the_pronunciation` — intervocalic voicing
//!   mints /b/ /d/ /ɡ/, which the alphabet has no letters for.
//! - `nothing_in_the_change_file_mentions_the_script` — greps the rule file. If the
//!   drift were authored rather than measured, this is the test that would catch it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::{PhonemeId, Validate};
use stem_genome::{LanguageGenome, ScriptHistory};
use stem_script::{DEEP_ORTHOGRAPHY, GlyphRole, WritingSystem, script_drift};

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

/// The rules, read through M10's DSL parser — the same front end the CLI uses.
fn rules() -> stem_soundchange::RuleSet {
    let path = fixture("rules_glide_loss.sc");
    let source = std::fs::read_to_string(&path).expect("the rule file is readable");
    stem_soundchange::parse_rule_set(&source, &path.display().to_string())
        .expect("the rule set parses")
}

/// Written Asterian with its vocabulary coined — three scripts, spelling still exact.
fn proto() -> LanguageGenome {
    let genome: LanguageGenome = stem_io::load(fixture("written_asterian.ron")).expect("loads");
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

/// The same language 450 years later, after two changes that ignored the script.
fn late() -> LanguageGenome {
    let (evolved, _) = proto()
        .evolve("asterian_late", "Late Written Asterian", &rules(), 450)
        .expect("evolves");
    evolved
}

fn script<'a>(genome: &'a LanguageGenome, id: &str) -> &'a WritingSystem {
    stem_script::resolve(&genome.scripts, Some(id)).expect("the fixture declares it")
}

// ------------------------------------------------- half one: the biography walks

/// **ROADMAP M21's acceptance, first clause.** The trace walks back to the pictogram —
/// and the fixture's chain is §7.6's worked example, stage for stage.
#[test]
fn a_glyph_walks_back_to_its_pictogram() {
    let out = stdout(&stemma(&[
        "glyph-trace",
        fixture("written_asterian.ron").to_str().expect("path"),
        "ge_star",
    ]));

    // §7.6's chain, in order: pictogram → logogram → determinative → rebus → simplified
    // manuscript form. The present (`= STAR`) is printed after them and is not a stage.
    for rung in [
        "pictogram",
        "logogram",
        "determinative",
        "rebus sign",
        "phonogram",
    ] {
        assert!(
            out.contains(rung),
            "§7.6's chain is missing `{rung}`: {out}"
        );
    }
    let at = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {out}"))
    };
    assert!(
        at("pictogram") < at("logogram")
            && at("logogram") < at("determinative")
            && at("determinative") < at("rebus sign"),
        "oldest first, so reading down the page is reading forwards in time: {out}"
    );
    assert!(
        at("rebus sign") < at("→  now"),
        "the present comes last, after every recorded stage: {out}"
    );
}

/// `history[0]` is the pictogram and **the present is not in it** — the `Derivation`
/// shape. A stage repeating the current form would be the second source of truth M2
/// banned when it refused to store a rendered string beside `phonemic_form`.
#[test]
fn the_present_is_not_one_of_the_recorded_stages() {
    let genome = proto();
    let (_, star) =
        stem_script::resolve_glyph(&genome.scripts, "ge_star", None).expect("the fixture has it");

    assert_eq!(
        star.origin().expect("a pictogram").role,
        GlyphRole::Pictogram,
        "the oldest stage is where the walk stops"
    );
    assert_eq!(star.form, "★", "the present shape lives on the glyph");
    assert!(
        star.history.iter().all(|s| s
            .meant
            .as_ref()
            .is_none_or(|k| !(k.as_str() == "STAR" && s.role == GlyphRole::Unspecified))),
        "no stage is a copy of the present"
    );
    // The last recorded stage is a phonogram for /s/; the sign's job *today* is
    // logographic. That the two differ is the point — the present is read from the
    // script's mappings, never from the history.
    assert_eq!(
        star.history.last().expect("stages").wrote,
        Some(PhonemeId::new("ph_s")),
        "the last recorded job is not the current one"
    );
}

/// An empty `form` on a stage means *the shape did not change*, and must not be filled
/// in with the current one — that would claim a redraw that never happened.
#[test]
fn a_stage_that_changed_only_the_job_does_not_claim_a_new_shape() {
    let genome = proto();
    let (_, star) =
        stem_script::resolve_glyph(&genome.scripts, "ge_star", None).expect("the fixture has it");

    let logogram = star
        .history
        .iter()
        .find(|s| s.role == GlyphRole::Logogram)
        .expect("§7.6's second rung");
    let pictogram = star.origin().expect("the first");
    assert_eq!(
        logogram.form, pictogram.form,
        "becoming a logogram changed the job, not the drawing"
    );
    assert_ne!(
        logogram.form, star.form,
        "and it is not the modern shape either"
    );
}

/// A sign with no recorded past says so plainly rather than printing an empty block.
#[test]
fn a_glyph_with_no_history_is_told_so() {
    let out = stdout(&stemma(&[
        "glyph-trace",
        fixture("written_asterian.ron").to_str().expect("path"),
        "g_t",
        "--script",
        "kirran",
    ]));
    assert!(out.contains("no history recorded"), "{out}");
    assert!(
        out.contains("`history:` on the glyph is what writes one"),
        "and is told what would write one: {out}"
    );
}

// ------------------------------------- half two: the two histories come apart

/// **ROADMAP M21's acceptance, second clause.** A sound change that never heard of the
/// script leaves a letter writing a sound nobody says any more.
#[test]
fn the_sign_outlived_its_sound_and_the_engine_found_it() {
    let genome = late();
    let kirran = script(&genome, "kirran");
    let drift = script_drift(kirran, &genome.lexicon, &genome.phonemes);

    assert!(
        drift.fossils.iter().any(|g| g.as_str() == "g_w"),
        "glide loss stranded the letter `w`: {:?}",
        drift.fossils
    );
    assert!(
        drift.fossils.iter().any(|g| g.as_str() == "g_y"),
        "and the letter `y` with it: {:?}",
        drift.fossils
    );

    let text =
        stem_genome::render_glyph_trace(&genome, kirran, kirran.glyph(&"g_w".into()).unwrap())
            .expect("renders");
    assert!(
        text.contains("has outlived its sound"),
        "§7.6's claim, in the tool's own words: {text}"
    );
    assert!(
        text.contains("recorded writing /w/ in the past"),
        "and it names the sound it used to write: {text}"
    );
}

/// **The trap this milestone had to avoid.** `apply_rules` only ever *grows* an
/// inventory — a phoneme stays in it after the last word containing it changed, so
/// earlier trace steps keep resolving. Asking the inventory "does this language still
/// have /w/?" therefore answers *yes* forever, and the fossil finding would never fire.
///
/// This test pins that the measurement comes from the **lexicon** by asserting both
/// halves at once: /w/ is still in the inventory, and the sign is a fossil anyway.
#[test]
fn the_finding_comes_from_the_lexicon_and_not_from_the_inventory() {
    let genome = late();
    let w = PhonemeId::new("ph_w");

    assert!(
        genome.phonemes.get(&w).is_some(),
        "the inventory keeps /w/ — that is what makes the naive check wrong"
    );
    assert!(
        !genome
            .lexicon
            .iter()
            .any(|e| e.phonemic_form.segments().any(|s| *s == w)),
        "but no word contains it any more"
    );

    let drift = script_drift(script(&genome, "kirran"), &genome.lexicon, &genome.phonemes);
    assert!(
        drift.fossils.iter().any(|g| g.as_str() == "g_w"),
        "so the sign is a fossil, found from the word list"
    );
}

/// The other direction: the language innovates sounds the script has no signs for.
#[test]
fn the_spelling_could_not_keep_up_with_the_pronunciation() {
    let genome = late();
    let drift = script_drift(script(&genome, "kirran"), &genome.lexicon, &genome.phonemes);

    for minted in ["ph_b", "ph_d", "ph_g"] {
        assert!(
            drift.unwritable.iter().any(|p| p.as_str() == minted),
            "intervocalic voicing minted {minted}, and the alphabet has no letter for it: {:?}",
            drift.unwritable
        );
    }
    assert!(
        drift.affected_words > 0,
        "and real words are affected, not just the inventory"
    );

    let out = stdout(&stemma(&["scripts", &write_late()]));
    assert!(
        out.contains("the language moved on: no sign for"),
        "`stemma scripts` reports the gap: {out}"
    );
    assert!(
        out.contains("signs that outlived their sound:"),
        "and the fossils beside it: {out}"
    );
}

/// **The independence is real, not staged.** Nothing in the change file mentions a
/// script, a glyph, or a spelling — it names feature bundles, and it would run
/// identically on a language nobody had ever written down.
///
/// If the drift were authored rather than measured, this is the test that would catch
/// it: the M19 discipline, applied to Phase 6.
#[test]
fn nothing_in_the_change_file_mentions_the_script() {
    let source = std::fs::read_to_string(fixture("rules_glide_loss.sc")).expect("readable");

    // Scanned: the DIRECTIVES — the lines the parser turns into a rule. Not scanned:
    // `//` commentary and `note:` prose, both of which explain to a human what these
    // changes will do to the scripts, and neither of which the engine reads. The claim
    // is that no rule can *condition* on a spelling, not that nobody may mention one.
    //
    // The structural version of the same claim, and the one that lasts, is
    // `stem_soundchange`'s `the_engine_never_references_script`: the crate names no
    // script type at all, so a rule *cannot* consult a glyph even in principle.
    let directives: String = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with("note:"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "glyph", "script", "spell", "kirran", "tirran", "emmen", "g_w",
    ] {
        assert!(
            !directives.to_lowercase().contains(forbidden),
            "a sound change that knew about `{forbidden}` would be staging the result:\n{directives}"
        );
    }
}

/// The proto is not drifted — so the finding is a *difference*, not a constant. Without
/// this, every assertion above would pass on a measure that always said "drifted".
#[test]
fn the_same_script_had_not_drifted_before_the_sound_changes_ran() {
    let genome = proto();
    let drift = script_drift(script(&genome, "kirran"), &genome.lexicon, &genome.phonemes);

    assert!(
        !drift.is_historical(),
        "the alphabet was cut for this language and still fits it: {drift:?}"
    );
    assert_eq!(drift.distance(), 0);
    assert_eq!(
        genome.plausibility_profile().script_history,
        ScriptHistory::Phonemic
    );
}

// --------------------------------------------------- half three: §17's last row

/// **ROADMAP M21's acceptance, third clause.** §17's script-history row scores, and the
/// deferred list is down to one.
#[test]
fn the_script_history_row_finally_scores() {
    let out = stdout(&stemma(&["profile", &write_late()]));

    assert!(out.contains("Script history"), "{out}");
    assert!(
        out.contains("historical"),
        "the drifted daughter reads as historical: {out}"
    );
    assert!(
        !out.contains("§7.6"),
        "script history left the not-yet-modelled block: {out}"
    );
    // M21 left one deferred dimension, alien embodiment (§18). **M23 filled it**, so
    // the block is empty now — asserted as the absence of §7.6 rather than the
    // presence of §18, because the second would have to be rewritten again the moment
    // the list changed, which is how a test goes quietly dishonest.
    assert!(
        !out.contains("§7.6"),
        "script history left the not-yet-modelled block at M21: {out}"
    );
    // The raw basis rides along, so the band can be checked rather than trusted — the
    // `recorded_changes` discipline.
    assert!(
        out.contains("kirran"),
        "the per-script counts are shown: {out}"
    );
}

/// The band agrees with the Note through one shared constant (`docs/adr/0009`, now on
/// its fourth instance). A projection test: `Deep` holds exactly when
/// `deep_orthography` fires.
#[test]
fn the_deep_band_and_the_deep_note_cannot_disagree() {
    let genome = late();
    let profile = genome.plausibility_profile();
    let worst = profile
        .script_drift
        .iter()
        .map(|(_, n)| *n)
        .max()
        .expect("three scripts");
    let report = genome.validate();
    let note_fired = report
        .issues
        .iter()
        .any(|i| i.code == "script.deep_orthography");

    assert_eq!(
        profile.script_history == ScriptHistory::Deep,
        note_fired,
        "band and Note read the same constant: worst {worst}, bar {DEEP_ORTHOGRAPHY}"
    );
    assert_eq!(
        worst >= DEEP_ORTHOGRAPHY,
        note_fired,
        "and both agree with the measurement"
    );
    // M8's rule: the tool's own showcase must sit BELOW the extreme bar, so the band
    // reads `historical` and the Note stays quiet.
    assert_eq!(profile.script_history, ScriptHistory::Historical);
    assert!(!note_fired);
}

/// A drifted orthography is **reported, never policed** (§17). English is deep and is
/// not thereby broken.
#[test]
fn a_spelling_that_has_fallen_behind_is_never_an_error() {
    let genome = late();
    let report = genome.validate();

    assert!(
        report.is_ok(),
        "an orthography drifting from its pronunciation is the normal fate of writing: {report}"
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "script.sign_outlived_its_sound"),
        "but it is reported: {report}"
    );
    assert!(
        report
            .warnings()
            .any(|i| i.code == "script.spelling_behind_pronunciation"),
        "{report}"
    );
}

/// A logography writes meanings, so no pronunciation can leave it behind. Reporting it
/// as `phonemic` would claim every sound has a sign in a script with no sound signs at
/// all — and the three places that could say it must all say the true thing.
#[test]
fn a_logography_cannot_fall_behind_a_pronunciation() {
    let genome = late();
    let emmen = script(&genome, "emmen");
    assert!(!emmen.writes_sound());

    let drift = script_drift(emmen, &genome.lexicon, &genome.phonemes);
    assert!(!drift.is_historical(), "nothing to fall behind: {drift:?}");

    let list = stem_genome::render_scripts(&genome).expect("renders");
    assert!(
        list.contains("writes no sounds, so no pronunciation can leave it behind"),
        "{list}"
    );

    let star = emmen.glyph(&"ge_star".into()).expect("the star");
    let trace = stem_genome::render_glyph_trace(&genome, emmen, star).expect("renders");
    assert!(
        trace.contains("writes no sound, so no sound change can strand it"),
        "{trace}"
    );
    assert!(
        !trace.contains("still spoken"),
        "a sign that writes no sound has no sound to still be spoken: {trace}"
    );
}

// ------------------------------------------------------------- the invariants

/// `history` is additive: an M20 file with no glyph biographies loads, and saving it
/// back writes no `history` bytes.
#[test]
fn a_pre_m21_glyph_loads_and_saves_without_gaining_a_history() {
    let genome = proto();
    let kirran = script(&genome, "kirran");
    let plain = kirran.glyph(&"g_t".into()).expect("an ordinary letter");
    assert!(plain.history.is_empty());

    let text = serde_json::to_string(plain).expect("serialise");
    assert!(
        !text.contains("history"),
        "an empty history must not appear in the file at all: {text}"
    );
    let back: stem_script::Glyph = serde_json::from_str(&text).expect("deserialise");
    assert_eq!(back, *plain);
}

/// A glyph id is script-scoped, so an unqualified one can be ambiguous. Naming the
/// scripts and asking is better than tracing the wrong sign's biography.
#[test]
fn an_ambiguous_glyph_id_is_refused_rather_than_guessed() {
    let mut genome = proto();
    // Give the abjad a sign under the alphabet's id — legitimate, since ids are scoped.
    genome.scripts[1].glyphs.push(stem_script::Glyph {
        id: "g_w".into(),
        form: "Ẇ".to_owned(),
        name: String::new(),
        note: String::new(),
        history: Vec::new(),
    });

    let err = stem_script::resolve_glyph(&genome.scripts, "g_w", None).expect_err("ambiguous");
    let message = err.to_string();
    assert!(
        message.contains("kirran") && message.contains("tirran"),
        "{message}"
    );
    assert!(
        message.contains("--script"),
        "and says how to disambiguate: {message}"
    );

    // Named, it resolves.
    let (system, _) =
        stem_script::resolve_glyph(&genome.scripts, "g_w", Some("tirran")).expect("named");
    assert_eq!(system.id, "tirran");
}

/// Two runs, byte for byte (§9.4). No map on the way to the page.
#[test]
fn tracing_the_same_glyph_twice_produces_identical_bytes() {
    let path = write_late();
    let first = stemma(&["glyph-trace", &path, "g_w", "--script", "kirran"]);
    let second = stemma(&["glyph-trace", &path, "g_w", "--script", "kirran"]);
    assert_eq!(first.stdout, second.stdout);
    assert!(first.status.success());
}

/// Builds the drifted daughter on disk once per test that needs the CLI, and returns
/// its path. Written to the temp dir, not into the repo.
fn write_late() -> String {
    let proto_path = std::env::temp_dir().join("stemma_m21_proto.ron");
    let late_path = std::env::temp_dir().join("stemma_m21_late.ron");
    stem_io::save(&proto_path, &proto()).expect("saves");
    stem_io::save(&late_path, &late()).expect("saves");
    late_path.display().to_string()
}
