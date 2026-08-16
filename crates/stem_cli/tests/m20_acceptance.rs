//! The ROADMAP M20 acceptance tests: **`stemma write <lang> <word>` renders a word in
//! its script; the mapping is reported where it is lossy (an abjad drops vowels — that
//! is the point, and the tool says so rather than pretending round-trip).**
//!
//! # The clause that carries the milestone
//!
//! "rather than pretending round-trip". Writing a word in a script is a lookup table
//! and would be a morning's work; what makes it worth a milestone is that a script is
//! allowed to *lose* things, and a tool that hid the loss would be lying about the
//! language. There are two ways to lie here and Stemma must do neither:
//!
//! 1. **Invent the missing signs** so the round trip works. A comfortable abjad with
//!    vowel letters is an alphabet, and the author asked for an abjad.
//! 2. **Drop them quietly**, so `SSM` looks like a complete spelling of *sosem* and a
//!    reader believes they could read it back.
//!
//! So the fixture is one phonology under **three** scripts — an alphabet that writes
//! every sound, an abjad that writes only consonants, and a logography that writes no
//! sounds at all — and the tests below put the same word through each. Everything that
//! differs between the spellings is attributable to the mapping, exactly as M15's two
//! ecologies and M18's two grammars were built.
//!
//! The round-trip claim is *proved*, not printed: `covers` is replayed back into a
//! segment list and compared with the word's own segments. A message saying "can be
//! read back exactly" that nothing checked would be the third lie.
//!
//! The logography adds a fourth: a word it has **no sign for** was not written
//! *incompletely*, it was not written at all, and the two must not print the same
//! sentence.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stem_core::{PhonemeId, Validate};
use stem_genome::LanguageGenome;
use stem_script::{Mapping, ScriptKind, WritingSystem};

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

/// Written Asterian with its vocabulary coined — the reference inventory, three scripts.
fn written_asterian() -> LanguageGenome {
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

fn script<'a>(genome: &'a LanguageGenome, id: &str) -> &'a WritingSystem {
    stem_script::resolve(&genome.scripts, Some(id)).expect("the fixture declares it")
}

/// The word for `star`, resolved the way the CLI resolves it.
fn a_word(genome: &LanguageGenome) -> &stem_lexicon::WordEntry {
    genome
        .lexicon
        .by_meaning("star")
        .first()
        .copied()
        .expect("the reference concept list has STAR")
}

/// What a reader recovers from a spelling: the sounds the signs actually carried.
///
/// This is the honest definition of "round trips" — replay `covers` and see whether
/// you get the word back. A script round-trips when this equals the word's segments.
fn read_back(written: &stem_script::Written) -> Vec<PhonemeId> {
    written
        .glyphs
        .iter()
        .flat_map(|g| g.covers.iter().cloned())
        .collect()
}

// ------------------------------------------------------- the acceptance itself

/// **ROADMAP M20's acceptance.** One word, three scripts: the alphabet writes it whole,
/// the abjad drops the vowels, the logography writes none of them — and `stemma write`
/// says which situation you are in each time.
#[test]
fn one_word_in_three_scripts_and_the_tool_says_which_ones_lose() {
    let out = std::env::temp_dir().join("stemma_m20_written.ron");
    let out = out.display().to_string();
    let coined = stemma(&[
        "new-lexicon",
        fixture("written_asterian.ron").to_str().expect("path"),
        "--out",
        &out,
    ]);
    assert!(coined.status.success(), "{}", stdout(&coined));

    let alphabet = stdout(&stemma(&["write", &out, "star"]));
    let abjad = stdout(&stemma(&["write", &out, "star", "--script", "tirran"]));

    // The alphabet: everything written, and the tool volunteers that fact. A report
    // that only ever spoke up about *failure* would let silence read as completeness.
    assert!(
        alphabet.starts_with("sosem\n"),
        "the alphabet writes every sound of `sosem`: {alphabet}"
    );
    assert!(
        alphabet.contains("can be read back exactly"),
        "the lossless case must say so too: {alphabet}"
    );
    assert!(
        !alphabet.contains("Not written"),
        "nothing was lost, so nothing may be listed as lost: {alphabet}"
    );

    // The abjad: the consonants only, and the loss named.
    assert!(
        abjad.starts_with("SSM\n"),
        "the abjad writes s-s-m and no vowels: {abjad}"
    );
    assert!(
        abjad.contains("Not written:") && abjad.contains("/o/") && abjad.contains("/e/"),
        "the dropped vowels must be listed, not implied: {abjad}"
    );
    assert!(
        abjad.contains("by design"),
        "an abjad dropping vowels is the design, and must not read as a fault: {abjad}"
    );
    assert!(
        abjad.contains("does not round-trip, and is not meant to"),
        "M20's acceptance clause verbatim: {abjad}"
    );

    // And the third: one sign for the meaning, no sounds at all.
    let logography = stdout(&stemma(&["write", &out, "star", "--script", "emmen"]));
    assert!(
        logography.starts_with("★\n"),
        "the Emmen sign for STAR, and nothing spelled out: {logography}"
    );
    assert!(logography.contains("= STAR"), "{logography}");
    assert!(
        logography.contains("what a logography does"),
        "the third kind gets its own explanation, not the abjad's: {logography}"
    );

    // All three name the same word, so the columns are comparable.
    assert!(
        alphabet.contains("\"star\"")
            && abjad.contains("\"star\"")
            && logography.contains("\"star\""),
        "{abjad}"
    );
}

/// The round trip is **measured**, not announced. Replay what each spelling carried and
/// compare it with the word itself.
#[test]
fn the_round_trip_claim_is_true_of_the_alphabet_and_false_of_the_abjad() {
    let genome = written_asterian();
    let entry = a_word(&genome);
    let segments: Vec<PhonemeId> = entry.phonemic_form.segments().cloned().collect();

    let alphabet = stem_genome::write_word(&genome, entry, script(&genome, "kirran"));
    assert_eq!(
        read_back(&alphabet),
        segments,
        "an alphabet's spelling must reconstruct the word exactly"
    );
    assert!(!alphabet.is_lossy());

    let abjad = stem_genome::write_word(&genome, entry, script(&genome, "tirran"));
    assert_ne!(
        read_back(&abjad),
        segments,
        "if an abjad's spelling reconstructed the word, it would not be an abjad"
    );
    assert!(abjad.is_lossy());
    assert!(
        abjad.unwritten.iter().all(|u| u.expected),
        "every one of the abjad's gaps is a vowel it was never going to write"
    );
}

/// The tool must not invent the signs it does not have. The abjad has ten glyphs and
/// none of them is a vowel, so no vowel form may appear in a Tirran spelling — the
/// failure this milestone exists to prevent, checked over the whole lexicon rather
/// than over one convenient word.
#[test]
fn the_abjad_never_writes_a_vowel_anywhere_in_the_lexicon() {
    let genome = written_asterian();
    let tirran = script(&genome, "tirran");
    let vowels = ["a", "e", "i", "o", "u", "A", "E", "I", "O", "U"];

    for entry in genome.lexicon.iter() {
        let written = stem_genome::write_word(&genome, entry, tirran);
        let text = written.text();
        for vowel in vowels {
            assert!(
                !text.contains(vowel),
                "`{}` came out as `{text}`, which contains a vowel sign the Tirran \
                 abjad does not have",
                entry.id
            );
        }
    }
}

/// Every sound of every word is accounted for: written or listed as unwritten, never
/// simply gone. The invariant that makes the "Not written" list trustworthy.
#[test]
fn no_sound_is_ever_lost_without_being_named() {
    let genome = written_asterian();
    for script in &genome.scripts {
        for entry in genome.lexicon.iter() {
            let written = stem_genome::write_word(&genome, entry, script);
            let accounted = read_back(&written).len() + written.unwritten.len();
            assert_eq!(
                accounted,
                entry.phonemic_form.segments().count(),
                "`{}` in `{}`: {accounted} sound(s) accounted for out of {}",
                entry.id,
                script.id,
                entry.phonemic_form.segments().count()
            );
        }
    }
}

// ----------------------------------------------------------------- the report

/// A lossy script is **reported, never policed** (§17). The fixture carries an abjad
/// that cannot write five of its language's fifteen sounds and is still a valid file.
#[test]
fn a_script_that_cannot_write_the_language_is_a_note_and_not_an_error() {
    let genome: LanguageGenome = stem_io::load(fixture("written_asterian.ron")).expect("loads");
    let report = genome.validate();

    assert!(
        report.is_ok(),
        "a language whose script loses information is unusual, not broken: {report}"
    );
    // `script.` — the genome scopes a script's codes the way it scopes every other
    // sub-report's, so a reader knows which part of the language spoke.
    let note = report
        .issues
        .iter()
        .find(|i| i.code == "script.lossy_script")
        .unwrap_or_else(|| panic!("the abjad must be reported: {report}"));
    assert!(
        note.message
            .contains("that is the design rather than a fault"),
        "{}",
        note.message
    );
}

/// A *hole* in an alphabet is a different fact from an abjad's design, and the two must
/// not print the same. This is the whole reason `Unwritten::expected` exists.
#[test]
fn a_hole_in_an_alphabet_reads_as_a_gap_and_not_as_a_design() {
    let mut genome = written_asterian();
    let holed = &mut genome.scripts[0];
    assert_eq!(holed.kind, ScriptKind::Alphabet);
    holed
        .mappings
        .retain(|m| !matches!(m, Mapping::Phoneme { phoneme, .. } if phoneme.as_str() == "ph_o"));

    let report = stem_script::check_against_inventory(&genome.scripts[0], &genome.phonemes);
    assert!(
        report.warnings().any(|i| i.code == "unwritten_phoneme"),
        "an alphabet missing a vowel sign has a gap: {report}"
    );
    assert!(
        !report.issues.iter().any(|i| i.code == "lossy_script"),
        "a hole is not the abjad's design, and must not borrow its explanation: {report}"
    );

    let entry = a_word(&genome);
    let written = stem_genome::write_word(&genome, entry, &genome.scripts[0]);
    assert!(
        written.unwritten.iter().all(|u| !u.expected),
        "an alphabet is supposed to write its vowels"
    );
    assert!(
        stem_script::lossiness(&written, &genome.scripts[0]).contains("gap in the mapping"),
        "{}",
        stem_script::lossiness(&written, &genome.scripts[0])
    );
}

/// `stemma scripts` answers the question a reader has before writing anything: what can
/// each of these write? Including the answer "not these five sounds".
#[test]
fn the_script_list_says_what_each_one_can_carry() {
    let out = stdout(&stemma(&[
        "scripts",
        fixture("written_asterian.ron").to_str().expect("path"),
    ]));
    assert!(
        out.contains("The Kirran alphabet") && out.contains("(kirran, alphabet)"),
        "{out}"
    );
    assert!(out.contains("writes every sound in the language"), "{out}");
    assert!(
        out.contains("The Tirran abjad") && out.contains("(tirran, abjad)"),
        "{out}"
    );
    assert!(
        out.contains("does not write: /a/ /e/ /i/ /o/ /u/"),
        "the abjad's gaps are named individually, in inventory order: {out}"
    );

    // The logography is asked a different question. Listing all fifteen sounds as "not
    // written" would read as fifteen holes in a mapping that was never phonographic.
    assert!(
        out.contains("The Emmen signs") && out.contains("(emmen, logography)"),
        "{out}"
    );
    assert!(
        out.contains("writes 5 meaning(s), and no sounds at all"),
        "{out}"
    );
    assert!(
        !out.contains("does not write: /p/"),
        "a logography's sounds are not gaps in it: {out}"
    );
}

/// The third script writes **meanings**, and the fifth script kind is therefore built
/// rather than merely named. A `Mapping::Concept` nothing consumed would make
/// `kind: logography` a label that changed nothing observable — the scaffolding M19's
/// notes warn against, in a milestone whose own scope line names logography.
#[test]
fn a_logography_writes_the_meaning_and_says_it_carried_no_sounds() {
    let genome = written_asterian();
    let emmen = script(&genome, "emmen");
    let entry = a_word(&genome);

    let written = stem_genome::write_word(&genome, entry, emmen);
    assert_eq!(written.text(), "★", "one sign for the whole word");
    assert!(
        written.glyphs[0].covers.is_empty(),
        "a logogram stands for a meaning, not for a run of sounds"
    );
    assert_eq!(
        read_back(&written),
        Vec::<PhonemeId>::new(),
        "there is no pronunciation to read off a logogram"
    );
    assert!(
        written.unwritten.iter().all(|u| u.expected),
        "a logography not writing sounds is its design, consonants included"
    );

    let text = stem_genome::render_written(&genome, entry, emmen).expect("renders");
    assert!(
        text.contains("= STAR"),
        "the sign names its meaning: {text}"
    );
    assert!(text.contains("what a logography does"), "{text}");
}

/// A logography can only write the words that earned a sign. When it has none it wrote
/// **nothing** — which must not be reported as "unwritten by design", because that
/// would let a blank line pass for a spelling a reader could complete.
#[test]
fn a_word_the_logography_has_no_sign_for_is_told_it_was_not_written_at_all() {
    let genome = written_asterian();
    let emmen = script(&genome, "emmen");
    let king = genome
        .lexicon
        .by_meaning("king")
        .first()
        .copied()
        .expect("the fixture's gloss override puts `king` on MAN");

    let written = stem_genome::write_word(&genome, king, emmen);
    assert!(written.glyphs.is_empty(), "no sign, so nothing on the page");
    assert!(
        written.unwritten.iter().all(|u| !u.expected),
        "there was no sign, so there is no design to appeal to"
    );

    let text = stem_genome::render_written(&genome, king, emmen).expect("renders");
    assert!(
        text.contains("no sign in this script wrote any part of this word"),
        "an empty spelling must be named, not left as a blank line: {text}"
    );
    assert!(
        text.contains(
            "Nothing was written — which is not the same as something \
             written incompletely"
        ),
        "{text}"
    );
    assert!(
        !text.contains("what a logography does"),
        "an unwritable word must not borrow the design's explanation: {text}"
    );
}

/// A language with no script is told so plainly — the `render_grammar` precedent. An
/// unwritten language is the normal case in the world and must not look like an error.
#[test]
fn an_unwritten_language_is_told_so_rather_than_shown_an_empty_list() {
    let out = stdout(&stemma(&[
        "scripts",
        fixture("proto_asterian.ron").to_str().expect("path"),
    ]));
    assert!(out.contains("This language is unwritten"), "{out}");
    assert!(
        out.contains("Declare `scripts:` in the genome"),
        "and told what to do about it: {out}"
    );
}

// ------------------------------------------------------------- the invariants

/// A glyph is an **entity with an id**, not a character in a string (§7.6: "a glyph
/// should have ancestry just like a word"). Redrawing every sign leaves the mapping
/// intact and the identities unmoved — which is what M21's glyph descent will hang
/// from, and what would be impossible if `form` were the identity.
#[test]
fn a_glyph_keeps_its_identity_when_its_form_is_redrawn() {
    let mut genome = written_asterian();
    let entry_id = a_word(&genome).id.clone();
    let before = stem_genome::write_word(&genome, a_word(&genome), script(&genome, "tirran"));

    for glyph in &mut genome.scripts[1].glyphs {
        glyph.form = format!("{}\u{0301}", glyph.form); // the same signs, drawn differently
    }
    let entry = genome.lexicon.get(&entry_id).expect("still there");
    let after = stem_genome::write_word(&genome, entry, script(&genome, "tirran"));

    assert_eq!(
        before.glyphs.iter().map(|g| &g.glyph).collect::<Vec<_>>(),
        after.glyphs.iter().map(|g| &g.glyph).collect::<Vec<_>>(),
        "the same glyphs wrote the word before and after the redraw"
    );
    assert_ne!(before.text(), after.text(), "but the page looks different");
}

/// Writing is deterministic: same file, same word, same bytes (§9.4). No map iteration
/// on the way to the page.
#[test]
fn writing_the_same_word_twice_produces_identical_bytes() {
    let out = std::env::temp_dir().join("stemma_m20_determinism.ron");
    let out = out.display().to_string();
    assert!(
        stemma(&[
            "new-lexicon",
            fixture("written_asterian.ron").to_str().expect("path"),
            "--out",
            &out,
        ])
        .status
        .success()
    );

    let first = stemma(&["write", &out, "star", "--script", "tirran"]);
    let second = stemma(&["write", &out, "star", "--script", "tirran"]);
    assert_eq!(first.stdout, second.stdout, "two runs, byte for byte");
}

/// `scripts` is additive: a pre-M20 file loads, and saving it back writes no `scripts`
/// bytes. A user's saved language is their work — a schema change must not strand it,
/// and must not silently rewrite it either.
#[test]
fn a_pre_m20_file_loads_and_saves_without_gaining_a_scripts_block() {
    let genome: LanguageGenome = stem_io::load(fixture("proto_asterian.ron")).expect("loads");
    assert!(
        genome.scripts.is_empty(),
        "the reference language is unwritten"
    );

    let path = std::env::temp_dir().join("stemma_m20_roundtrip.ron");
    stem_io::save(&path, &genome).expect("saves");
    let text = std::fs::read_to_string(&path).expect("readable");
    assert!(
        !text.contains("scripts"),
        "an empty script list must not appear in the file at all"
    );

    let back: LanguageGenome = stem_io::load(&path).expect("reloads");
    assert_eq!(back.scripts, genome.scripts);
}

/// Naming a script that does not exist is an error, not a silent fall back to the
/// first. A language may have several, and writing in the wrong one quietly would be
/// worse than saying so.
#[test]
fn asking_for_a_script_this_language_lacks_fails_rather_than_guessing() {
    let output = stemma(&[
        "write",
        fixture("written_asterian.ron").to_str().expect("path"),
        "star",
        "--script",
        "ogham",
    ]);
    assert!(!output.status.success(), "{}", stdout(&output));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("ogham"),
        "the name that failed must appear: {err}"
    );
}
