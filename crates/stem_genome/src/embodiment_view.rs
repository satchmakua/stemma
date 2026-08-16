//! What a language's speakers are, and what Stemma can do about it (M23, §18).
//!
//! A **pure library renderer** — the `render_grammar` / `script_view` precedent
//! (`docs/adr/0006`): no map, no float, no clock, no RNG, so two calls are
//! byte-identical.
//!
//! # The finding, not just the table
//!
//! [`stem_embodiment::applicability`] answers "does this subsystem apply to this body?"
//! from the profile alone. That is a statement about what *should* happen. The half
//! that needs the genome is what *did*: a bioluminescent species with no vocal tract
//! and a five-vowel inventory has been handed vowels by machinery that does not apply
//! to it, and ROADMAP M23's acceptance is precisely that Stemma says so **rather than
//! silently producing** them.
//!
//! So [`check_against_language`] compares the two and reports the overlap. The profile
//! proposes; the engine observes — M19's discipline, in its fourth outing.

use std::fmt::Write;

use stem_core::{Issue, Result, Severity, StemmaError, ValidationReport};
use stem_embodiment::{Applies, EmbodimentProfile, Subsystem, applicability};

use crate::LanguageGenome;
use crate::pad;

/// What this language's speakers are, and which machinery applies to them.
pub fn render_embodiment(genome: &LanguageGenome) -> Result<String> {
    let profile = &genome.embodiment;
    let mut out = String::new();
    writeln!(out, "Embodiment — {}", genome.name).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    if profile.is_empty() {
        // Not an error, and not a gap to be filled in. Every language in this project
        // has speakers with mouths and none of them says so, because Stemma's whole
        // model is built for that body.
        writeln!(
            out,
            "  Nothing is declared about these speakers, so Stemma assumes what it is"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  built for: a vocal tract, moving air. Every subsystem applies."
        )
        .map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
        writeln!(
            out,
            "  Declare `embodiment:` in the genome to describe a different body."
        )
        .map_err(fmt_err)?;
        return Ok(out);
    }

    if !profile.note.is_empty() {
        writeln!(out, "  {}", profile.note).map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
    }

    // --- the body.
    writeln!(
        out,
        "  Vocal tract: {}",
        match &profile.vocal_tract {
            Some(tract) if !tract.note.is_empty() => tract.note.clone(),
            Some(_) => "yes".to_owned(),
            None => "none".to_owned(),
        }
    )
    .map_err(fmt_err)?;
    if profile.social_cognition.structure != stem_embodiment::SocialStructure::Unspecified {
        writeln!(
            out,
            "  Speakers: {}{}",
            profile.social_cognition.structure.label(),
            match profile.social_cognition.speakers_per_utterance {
                0 | 1 => String::new(),
                n => format!(", {n} to a complete utterance"),
            }
        )
        .map_err(fmt_err)?;
    }
    for manipulator in &profile.manipulators {
        writeln!(
            out,
            "  Manipulators: {} × {}  (dexterity {})",
            manipulator.count,
            manipulator.name,
            manipulator.dexterity.label()
        )
        .map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    // --- the channels, with §18.2's constraints. The six printed-only ones are
    //     labelled as printed-only, so a reader is never left thinking a band they
    //     filled in drives something it does not.
    writeln!(out, "  Channels").map_err(fmt_err)?;
    if profile.channels.is_empty() {
        writeln!(out, "    none declared").map_err(fmt_err)?;
    }
    for channel in &profile.channels {
        writeln!(
            out,
            "    {}  ({}, {})",
            channel.name,
            channel.id,
            channel.kind.label()
        )
        .map_err(fmt_err)?;
        if !channel.note.is_empty() {
            writeln!(out, "      {}", channel.note).map_err(fmt_err)?;
        }
        writeln!(
            out,
            "      persistence {}   simultaneity {}   direction {}   privacy {}",
            channel.persistence.label(),
            channel.simultaneity.label(),
            channel.directionality.label(),
            channel.privacy.label()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "      bandwidth {}   range {}   energy {}   learnability {}   noise {}   salience {}",
            channel.bandwidth.label(),
            channel.range.label(),
            channel.energy_cost.label(),
            channel.learnability.label(),
            channel.noise.label(),
            channel.cultural_salience.label()
        )
        .map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    // --- the acceptance clause: what applies, and what does not.
    writeln!(out, "  What Stemma can do with this body").map_err(fmt_err)?;
    let table = applicability(profile);
    let width = table
        .iter()
        .map(|a| a.subsystem.label().chars().count())
        .max()
        .unwrap_or(0);
    let verdict_width = table
        .iter()
        .map(|a| a.verdict.label().chars().count())
        .max()
        .unwrap_or(0);
    for row in &table {
        writeln!(
            out,
            "    {}{}  {}{}  {}",
            row.subsystem.label(),
            pad(row.subsystem.label(), width),
            row.verdict.label(),
            pad(row.verdict.label(), verdict_width),
            row.because
        )
        .map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    // --- and what this language has actually used.
    let findings = check_against_language(genome);
    if findings.issues.is_empty() {
        writeln!(
            out,
            "  Nothing in this language uses machinery that does not apply to it."
        )
        .map_err(fmt_err)?;
    } else {
        writeln!(out, "  But this language has used it anyway:").map_err(fmt_err)?;
        for issue in &findings.issues {
            writeln!(out, "    {issue}").map_err(fmt_err)?;
        }
    }

    Ok(out)
}

/// What this language has that its speakers' bodies cannot account for.
///
/// **ROADMAP M23's acceptance clause, as a finding.** The applicability table says
/// which machinery does not apply; this says whether it was used regardless — because
/// "reports which machinery does not apply *rather than silently producing vowels*"
/// only means something if the vowels are named when they are there.
///
/// Everything is a Warning or a Note. A half-converted language is a normal state to be
/// in — an author gives a species a body before rewriting its inventory — and refusing
/// to load it would make the conversion impossible to do a step at a time.
pub fn check_against_language(genome: &LanguageGenome) -> ValidationReport {
    let mut report = ValidationReport::new();
    let profile = &genome.embodiment;
    if profile.is_empty() || profile.has_vocal_tract() {
        return report;
    }

    // The vowels themselves. This is the sentence the acceptance is written around.
    let vowels = genome.phonemes.vowels().count();
    let consonants = genome.phonemes.consonants().count();
    if vowels + consonants > 0 {
        report.push(
            Issue::new(
                Severity::Warning,
                "vocal_inventory_without_a_vocal_tract",
                format!(
                    "these speakers have no vocal tract, and this language declares \
                     {consonants} consonant(s) and {vowels} vowel(s) — sounds made by \
                     an anatomy the profile says they do not have"
                ),
            )
            .about(&genome.id),
        );
    }

    if !genome.phonotactics.is_empty() {
        report.warn(
            "syllable_templates_without_syllables",
            "this language declares root templates, which describe syllables; a signal \
             on a non-vocal channel has none",
        );
    }
    if !genome.prosody.is_unspecified() {
        report.warn(
            "stress_without_syllables",
            "this language declares a stress policy, and stress is a property of a \
             syllable",
        );
    }
    if !genome.lexicon.is_empty() {
        report.note(
            "lexicon_built_from_vocal_machinery",
            format!(
                "this language's {} word(s) were coined from a phoneme inventory and \
                 syllable templates; they are placeholders for signals M24 will model, \
                 not things these speakers can say",
                genome.lexicon.len()
            ),
        );
    }
    if !genome.applied_rules.is_empty() {
        report.note(
            "sound_changes_over_a_silent_channel",
            format!(
                "{} sound change(s) are recorded; the engine transformed segments that \
                 stand for nothing these speakers produce",
                genome.applied_rules.len()
            ),
        );
    }

    report
}

/// The subsystems this body cannot use, for a caller that wants the data.
pub fn unavailable(profile: &EmbodimentProfile) -> Vec<Subsystem> {
    applicability(profile)
        .into_iter()
        .filter(|a| a.verdict == Applies::No || a.verdict == Applies::Unbuilt)
        .map(|a| a.subsystem)
        .collect()
}

/// `render_paradigm`'s policy verbatim.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "embodiment",
        message: "formatting an embodiment sketch into a string failed".to_owned(),
    }
}
