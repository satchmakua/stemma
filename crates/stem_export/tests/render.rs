//! Rendering tests, in two tiers.
//!
//! The **canary** is hand-built from four phonemes and two entries. No fixture
//! edit can move its bytes, so a change there means the *renderer* changed.
//!
//! The **goldens** are whatever the reference fixture produces. Every legitimate
//! weight edit moves them, and re-baselining is a normal act.
//!
//! Keeping the two distinguishable is what stops re-baselining from becoming a
//! reflex that eventually waves through a real regression — the lesson M1 recorded
//! about corpus digests versus data-free canaries.

use std::path::{Path, PathBuf};

use stem_core::{CognateSetId, LanguageId, PhonemeId, WordId};
use stem_export::{
    write_lexicon_csv, write_lexicon_csv_header, write_lexicon_csv_rows, write_lexicon_markdown,
};
use stem_genome::LanguageGenome;
use stem_lexicon::{
    CONCEPTS, ConceptKey, Lexicon, PartOfSpeech, WordEntry, WordSource, build_proto_lexicon,
};
use stem_phonology::{
    Phoneme, PhonemeInventory, Phonotactics, Root, SegmentKind, Syllable, WeightedSyllableCount,
    WeightedTemplate,
};

// ---------------------------------------------------------------- the canary

fn syllable(pattern: &str, segments: &[&str]) -> Syllable {
    Syllable {
        pattern: pattern.to_owned(),
        segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
        // The canary is a hand-built genome that never went through a rule, so it
        // is honestly unanalysed for stress.
        stress: None,
    }
}

/// Four phonemes, two entries, nothing drawn. `/j/` romanises as `y`, which is the
/// point: it exercises the romanisation/IPA split that a same-looking inventory
/// would hide.
fn canary() -> LanguageGenome {
    let genome = LanguageGenome::proto("demo", "Demo")
        .with_seed(7)
        .with_phonemes(PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            Phoneme::new("ph_j", "j", SegmentKind::Consonant).with_romanization("y"),
        ]))
        .with_phonotactics(Phonotactics {
            templates: vec![WeightedTemplate::new("CVC"), WeightedTemplate::new("CV")],
            syllables_per_root: vec![WeightedSyllableCount::new(1), WeightedSyllableCount::new(2)],
        });

    genome.with_lexicon(Lexicon::from_entries([
        WordEntry {
            id: WordId::new("w_0001"),
            concept: Some(ConceptKey::new("ALL")),
            phonemic_form: Root {
                syllables: vec![syllable("CVC", &["ph_t", "ph_a", "ph_k"])],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Determiner,
            cognate_set: CognateSetId::new("cog_demo_0001"),
            source: WordSource::Generated,
            trace: None,
        },
        WordEntry {
            id: WordId::new("w_0002"),
            concept: Some(ConceptKey::new("ASH")),
            phonemic_form: Root {
                syllables: vec![
                    syllable("CV", &["ph_j", "ph_a"]),
                    syllable("CV", &["ph_k", "ph_a"]),
                ],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: CognateSetId::new("cog_demo_0002"),
            source: WordSource::Generated,
            trace: None,
        },
    ]))
}

fn markdown_of(genome: &LanguageGenome) -> String {
    let mut out = String::new();
    write_lexicon_markdown(&mut out, genome).expect("renders");
    out
}

fn csv_of(genome: &LanguageGenome) -> String {
    let mut out = String::new();
    write_lexicon_csv(&mut out, genome).expect("renders");
    out
}

#[test]
fn the_canary_dictionary_matches_its_frozen_bytes() {
    let expected = "\
# Demo — lexicon

`demo` · proto-language · seed 7

2 words over 2 concepts, seeded from the built-in Swadesh-style concept list.
Forms are romanised; IPA in slashes.

| Form | IPA | Gloss | POS | Concept | Word | Cognate set |
| --- | --- | --- | --- | --- | --- | --- |
| tak | /tak/ | all | determiner | `ALL` | `w_0001` | `cog_demo_0001` |
| yaka | /jaka/ | ashes | noun | `ASH` | `w_0002` | `cog_demo_0002` |

## Etymology

Demo is the root of its family, so no entry has an ancestor and no sound change
has applied. Every form above was **coined** — drawn from the phoneme inventory
under this language's phonotactic templates, on the `lexicon` RNG stream at seed
7, one root per concept in concept-list order.

`stemma new-lexicon` on this language reproduces every form in this document.
";
    assert_eq!(markdown_of(&canary()), expected);
}

#[test]
fn the_canary_csv_matches_its_frozen_bytes() {
    let expected = "\
\"ID\",\"Language_ID\",\"Parameter_ID\",\"Concepticon_ID\",\"Form\",\"Segments\",\"Template\",\"Gloss\",\"Part_Of_Speech\",\"Cognateset_ID\",\"Stemma_Source\"
\"demo-w_0001\",\"demo\",\"ALL\",\"98\",\"tak\",\"t a k\",\"CVC\",\"all\",\"determiner\",\"cog_demo_0001\",\"generated\"
\"demo-w_0002\",\"demo\",\"ASH\",\"646\",\"yaka\",\"j a k a\",\"CV.CV\",\"ashes\",\"noun\",\"cog_demo_0002\",\"generated\"
";
    assert_eq!(csv_of(&canary()), expected);
}

// ---------------------------------------------------------------- properties

#[test]
fn rendering_the_same_lexicon_twice_produces_identical_bytes() {
    let genome = canary();
    assert_eq!(markdown_of(&genome), markdown_of(&genome));
    assert_eq!(csv_of(&genome), csv_of(&genome));
}

#[test]
fn no_exported_document_contains_a_carriage_return() {
    // Asserted on the rendered bytes rather than on a golden file, so a checkout
    // with mangled line endings cannot mask it.
    for document in [markdown_of(&canary()), csv_of(&canary())] {
        assert!(!document.contains('\r'), "a CR reached the output");
    }
}

#[test]
fn no_exported_document_starts_with_a_byte_order_mark() {
    for document in [markdown_of(&canary()), csv_of(&canary())] {
        assert!(!document.starts_with('\u{feff}'));
    }
}

#[test]
fn the_csv_header_is_a_cldf_form_table() {
    let mut header = String::new();
    write_lexicon_csv_header(&mut header).expect("renders");
    for column in [
        "ID",
        "Language_ID",
        "Parameter_ID",
        "Form",
        "Segments",
        "Cognateset_ID",
    ] {
        assert!(header.contains(&format!("\"{column}\"")), "{header}");
    }
    // CLDF's `Source` is a bibliographic reference; reusing the name would produce
    // a file that looks conformant and is not.
    assert!(header.contains("\"Stemma_Source\""));
    assert!(!header.contains("\"Source\""));
}

#[test]
fn the_csv_row_count_is_the_lexicon_entry_count() {
    let genome = canary();
    let lines = csv_of(&genome).lines().count();
    assert_eq!(lines, genome.lexicon.len() + 1, "entries plus one header");
}

/// The M5 shape: several languages concatenated into one table emit one header.
#[test]
fn two_languages_rendered_into_one_table_emit_one_header() {
    let mut out = String::new();
    write_lexicon_csv_header(&mut out).expect("header");
    write_lexicon_csv_rows(&mut out, &canary()).expect("rows");

    let mut sister = canary();
    sister.id = LanguageId::new("demo2");
    write_lexicon_csv_rows(&mut out, &sister).expect("rows");

    assert_eq!(out.matches("\"Language_ID\"").count(), 1, "one header only");
    assert_eq!(out.lines().count(), 5, "1 header + 2 + 2 rows");
    // The composed ID is what keeps the two languages' `w_0001` apart.
    assert!(out.contains("\"demo-w_0001\"") && out.contains("\"demo2-w_0001\""));
}

#[test]
fn a_dictionary_renders_in_lexicon_order_not_alphabetical_order() {
    // `tak` (ALL) precedes `yaka` (ASH) because the concept list does, and nothing
    // sorts. Reversing the entries must reverse the document.
    let mut reversed = canary();
    let entries: Vec<WordEntry> = reversed.lexicon.iter().rev().cloned().collect();
    reversed.lexicon = Lexicon::from_entries(entries);

    let document = markdown_of(&reversed);
    let yaka = document.find("yaka").expect("present");
    let tak = document.find("| tak ").expect("present");
    assert!(
        yaka < tak,
        "row order must follow the lexicon, not the alphabet"
    );
}

// ---------------------------------------------------------------- nasty input

#[test]
fn a_field_containing_a_comma_a_quote_and_a_newline_round_trips() {
    let mut genome = canary();
    let mut entries: Vec<WordEntry> = genome.lexicon.iter().cloned().collect();
    entries[0].glosses = vec!["a \"loud\", odd\ngloss".to_owned()];
    genome.lexicon = Lexicon::from_entries(entries);

    let csv = csv_of(&genome);
    // Doubled quotes, never a backslash; the embedded newline stays inside quotes.
    assert!(csv.contains(r#""a ""loud"", odd"#), "{csv}");
    assert!(!csv.contains('\\'), "escaping is doubling, not backslashes");
}

#[test]
fn a_gloss_containing_a_pipe_does_not_break_the_table() {
    let mut genome = canary();
    let mut entries: Vec<WordEntry> = genome.lexicon.iter().cloned().collect();
    entries[0].glosses = vec!["all | every".to_owned()];
    genome.lexicon = Lexicon::from_entries(entries);

    let document = markdown_of(&genome);
    let row = document
        .lines()
        .find(|l| l.contains("all \\| every"))
        .unwrap_or_else(|| panic!("{document}"));
    // Seven columns means eight pipes; the escaped one must not add a ninth.
    assert_eq!(
        row.match_indices('|').count() - row.matches("\\|").count(),
        8,
        "the escaped pipe leaked a column: {row}"
    );
}

// ------------------------------------------------- validator/engine agreement

/// Nothing that merely warns may make an export refuse. The second direction of
/// the defect M1's review caught.
#[test]
fn a_lexicon_that_only_warns_still_exports() {
    let mut genome = canary();
    let mut entries: Vec<WordEntry> = genome.lexicon.iter().cloned().collect();
    entries[0].concept = Some(ConceptKey::new("OBSIDIAN")); // warns: unknown concept
    entries[0].glosses = vec!["obsidian".to_owned()];
    genome.lexicon = Lexicon::from_entries(entries);

    let document = markdown_of(&genome);
    assert!(document.contains("obsidian"), "its own gloss is used");
    assert!(document.contains("`OBSIDIAN`"), "the raw key is shown");

    let csv = csv_of(&genome);
    // Unresolvable concept, so no anchor: an empty field, not a fabricated one.
    assert!(csv.contains(r#""OBSIDIAN","""#), "{csv}");
}

#[test]
fn an_entry_referencing_a_missing_phoneme_fails_the_export_rather_than_dropping_a_column() {
    let mut genome = canary();
    let mut entries: Vec<WordEntry> = genome.lexicon.iter().cloned().collect();
    entries[0].phonemic_form = Root {
        syllables: vec![syllable("CV", &["ph_zzz", "ph_a"])],
    };
    genome.lexicon = Lexicon::from_entries(entries);

    let mut out = String::new();
    assert!(
        write_lexicon_markdown(&mut out, &genome).is_err(),
        "a form that cannot be rendered must be an error, not a silent gap"
    );
}

#[test]
fn a_language_with_no_lexicon_renders_a_document_that_says_so() {
    let empty = LanguageGenome::proto("bare", "Bare");
    let document = markdown_of(&empty);
    assert!(document.contains("# Bare — lexicon"));
    assert!(document.contains("no lexicon yet"), "{document}");
}

// ---------------------------------------------------------------- goldens

fn fixture_genome_with_lexicon() -> LanguageGenome {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/proto_asterian.ron");
    let genome: LanguageGenome = stem_io::load(&path).expect("the reference fixture loads");
    let lexicon = build_proto_lexicon(
        &genome.id,
        &genome.phonemes,
        &genome.phonotactics,
        CONCEPTS,
        genome.seed,
    )
    .expect("the fixture seeds a lexicon");
    genome.with_lexicon(lexicon)
}

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

/// Re-baseline by running `stemma new-lexicon … --out` then `export-md`, but only
/// after checking the canary above is still green: the canary moving means the
/// renderer changed, and only a fixture edit legitimately moves this.
#[test]
fn the_fixture_dictionary_matches_its_golden_file() {
    let rendered = markdown_of(&fixture_genome_with_lexicon());
    let expected = std::fs::read_to_string(golden("proto_asterian.md")).expect("golden exists");
    assert_eq!(rendered, expected.replace("\r\n", "\n"));
}

#[test]
fn the_fixture_csv_matches_its_golden_file() {
    let rendered = csv_of(&fixture_genome_with_lexicon());
    let expected = std::fs::read_to_string(golden("proto_asterian.csv")).expect("golden exists");
    assert_eq!(rendered, expected.replace("\r\n", "\n"));
}

#[test]
fn the_fixture_dictionary_holds_one_row_per_concept() {
    let genome = fixture_genome_with_lexicon();
    let document = markdown_of(&genome);
    let rows = document
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| --- ") && !l.starts_with("| Form "))
        .count();
    assert_eq!(rows, CONCEPTS.len());
    // The meanings ROADMAP M5's own command names must all be printed.
    for gloss in ["water", "sun", "star", "king", "mother"] {
        assert!(
            document.contains(&format!("| {gloss} |")),
            "missing `{gloss}`"
        );
    }
}
