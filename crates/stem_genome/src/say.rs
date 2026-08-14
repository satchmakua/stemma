//! Rendering a generated sentence and the constructions that built it (M18, §3.3).
//!
//! A **pure library renderer**, the `render_grammar` / `render_etymology` precedent
//! (`docs/adr/0006`): no map, no float, no clock, so two calls are byte-identical
//! and the M11 UI would render the identical text. It arranges nothing — the
//! ordering happened in `stem_syntax::generate`, and this prints the record it left.
//!
//! # The shape of the output is §10.2's, one level up
//!
//! A word trace answers *why does this word look like that?* with a rule per line.
//! A sentence trace answers *why are these words in this order?* with a construction
//! per line, each naming the profile parameter that decided it. The two read alike on
//! purpose: it is the same question about a bigger object.

use std::fmt::Write;

use stem_core::{Result, StemmaError};
use stem_syntax::Sentence;

use crate::LanguageGenome;
use crate::pad;

/// Says `proposition` in `genome`.
///
/// The whole of `stemma say`, so the CLI builds no sentence of its own: it parses a
/// string into a [`Proposition`] and asks for it. The generator reads this language's
/// own syntax profile, lexicon and morphology, which is why the same proposition in
/// two languages comes out differently — and why that difference is theirs rather
/// than the command's.
///
/// [`Proposition`]: stem_syntax::Proposition
pub fn say(genome: &LanguageGenome, proposition: &stem_syntax::Proposition) -> Result<Sentence> {
    stem_syntax::generate(
        proposition,
        &genome.syntax,
        &genome.lexicon,
        &genome.morphology,
    )
}

/// Renders a sentence: the string, its word-by-word gloss, and the record.
pub fn render_sentence(genome: &LanguageGenome, sentence: &Sentence) -> Result<String> {
    let inventory = &genome.phonemes;
    let mut out = String::new();

    writeln!(out, "{}", sentence.written(inventory)?).map_err(fmt_err)?;
    writeln!(out, "  {}  ·  {}", sentence.proposition, genome.name).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // The interlinear line: every word with its role, its source entry and its
    // descent class, so a reader can go from a sentence straight to `stemma trace`.
    let forms: Vec<String> = sentence
        .slots
        .iter()
        .map(|s| s.form.written(inventory))
        .collect::<Result<_>>()?;
    let width = forms.iter().map(|f| f.chars().count()).max().unwrap_or(0);
    let role_width = sentence
        .slots
        .iter()
        .map(|s| s.role.chars().count())
        .max()
        .unwrap_or(0);

    for (slot, form) in sentence.slots.iter().zip(&forms) {
        // The gloss comes from the lexicon entry, not from the slot: a slot stores an
        // identity, never a rendered label, so there is nothing here to go stale.
        let gloss = genome
            .lexicon
            .get(&slot.word)
            .and_then(|e| e.display_gloss())
            .unwrap_or("?");
        write!(
            out,
            "  {form}{}  {}{}  {gloss}",
            pad(form, width),
            slot.role,
            pad(slot.role, role_width)
        )
        .map_err(fmt_err)?;
        // What was attached to it, if anything — M8's record, read straight through.
        if let Some(affix) = slot
            .morphemes
            .iter()
            .find(|m| m.role != stem_lexicon::MorphemeRole::Stem)
        {
            write!(out, "-{}", affix.gloss).map_err(fmt_err)?;
        }
        writeln!(out, "  [{} · {}]", slot.word, slot.cognate_set).map_err(fmt_err)?;
    }

    // §3.3, for a sentence.
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "Constructions:").map_err(fmt_err)?;
    for (i, construction) in sentence.constructions.iter().enumerate() {
        writeln!(out, "  {i}  {}  {}", construction.id, construction.effect).map_err(fmt_err)?;
        writeln!(out, "     because {}", construction.because).map_err(fmt_err)?;
    }

    // What the generator could not do, stated rather than silently skipped.
    if !sentence.gaps.is_empty() {
        writeln!(out).map_err(fmt_err)?;
        writeln!(out, "Not done:").map_err(fmt_err)?;
        for gap in &sentence.gaps {
            writeln!(out, "  · {gap}").map_err(fmt_err)?;
        }
    }

    Ok(out)
}

/// `render_paradigm`'s policy verbatim: a `std::fmt::Error` in string building means
/// an allocation failure, not a domain error.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "sentence",
        message: "formatting a sentence into a string failed".to_owned(),
    }
}
