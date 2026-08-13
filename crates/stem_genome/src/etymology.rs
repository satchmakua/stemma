//! Rendering a derived word's etymology — what it is made of (M14, `DESIGN.md`
//! §7.3, §10.2).
//!
//! A **pure library renderer**, the `render_paradigm` / `render_sense_history`
//! precedent (`docs/adr/0006`): built in memory, newline-terminated, no map, no
//! float, no clock, so two calls are byte-identical (§9.4) and the M11 UI renders
//! the identical text. It reads the word's stored [`BaseRef`]s and its own trace; it
//! runs no engine and re-derives nothing.
//!
//! # A word that was not derived renders nothing
//!
//! [`render_etymology`] returns `String::new()` when `bases` is empty, which is what
//! keeps every pre-M14 `stemma trace` output byte-identical — the same discipline
//! that let `render_sense_history` join `render_word_history` at M9 without moving
//! a single existing byte.
//!
//! # What it is for
//!
//! The milestone's second acceptance: *a sound change applied afterwards makes a
//! compound opaque — its parts no longer recoverable by eye but still recorded.*
//! This is where that becomes visible. Each part prints its composition form and,
//! when a rule has since eroded it, **what it became** — read through
//! [`stem_lexicon::WordEntry::morpheme_surface`], which walks the stored span
//! through the word's own trace. The eye cannot find `tira` inside `tirsula`; the
//! record can, and says which rule hid it.
//!
//! [`BaseRef`]: stem_lexicon::BaseRef

use std::fmt::Write;

use stem_core::{PhonemeId, Result, StemmaError};
use stem_lexicon::{MorphemeRole, WordEntry};
use stem_phonology::PhonemeInventory;

use crate::LanguageGenome;

/// The etymology of one word — **empty when the word was not derived**.
///
/// Errors only if a segment escapes the inventory, which `lexicon.unknown_phoneme`
/// already reports as an Error: rendering a missing segment as nothing would corrupt
/// the word (`WordEntry::written`'s policy, unchanged).
pub fn render_etymology(genome: &LanguageGenome, entry: &WordEntry) -> Result<String> {
    if entry.bases.is_empty() {
        return Ok(String::new());
    }
    let inventory = &genome.phonemes;
    let mut out = String::new();

    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "Formation:").map_err(fmt_err)?;

    // Parts in surface order. `bases` and `morphemes` are each already in span
    // order, so one merge by start index puts an affix in its real position —
    // a prefix before the base it attaches to, a suffix after.
    let mut parts: Vec<Part<'_>> = entry
        .bases
        .iter()
        .map(|b| Part {
            start: b.start,
            end: b.end,
            label: b.gloss.clone(),
            identity: format!("{} · {}", b.word, b.cognate_set),
            kind: "word",
        })
        .chain(entry.morphemes.iter().map(|m| Part {
            start: m.start,
            end: m.end,
            label: m.gloss.clone(),
            identity: m.morpheme.as_str().to_owned(),
            kind: match m.role {
                MorphemeRole::Prefix => "prefix",
                MorphemeRole::Suffix => "suffix",
                _ => "stem",
            },
        }))
        .collect();
    // Sort by span start. A stable sort over an authored-order `Vec`, so two
    // zero-width parts (which nothing produces today) keep their relative order
    // rather than depending on the comparator.
    parts.sort_by_key(|p| p.start);

    for part in &parts {
        // The part as it went in: a raw slice of the composition form, which is
        // `Derivation::input` once the word has evolved and `phonemic_form` before.
        let composed = romanize(inventory, composition_slice(entry, part.start, part.end))?;
        // The part as it comes out, walked through this word's own trace.
        let surface = romanize(
            inventory,
            entry.morpheme_surface(part.start as usize, part.end as usize),
        )?;

        write!(
            out,
            "  {composed:<12} {:<8} \"{}\"",
            part.kind,
            truncate(&part.label, 28)
        )
        .map_err(fmt_err)?;
        if surface != composed {
            // The point of the whole record: what erosion did to this part.
            write!(out, "  →  {surface}").map_err(fmt_err)?;
        }
        writeln!(out, "  [{}]", part.identity).map_err(fmt_err)?;
    }

    // The composed whole, then the surface whole, so the reader sees the distance
    // the word has travelled from its own parts.
    let composition = match &entry.trace {
        Some(trace) => romanize(inventory, trace.input.segments().cloned())?,
        None => entry.written(inventory)?,
    };
    let surface = entry.written(inventory)?;
    writeln!(out).map_err(fmt_err)?;
    // Three states, not two — [`WordEntry::trace`]'s own distinction. "No rule has
    // run" and "every rule ran and none of them touched it" are different facts
    // about a word, and collapsing them would make this renderer assert the first
    // about a word for which only the second is true.
    match (&entry.trace, surface == composition) {
        (None, _) => {
            writeln!(out, "  = {composition}  (no rule has run over it yet)").map_err(fmt_err)?
        }
        (Some(_), true) => writeln!(
            out,
            "  = {composition}  (exposed to every rule since, and changed by none — \
             the seam is still visible)"
        )
        .map_err(fmt_err)?,
        (Some(_), false) => writeln!(
            out,
            "  {composition}  →  {surface}   — the seam has eroded; the record above \
             is how the parts are still recoverable"
        )
        .map_err(fmt_err)?,
    }

    Ok(out)
}

/// One part of a composition, ready to print.
struct Part<'a> {
    start: u32,
    end: u32,
    label: String,
    identity: String,
    kind: &'a str,
}

/// The composition-form segments a span covers — the part **as it went in**.
///
/// `Derivation::input` is the composition form by construction (`MorphemeRef`'s
/// contract), and before any rule has run `phonemic_form` is. This is the
/// unevolved half of `morpheme_surface`'s invariant, kept beside it deliberately:
/// the two are the same span read at two times.
fn composition_slice(entry: &WordEntry, start: u32, end: u32) -> Vec<PhonemeId> {
    let source = match &entry.trace {
        Some(trace) => &trace.input,
        None => &entry.phonemic_form,
    };
    source
        .segments()
        .skip(start as usize)
        .take((end - start) as usize)
        .cloned()
        .collect()
}

/// `render_paradigm`'s helper, by value: a span's segments arrive owned from
/// `morpheme_surface`, so this takes `PhonemeId` rather than `&PhonemeId`.
fn romanize(
    inventory: &PhonemeInventory,
    segments: impl IntoIterator<Item = PhonemeId>,
) -> Result<String> {
    let mut out = String::new();
    for id in segments {
        out.push_str(inventory.require(&id)?.written());
    }
    Ok(out)
}

/// Keeps a long gloss from ragging the columns. Truncates on a char boundary and
/// says it did, rather than silently printing a different meaning.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let head: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

/// `render_paradigm`'s policy verbatim: a `std::fmt::Error` in string building means
/// an allocation failure, not a domain error.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "etymology",
        message: "formatting an etymology into a string failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_lexicon::{
        CompoundPair, ConceptKey, DerivationPattern, Formation, Lexicon, PartOfSpeech, WordEntry,
        WordSource, derive, scoped_cognate_set,
    };
    use stem_phonology::{Phoneme, PhonemeInventory, Root, SegmentKind, Syllable};

    fn syllable(pattern: &str, segments: &[&str]) -> Syllable {
        Syllable {
            pattern: pattern.to_owned(),
            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
            stress: None,
        }
    }

    fn genome() -> LanguageGenome {
        LanguageGenome::proto("x", "X").with_phonemes(PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel),
            Phoneme::new("ph_r", "r", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_s", "s", SegmentKind::Consonant),
            Phoneme::new("ph_u", "u", SegmentKind::Vowel),
            Phoneme::new("ph_l", "l", SegmentKind::Consonant),
        ]))
    }

    fn word(ordinal: usize, concept: &str, gloss: &str, syllables: Vec<Syllable>) -> WordEntry {
        WordEntry {
            id: stem_core::WordId::sequential(ordinal),
            concept: Some(ConceptKey::new(concept)),
            phonemic_form: Root { syllables },
            glosses: vec![gloss.to_owned()],
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: scoped_cognate_set(&stem_core::LanguageId::new("x"), ordinal),
            source: WordSource::Generated,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: Vec::new(),
            sense_history: None,
        }
    }

    fn compounded() -> (LanguageGenome, WordEntry) {
        let lexicon = Lexicon::from_entries([
            word(
                1,
                "STAR",
                "star",
                vec![
                    syllable("CV", &["ph_t", "ph_i"]),
                    syllable("CV", &["ph_r", "ph_a"]),
                ],
            ),
            word(
                2,
                "STONE",
                "stone",
                vec![
                    syllable("CV", &["ph_s", "ph_u"]),
                    syllable("CV", &["ph_l", "ph_a"]),
                ],
            ),
        ]);
        let pattern = DerivationPattern {
            id: "C".to_owned(),
            name: "Compound".to_owned(),
            formation: Formation::Compound {
                pairs: vec![CompoundPair {
                    left: ConceptKey::new("STAR"),
                    right: ConceptKey::new("STONE"),
                }],
            },
            gloss: "{1}-{2}".to_owned(),
            part_of_speech: PartOfSpeech::Noun,
            limit: None,
            note: String::new(),
        };
        let coined =
            derive(&[pattern], &[], &lexicon, &stem_core::LanguageId::new("x")).expect("derives");
        let entry = coined.into_iter().next().expect("one compound");
        (genome().with_lexicon(lexicon), entry)
    }

    #[test]
    fn a_word_that_was_not_derived_renders_nothing() {
        let genome = genome();
        let plain = word(1, "STAR", "star", vec![syllable("CV", &["ph_t", "ph_a"])]);
        assert_eq!(
            render_etymology(&genome, &plain).expect("renders"),
            "",
            "pre-M14 trace output must not move by a byte"
        );
    }

    #[test]
    fn a_compound_prints_each_part_with_its_word_and_cognate_set() {
        let (genome, entry) = compounded();
        let text = render_etymology(&genome, &entry).expect("renders");
        assert!(text.contains("Formation:"), "{text}");
        assert!(text.contains("tira"), "the left base's form: {text}");
        assert!(text.contains("sula"), "the right base's form: {text}");
        assert!(
            text.contains("\"star\"") && text.contains("\"stone\""),
            "{text}"
        );
        assert!(text.contains("w_0001"), "the part is addressable: {text}");
        assert!(
            text.contains("cog_x_0001"),
            "cognate-visible, which is the ROADMAP's word: {text}"
        );
        assert!(text.contains("no rule has run"), "{text}");
    }

    #[test]
    fn rendering_an_etymology_twice_produces_identical_bytes() {
        let (genome, entry) = compounded();
        assert_eq!(
            render_etymology(&genome, &entry).expect("renders"),
            render_etymology(&genome, &entry).expect("renders")
        );
    }

    /// **ROADMAP M14's second acceptance, rendered.** After erosion the surface no
    /// longer contains `tira`, and the etymology says what it became.
    #[test]
    fn an_eroded_compound_prints_what_each_part_became() {
        use stem_lexicon::{Derivation, RuleApplication, SiteTrace};

        let (genome, mut entry) = compounded();
        entry.trace = Some(Derivation {
            input: entry.phonemic_form.clone(),
            steps: vec![RuleApplication {
                rule: stem_core::RuleId::new("r_syncope"),
                index: 0,
                sites: vec![SiteTrace {
                    at: 3,
                    before: PhonemeId::new("ph_a"),
                    after: None,
                    resolution: None,
                    left: vec![],
                    right: vec![],
                    emptied_syllable: None,
                }],
                blocked: vec![],
            }],
        });
        entry.phonemic_form = entry.trace.as_ref().unwrap().final_form();

        let text = render_etymology(&genome, &entry).expect("renders");
        assert!(
            !entry.written(&genome.phonemes).unwrap().contains("tira"),
            "the seam must really have eroded, or this test proves nothing"
        );
        assert!(
            text.contains("→"),
            "the part must print what it became: {text}"
        );
        assert!(
            text.contains("tir "),
            "`tira` lost its vowel and the record says so: {text}"
        );
        assert!(text.contains("the seam has eroded"), "{text}");
    }

    /// [`WordEntry::trace`]'s three states, kept distinct here because two of them
    /// look identical from the outside and mean different things. A word that went
    /// through every rule and was moved by none has a **history**; saying "no rule
    /// has run over it yet" about that word is simply false, and it was false in the
    /// first draft of this renderer.
    #[test]
    fn a_compound_exposed_to_rules_and_unchanged_says_so_rather_than_claiming_none_ran() {
        use stem_lexicon::Derivation;

        let (genome, mut entry) = compounded();
        let untouched = render_etymology(&genome, &entry).expect("renders");
        assert!(
            untouched.contains("no rule has run over it yet"),
            "{untouched}"
        );

        // Exposed to the whole sequence, moved by none of it — an empty-steps trace.
        entry.trace = Some(Derivation {
            input: entry.phonemic_form.clone(),
            steps: vec![],
        });
        let exposed = render_etymology(&genome, &entry).expect("renders");
        assert!(
            exposed.contains("changed by none"),
            "an empty-steps trace is a history, not the absence of one: {exposed}"
        );
        assert!(
            !exposed.contains("no rule has run over it yet"),
            "that claim is false of this word: {exposed}"
        );
    }
}
