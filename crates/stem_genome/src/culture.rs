//! Rendering why a language has the vocabulary it has (M15, `DESIGN.md` §7.5).
//!
//! A **pure library renderer**, the `render_paradigm` / `render_etymology`
//! precedent (`docs/adr/0006`): built in memory, newline-terminated, no map, no
//! float, no clock, so two calls are byte-identical (§9.4) and the M11 UI renders
//! identical text. It reads the genome's profile and its concept list; it coins
//! nothing and re-derives nothing.
//!
//! # What it is for
//!
//! ROADMAP M15's acceptance turns on one clause: *each gap is **explained in the
//! report** rather than silently empty.* A missing word is invisible by construction
//! — you cannot notice the absence of something you were never shown — so the only
//! way a gap can be a claim rather than an accident is if something prints it.
//! This is that something.
//!
//! Every absence is listed with the trait that explains it and the reason that trait
//! gave, and every elaboration with the distinctions the author named. A reader who
//! wants to know why this language has no word for the sea gets an answer, from the
//! file, in one command.

use std::fmt::Write;

use stem_core::{Result, StemmaError};
use stem_lexicon::{Meaning, Salience, salience, shaping_counts};

use crate::LanguageGenome;
use crate::pad;

/// Renders `genome`'s environment and culture profile.
///
/// Never empty: a language with no profile gets the honest statement that it has
/// none, and what that means — which is more useful than silence, because "this
/// language has every meaning on the built-in list" is itself a claim about its
/// speakers, just an undeliberate one.
pub fn render_culture(genome: &LanguageGenome) -> Result<String> {
    let profile = &genome.environment;
    let meanings = stem_lexicon::meanings(&genome.concepts);
    let mut out = String::new();

    writeln!(out, "Environment & culture — {}", genome.name).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    if profile.is_empty() {
        writeln!(
            out,
            "  No culture profile. This language coins all {} available meanings,",
            meanings.len()
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  which is itself a claim about its speakers — just not a deliberate one."
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  Declare `environment:` in the genome to say what these people elaborate"
        )
        .map_err(fmt_err)?;
        writeln!(out, "  and what they have no word for, and why.").map_err(fmt_err)?;
        return Ok(out);
    }

    if !profile.summary.is_empty() {
        writeln!(out, "  {}", profile.summary).map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
    }

    for culture_trait in &profile.traits {
        writeln!(out, "  {}  ({})", culture_trait.name, culture_trait.id).map_err(fmt_err)?;
        if !culture_trait.note.is_empty() {
            writeln!(out, "    {}", culture_trait.note).map_err(fmt_err)?;
        }

        for elaboration in &culture_trait.elaborates {
            writeln!(
                out,
                "    elaborates  {} into {} word(s):",
                elaboration.concept,
                elaboration.senses.len()
            )
            .map_err(fmt_err)?;
            for sense in &elaboration.senses {
                writeln!(out, "                  · {sense}").map_err(fmt_err)?;
            }
        }

        // The clause the milestone turns on: every gap, with its reason, in print.
        for absence in &culture_trait.lacks {
            let label = format!("{}", absence.concept);
            writeln!(
                out,
                "    lacks       {}{}  — {}",
                label,
                pad(&label, 20),
                absence.reason
            )
            .map_err(fmt_err)?;
        }
        writeln!(out).map_err(fmt_err)?;
    }

    render_arithmetic(&mut out, genome, &meanings)?;
    Ok(out)
}

/// The bottom line: how many meanings this culture coins, and how it got there.
///
/// Stated as arithmetic rather than as a total, because a total alone cannot be
/// checked against anything — and the whole point of the milestone is that the
/// number is *explained*.
fn render_arithmetic(
    out: &mut String,
    genome: &LanguageGenome,
    meanings: &[Meaning<'_>],
) -> Result<()> {
    let (absent, elaborated, extra) = shaping_counts(&genome.environment, meanings);
    let coined = meanings.len() - absent + extra;

    writeln!(out, "  Vocabulary").map_err(fmt_err)?;
    writeln!(
        out,
        "    {} available meaning(s) − {absent} uncoined + {extra} from {elaborated} \
         elaboration(s) = {coined} word(s)",
        meanings.len()
    )
    .map_err(fmt_err)?;

    // Only meaningful once the lexicon exists; saying nothing is better than
    // reporting a mismatch against a language that has not been coined yet.
    if !genome.lexicon.is_empty() {
        let actual = genome.lexicon.len();
        if actual == coined {
            writeln!(out, "    the stored lexicon holds {actual}, as expected").map_err(fmt_err)?;
        } else {
            // Not an error: `derive` (M14) appends, `inflect` replaces, and an
            // author may add words by hand. Reporting the difference beats
            // asserting a number that is only true straight out of `new-lexicon`.
            writeln!(
                out,
                "    the stored lexicon holds {actual}; the difference is words added \
                 after coining (derivation, inflection, or by hand)"
            )
            .map_err(fmt_err)?;
        }
    }
    Ok(())
}

/// Every meaning this culture has no word for, with the trait and reason — the
/// machine-readable half of [`render_culture`], for the UI and for tests.
///
/// In concept-list order, not authored-trait order, so two profiles are comparable
/// line by line. A `Vec`, never a map (§9.4).
pub fn absences(genome: &LanguageGenome) -> Vec<(&str, &str, &str)> {
    let meanings = stem_lexicon::meanings(&genome.concepts);
    let mut out = Vec::new();
    for meaning in &meanings {
        if let Salience::Absent {
            culture_trait,
            reason,
        } = salience(
            &genome.environment,
            &stem_lexicon::ConceptKey::new(meaning.key),
        ) {
            // The key is borrowed from `meanings`, which borrows the compiled table
            // for a built-in and the genome for a project concept — both outlive
            // `genome`'s borrow. Re-resolve against the static table so the lifetime
            // is honest rather than transmuted.
            let key = stem_lexicon::CONCEPTS
                .iter()
                .find(|c| c.key == meaning.key)
                .map(|c| c.key);
            if let Some(key) = key {
                out.push((key, culture_trait.id.as_str(), reason));
            }
        }
    }
    out
}

fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "culture",
        message: "formatting a culture profile into a string failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_lexicon::{Absence, ConceptKey, CultureTrait, Elaboration, EnvironmentProfile};

    fn desert_genome() -> LanguageGenome {
        LanguageGenome {
            environment: EnvironmentProfile {
                summary: "High desert; herders who trade north.".to_owned(),
                traits: vec![CultureTrait {
                    id: "DESERT".to_owned(),
                    name: "High desert".to_owned(),
                    note: "No coast within a season's travel.".to_owned(),
                    elaborates: vec![Elaboration {
                        concept: ConceptKey::new("SAND"),
                        senses: vec!["fine drifting sand".to_owned(), "coarse sand".to_owned()],
                    }],
                    lacks: vec![Absence {
                        concept: ConceptKey::new("SEA"),
                        reason: "no living speaker has seen open water".to_owned(),
                    }],
                }],
            },
            ..LanguageGenome::proto("d", "Desert Asterian")
        }
    }

    #[test]
    fn a_language_with_no_profile_says_so_rather_than_printing_nothing() {
        let text = render_culture(&LanguageGenome::proto("x", "X")).expect("renders");
        assert!(text.contains("No culture profile"), "{text}");
        assert!(
            text.contains("not a deliberate one"),
            "an undeliberate full vocabulary is still a claim: {text}"
        );
    }

    /// **The clause ROADMAP M15 turns on.** A gap you cannot see is indistinguishable
    /// from an accident, so every absence must print with its reason.
    #[test]
    fn every_gap_is_printed_with_the_trait_and_reason_that_explain_it() {
        let text = render_culture(&desert_genome()).expect("renders");
        assert!(text.contains("SEA"), "the gap is named: {text}");
        assert!(
            text.contains("no living speaker has seen open water"),
            "the reason is printed, not just the gap: {text}"
        );
        assert!(text.contains("DESERT"), "the trait is named: {text}");
    }

    #[test]
    fn an_elaboration_prints_every_distinction_the_author_named() {
        let text = render_culture(&desert_genome()).expect("renders");
        assert!(text.contains("SAND into 2 word(s)"), "{text}");
        assert!(text.contains("fine drifting sand"), "{text}");
        assert!(text.contains("coarse sand"), "{text}");
    }

    #[test]
    fn the_vocabulary_line_states_the_arithmetic_rather_than_a_bare_total() {
        let text = render_culture(&desert_genome()).expect("renders");
        assert!(text.contains("− 1 uncoined"), "{text}");
        assert!(text.contains("+ 1 from 1 elaboration(s)"), "{text}");
        assert!(
            text.contains(&format!("= {} word(s)", stem_lexicon::CONCEPT_COUNT)),
            "673 − 1 + 1 = 673: {text}"
        );
    }

    #[test]
    fn rendering_twice_produces_identical_bytes() {
        let genome = desert_genome();
        assert_eq!(
            render_culture(&genome).expect("renders"),
            render_culture(&genome).expect("renders")
        );
    }

    #[test]
    fn absences_lists_each_gap_with_its_trait_and_reason() {
        let genome = desert_genome();
        let gaps = absences(&genome);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].0, "SEA");
        assert_eq!(gaps[0].1, "DESERT");
        assert!(gaps[0].2.contains("open water"));
    }
}
