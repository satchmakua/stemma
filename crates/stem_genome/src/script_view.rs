//! Rendering a word in a script, and the script itself (M20, `DESIGN.md` §7.6).
//!
//! A **pure library renderer**, the `render_grammar` / `render_culture` precedent
//! (`docs/adr/0006`): no map, no float, no clock, so two calls are byte-identical and
//! the M11 UI would render identical text.
//!
//! # It prints the loss, every time
//!
//! ROADMAP M20's acceptance turns on the tool *saying* that an abjad spelling cannot
//! be read back. So the sound-by-sound line below shows which sounds reached the page
//! and which did not, and the closing sentence states whether the spelling round-trips
//! — including when it does. A report that only ever spoke up about failure would let
//! a reader assume silence meant completeness.

use std::fmt::Write;

use stem_core::{Result, StemmaError};
use stem_script::{Glyph, Mapping, ScriptKind, WritingSystem, Written, lossiness};

use crate::LanguageGenome;
use crate::pad;

/// Writes one word in one of `genome`'s scripts, with the record of what was lost.
pub fn render_written(
    genome: &LanguageGenome,
    entry: &stem_lexicon::WordEntry,
    script: &WritingSystem,
) -> Result<String> {
    let inventory = &genome.phonemes;
    let written = write_word(genome, entry, script);
    let mut out = String::new();

    writeln!(out, "{}", written.text()).map_err(fmt_err)?;
    writeln!(
        out,
        "  {}  \"{}\"  ·  {} ({})",
        entry.written(inventory)?,
        entry.display_gloss().unwrap_or("?"),
        script.name,
        script.kind.name()
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // Sign by sign: what each one is, and which sounds it carried. The `covers` list
    // is what makes a syllabary legible — one sign, two sounds — and it comes from
    // the writer rather than being recomputed here.
    for glyph in &written.glyphs {
        // A logographic sign covers no sounds — it stands for the meaning — so the
        // column names the meaning instead of printing empty. Blank would look like a
        // sign that carried nothing, which is the opposite of what a logogram is.
        let sounds: String = if glyph.covers.is_empty() {
            match &entry.concept {
                Some(key) => format!("= {key}"),
                None => "—".to_owned(),
            }
        } else {
            glyph
                .covers
                .iter()
                .map(|id| {
                    inventory
                        .get(id)
                        .map(|p| format!("/{}/", p.ipa))
                        .unwrap_or_else(|| format!("<{id}>"))
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        writeln!(
            out,
            "  {}{}  {}{}  [{}]",
            glyph.form,
            pad(&glyph.form, 4),
            sounds,
            pad(&sounds, 12),
            glyph.glyph
        )
        .map_err(fmt_err)?;
    }

    // A word no sign wrote leaves the loop above with nothing to print, and a silent
    // gap where the spelling goes reads as a rendering bug rather than as a fact about
    // the script. Say it instead. The spelling line stays genuinely empty: putting a
    // placeholder character there would be inventing a sign.
    if written.glyphs.is_empty() {
        writeln!(
            out,
            "  (no sign in this script wrote any part of this word)"
        )
        .map_err(fmt_err)?;
    }

    // And the sounds that never reached the page — the half a lossy script hides.
    if written.is_lossy() {
        writeln!(out).map_err(fmt_err)?;
        writeln!(out, "  Not written:").map_err(fmt_err)?;
        for gap in &written.unwritten {
            let ipa = inventory
                .get(&gap.phoneme)
                .map(|p| p.ipa.clone())
                .unwrap_or_else(|| gap.phoneme.to_string());
            writeln!(
                out,
                "    /{ipa}/  {}",
                if gap.expected {
                    "— by design"
                } else {
                    "— no sign for it in this script"
                }
            )
            .map_err(fmt_err)?;
        }
    }

    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "  {}", lossiness(&written, script)).map_err(fmt_err)?;
    Ok(out)
}

/// Renders a language's writing systems: what they are and what they can carry.
///
/// Never empty: a language with no script is told so, for the reason `render_grammar`
/// tells a language with no syntax profile.
pub fn render_scripts(genome: &LanguageGenome) -> Result<String> {
    let mut out = String::new();
    writeln!(out, "Writing — {}", genome.name).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    if genome.scripts.is_empty() {
        writeln!(
            out,
            "  This language is unwritten. It has a phonology, a lexicon and"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  a grammar, but nothing yet says how any of it is set down."
        )
        .map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
        writeln!(out, "  Declare `scripts:` in the genome to give it one.").map_err(fmt_err)?;
        return Ok(out);
    }

    for script in &genome.scripts {
        writeln!(
            out,
            "  {}  ({}, {})",
            script.name,
            script.id,
            script.kind.name()
        )
        .map_err(fmt_err)?;
        if !script.note.is_empty() {
            writeln!(out, "    {}", script.note).map_err(fmt_err)?;
        }
        writeln!(out, "    {} sign(s)", script.glyphs.len()).map_err(fmt_err)?;

        // Which of the language's sounds it can write — the question a reader has,
        // answered from the same measurement validation uses.
        //
        // A logography is asked a different question, because listing all fifteen
        // sounds as "not written" would read as fifteen holes in a mapping that was
        // never phonographic. It writes meanings; the useful number is how many.
        if script.kind == ScriptKind::Logography {
            let meanings = script
                .mappings
                .iter()
                .filter(|m| matches!(m, Mapping::Concept { .. }))
                .count();
            writeln!(
                out,
                "    writes {meanings} meaning(s), and no sounds at all — a word with \
                 no sign of its own cannot be written in it"
            )
            .map_err(fmt_err)?;
        } else {
            let written = script.written_phonemes();
            let unwritten: Vec<String> = genome
                .phonemes
                .iter()
                .filter(|p| !written.contains(&&p.id))
                .map(|p| format!("/{}/", p.ipa))
                .collect();
            if unwritten.is_empty() {
                writeln!(out, "    writes every sound in the language").map_err(fmt_err)?;
            } else {
                writeln!(out, "    does not write: {}", unwritten.join(" ")).map_err(fmt_err)?;
            }
        }

        // M21: has the spelling fallen behind the pronunciation? A finding, measured
        // from the lexicon — and stated in both directions, so a script that has kept
        // up says so rather than being silent about it.
        let drift = stem_script::script_drift(script, &genome.lexicon, &genome.phonemes);
        if genome.lexicon.is_empty() {
            writeln!(
                out,
                "    no lexicon yet, so nothing can be said about whether the spelling \
                 has kept up"
            )
            .map_err(fmt_err)?;
        } else if !script.writes_sound() {
            // It cannot fall behind a pronunciation it never encoded. Saying "the
            // spelling still matches" here would be the milestone's own lie: it would
            // credit a script with keeping up in a race it is not running.
            writeln!(
                out,
                "    writes no sounds, so no pronunciation can leave it behind"
            )
            .map_err(fmt_err)?;
        } else if !drift.is_historical() {
            writeln!(out, "    the spelling still matches the pronunciation").map_err(fmt_err)?;
        } else {
            if !drift.unwritable.is_empty() {
                let sounds: Vec<String> = drift
                    .unwritable
                    .iter()
                    .map(|id| {
                        genome
                            .phonemes
                            .get(id)
                            .map(|p| format!("/{}/", p.ipa))
                            .unwrap_or_else(|| id.to_string())
                    })
                    .collect();
                writeln!(
                    out,
                    "    the language moved on: no sign for {} ({} word(s) affected)",
                    sounds.join(" "),
                    drift.affected_words
                )
                .map_err(fmt_err)?;
            }
            if !drift.fossils.is_empty() {
                let signs: Vec<String> = drift
                    .fossils
                    .iter()
                    .map(|id| {
                        script
                            .glyph(id)
                            .map(|g| g.form.clone())
                            .unwrap_or_else(|| id.to_string())
                    })
                    .collect();
                writeln!(
                    out,
                    "    signs that outlived their sound: {}",
                    signs.join(" ")
                )
                .map_err(fmt_err)?;
            }
        }
        writeln!(out).map_err(fmt_err)?;
    }
    Ok(out)
}

/// Walks one glyph back to its pictogram (M21, §7.6).
///
/// The glyph analogue of `stemma trace`, and deliberately the same shape: the recorded
/// stages oldest first, then the present. `Derivation` prints its `input` and then the
/// steps that moved it; this prints the stages and then what the sign is **now**,
/// because the present lives in `Glyph::form` and the script's mappings rather than in
/// the history (`Glyph::history`'s docs say why).
///
/// # The independence line
///
/// §7.6's claim is that a glyph's history and a word's history come apart. So after the
/// biography this prints what the *engine* found: whether the sounds this sign is
/// recorded as having written are still spoken. That half is measured from the lexicon,
/// never authored — the M19 discipline.
pub fn render_glyph_trace(
    genome: &LanguageGenome,
    script: &WritingSystem,
    glyph: &Glyph,
) -> Result<String> {
    let mut out = String::new();
    let name = if glyph.name.is_empty() {
        String::new()
    } else {
        format!(" — {}", glyph.name)
    };
    writeln!(out, "{}  {}{}", glyph.form, glyph.id, name).map_err(fmt_err)?;
    writeln!(out, "  {} ({})", script.name, script.kind.name()).map_err(fmt_err)?;
    if !glyph.note.is_empty() {
        writeln!(out, "  {}", glyph.note).map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    if glyph.history.is_empty() {
        writeln!(
            out,
            "  no history recorded — this sign has no past on file; `history:` on the \
             glyph is what writes one"
        )
        .map_err(fmt_err)?;
        return Ok(out);
    }

    // Oldest first, so reading down the page is reading forwards in time — the order
    // `stemma trace` prints a derivation in, for the same reason.
    let width = glyph
        .history
        .iter()
        .map(|s| s.role.name().chars().count())
        .max()
        .unwrap_or(0)
        .max("now".len());
    for (i, stage) in glyph.history.iter().enumerate() {
        // An empty `form` means the shape did not change at this stage; showing the
        // current one would claim a redraw that never happened, so it shows nothing.
        let form = if stage.form.is_empty() {
            ""
        } else {
            &stage.form
        };
        writeln!(
            out,
            "  {i}  {}{}  {}{}  {}",
            stage.role.name(),
            pad(stage.role.name(), width),
            form,
            pad(form, 3),
            stage_value(genome, stage)
        )
        .map_err(fmt_err)?;
        if !stage.note.is_empty() {
            writeln!(out, "     {}{}", pad("", width), stage.note).map_err(fmt_err)?;
        }
    }

    // And the present, which is not a stage — it is the glyph itself.
    writeln!(
        out,
        "  →  now{}  {}{}  {}",
        pad("now", width),
        glyph.form,
        pad(&glyph.form, 3),
        present_value(genome, script, glyph)
    )
    .map_err(fmt_err)?;

    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "  {}", independence(genome, script, glyph)).map_err(fmt_err)?;
    Ok(out)
}

/// What a stage stood for: a sound, a meaning, both, or neither.
fn stage_value(genome: &LanguageGenome, stage: &stem_script::GlyphStage) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(id) = &stage.wrote {
        parts.push(
            genome
                .phonemes
                .get(id)
                .map(|p| format!("/{}/", p.ipa))
                .unwrap_or_else(|| format!("<{id}>")),
        );
    }
    if let Some(key) = &stage.meant {
        parts.push(format!("= {key}"));
    }
    if parts.is_empty() {
        // A determinative is silent and means nothing on its own; saying so is more
        // informative than a blank column.
        "(neither sound nor meaning)".to_owned()
    } else {
        parts.join("  ")
    }
}

/// What the sign stands for **now**, read from the script's mappings rather than from
/// any stored field — the one source of truth for the present.
fn present_value(genome: &LanguageGenome, script: &WritingSystem, glyph: &Glyph) -> String {
    let mut parts: Vec<String> = Vec::new();
    for mapping in &script.mappings {
        match mapping {
            Mapping::Phoneme { phoneme, glyph: g } if g == &glyph.id => parts.push(
                genome
                    .phonemes
                    .get(phoneme)
                    .map(|p| format!("/{}/", p.ipa))
                    .unwrap_or_else(|| format!("<{phoneme}>")),
            ),
            Mapping::Sequence { phonemes, glyph: g } if g == &glyph.id => {
                let run: Vec<String> = phonemes
                    .iter()
                    .map(|id| {
                        genome
                            .phonemes
                            .get(id)
                            .map(|p| p.ipa.clone())
                            .unwrap_or_else(|| id.to_string())
                    })
                    .collect();
                parts.push(format!("/{}/", run.join("")));
            }
            Mapping::Concept { concept, glyph: g } if g == &glyph.id => {
                parts.push(format!("= {concept}"))
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        "(nothing maps to it — it is written by no rule of this script)".to_owned()
    } else {
        parts.join("  ")
    }
}

/// The §7.6 independence sentence: has this sign outlived its sound?
///
/// Measured against the **lexicon**, never the inventory — `apply_rules` only grows an
/// inventory, so a phoneme stays in it long after the last word containing it changed.
/// Stated in every case, including when nothing has come apart, for the reason
/// `lossiness` states the lossless case: a line that only appeared on bad news would let
/// silence read as a clean bill of health.
fn independence(genome: &LanguageGenome, script: &WritingSystem, glyph: &Glyph) -> String {
    let drift = stem_script::script_drift(script, &genome.lexicon, &genome.phonemes);
    let ipa = |id: &stem_core::PhonemeId| {
        genome
            .phonemes
            .get(id)
            .map(|p| format!("/{}/", p.ipa))
            .unwrap_or_else(|| id.to_string())
    };

    if drift.fossils.contains(&glyph.id) {
        let once: Vec<String> = glyph
            .sounds_it_once_wrote()
            .iter()
            .map(|i| ipa(i))
            .collect();
        let past = if once.is_empty() {
            String::new()
        } else {
            format!(" It is recorded writing {} in the past.", once.join(" "))
        };
        return format!(
            "This sign has outlived its sound: no word of {} contains what it writes any \
             more, and it is still on the page.{past}",
            genome.name
        );
    }

    // A logogram writes no sound, so it has none to outlive. That independence is
    // §7.6's point rather than an omission, and it gets its own sentence. The question
    // is asked of `stem_script` because `Mapping` is `#[non_exhaustive]`.
    if !script.glyph_writes_sound(&glyph.id) {
        return "This sign writes no sound, so no sound change can strand it — which is \
                why signs like it outlast the pronunciations around them."
            .to_owned();
    }
    format!(
        "The sound this sign writes is still spoken in {} — its history and the \
         language's have not yet come apart.",
        genome.name
    )
}

/// The `Written` for a word, for a caller that wants the data rather than the text.
pub fn write_word(
    genome: &LanguageGenome,
    entry: &stem_lexicon::WordEntry,
    script: &WritingSystem,
) -> Written {
    stem_script::write_with_inventory(
        &entry.phonemic_form,
        entry.concept.as_ref(),
        script,
        &genome.phonemes,
    )
}

/// `render_paradigm`'s policy verbatim: a `std::fmt::Error` in string building means
/// an allocation failure, not a domain error.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "script",
        message: "formatting a written form into a string failed".to_owned(),
    }
}
