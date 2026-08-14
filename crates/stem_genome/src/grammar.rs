//! Rendering a language's grammar sketch (M17, `DESIGN.md` §7.4).
//!
//! A **pure library renderer**, the `render_culture` / `render_paradigm` precedent
//! (`docs/adr/0006`): built in memory, newline-terminated, no map, no float, no
//! clock, so two calls are byte-identical (§9.4) and the M11 UI renders identical
//! text. It reads the stored profile and derives nothing but headedness — which is
//! derived precisely so it cannot disagree with the parameters it summarises.
//!
//! # It describes; it does not generate
//!
//! Nothing here builds a sentence. M17 is parameters, their validation, and this
//! sketch; M18 is the first sentence. Splitting them is §20.1's scope discipline
//! applied to the largest remaining gap in the project, and it means the grammar has
//! to be legible before anything is built on top of it.
//!
//! # Unstated is printed, not skipped
//!
//! A parameter nobody has decided prints as `—  (not stated)` rather than vanishing.
//! A sketch that silently omitted its gaps would read as a complete grammar of a
//! language that is mostly undecided, which is the same invisibility M15 built a
//! whole milestone to remove from the vocabulary.

use std::fmt::Write;

use stem_core::{Result, StemmaError, Validate};

use crate::LanguageGenome;
use crate::pad;

/// Renders `genome`'s syntax profile as a readable sketch.
///
/// Never empty: a language with no profile gets the statement that it has none and
/// what to do about it, for the reason `render_culture` does the same.
pub fn render_grammar(genome: &LanguageGenome) -> Result<String> {
    let profile = &genome.syntax;
    let mut out = String::new();

    writeln!(out, "Grammar — {}", genome.name).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    if profile.is_empty() {
        writeln!(
            out,
            "  No syntax profile. This language has a phonology, a lexicon"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  and a history, but nothing yet says how it builds a clause."
        )
        .map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
        writeln!(
            out,
            "  Declare `syntax:` in the genome to state its word order,"
        )
        .map_err(fmt_err)?;
        writeln!(out, "  adpositions, alignment and the rest.").map_err(fmt_err)?;
        return Ok(out);
    }

    if !profile.note.is_empty() {
        writeln!(out, "  {}", profile.note).map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
    }

    // Every parameter, its value and a plain-English gloss — all supplied by
    // `stem_syntax`, which is where the enums are defined and therefore the only
    // place a `match` over them is exhaustiveness-checked.
    let rows = profile.rows();

    // Char count, not byte length: the values and glosses carry en-dashes and
    // italics markers, and `{:<n}` counts bytes.
    let label_width = rows
        .iter()
        .map(|r| r.label.chars().count())
        .max()
        .unwrap_or(0);
    let value_width = rows
        .iter()
        .map(|r| r.value.chars().count())
        .max()
        .unwrap_or(0);

    for row in &rows {
        write!(out, "  {}{}  ", row.label, pad(row.label, label_width)).map_err(fmt_err)?;
        // The value column is padded only when something follows it. Padding the last
        // cell on a line leaves trailing whitespace, which every diff tool and half
        // the world's editors flag — and it would sit in the middle of a file the
        // §21 demo is expected to keep byte-stable.
        if row.gloss.is_empty() {
            writeln!(out, "{}", row.value).map_err(fmt_err)?;
        } else {
            writeln!(
                out,
                "{}{}  {}",
                row.value,
                pad(row.value, value_width),
                row.gloss
            )
            .map_err(fmt_err)?;
        }
    }

    // The harmony report, below the parameters and clearly labelled as description.
    // It is the profile's own `Validate` output — one source, so the sketch and
    // `stemma validate` cannot disagree about what is unusual here.
    let report = profile.validate();
    let remarks: Vec<&stem_core::Issue> = report
        .issues
        .iter()
        .filter(|i| i.code != "unspecified")
        .collect();
    writeln!(out).map_err(fmt_err)?;
    if remarks.is_empty() {
        writeln!(
            out,
            "  Typologically harmonic: every stated order agrees with the others."
        )
        .map_err(fmt_err)?;
    } else {
        writeln!(out, "  Notes on harmony:").map_err(fmt_err)?;
        for issue in remarks {
            writeln!(out, "    · {}", issue.message).map_err(fmt_err)?;
        }
        writeln!(out).map_err(fmt_err)?;
        writeln!(
            out,
            "  These describe what is common, not what is correct. A rare language is"
        )
        .map_err(fmt_err)?;
        writeln!(
            out,
            "  a design; Stemma reports it and does not refuse it (§17)."
        )
        .map_err(fmt_err)?;
    }

    Ok(out)
}

/// `render_paradigm`'s policy verbatim: a `std::fmt::Error` in string building means
/// an allocation failure, not a domain error.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "grammar",
        message: "formatting a grammar sketch into a string failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_syntax::{
        AdjectiveOrder, AdpositionOrder, Alignment, Evidentiality, GenitiveOrder, Negation,
        ProDrop, QuestionFormation, RelativeClause, SwitchReference, SyntaxProfile, WordOrder,
    };

    fn head_final() -> LanguageGenome {
        LanguageGenome {
            syntax: SyntaxProfile {
                note: "A rigidly verb-final grammar with a rich case system.".to_owned(),
                word_order: WordOrder::Sov,
                adpositions: AdpositionOrder::Postpositions,
                genitive: GenitiveOrder::GenitiveNoun,
                adjective: AdjectiveOrder::AdjectiveNoun,
                alignment: Alignment::ErgativeAbsolutive,
                relative_clause: RelativeClause::Prenominal,
                negation: Negation::Affix,
                question: QuestionFormation::Particle,
                pro_drop: ProDrop::Yes,
                evidentiality: Evidentiality::TwoWay,
                switch_reference: SwitchReference::Marked,
            },
            ..LanguageGenome::proto("x", "Test Asterian")
        }
    }

    #[test]
    fn a_language_with_no_profile_says_so_rather_than_printing_an_empty_table() {
        let text = render_grammar(&LanguageGenome::proto("x", "X")).expect("renders");
        assert!(text.contains("No syntax profile"), "{text}");
        assert!(text.contains("how it builds a clause"), "{text}");
    }

    #[test]
    fn the_sketch_prints_every_parameter_with_a_plain_english_gloss() {
        let text = render_grammar(&head_final()).expect("renders");
        for label in [
            "Word order",
            "Headedness",
            "Adpositions",
            "Genitive",
            "Adjective",
            "Alignment",
            "Relative clause",
            "Negation",
            "Questions",
            "Pro-drop",
            "Evidentiality",
            "Switch-reference",
        ] {
            assert!(text.contains(label), "`{label}` is missing:\n{text}");
        }
        assert!(text.contains("SOV"), "{text}");
        assert!(text.contains("head-final"), "{text}");
        assert!(
            text.contains("*the king's road*"),
            "the gloss makes the parameter legible: {text}"
        );
    }

    /// The derived summary is printed **and** labelled as derived, so a reader is not
    /// left wondering whether it is a thirteenth parameter they could edit.
    #[test]
    fn headedness_is_shown_as_derived() {
        let text = render_grammar(&head_final()).expect("renders");
        assert!(
            text.contains("derived from the orders below, never stored"),
            "{text}"
        );
    }

    #[test]
    fn a_harmonic_language_is_told_that_it_is() {
        let text = render_grammar(&head_final()).expect("renders");
        assert!(text.contains("Typologically harmonic"), "{text}");
    }

    /// **ROADMAP M17's acceptance, rendered.** The odd combination is reported, with
    /// the sketch stating in as many words that rare is not wrong.
    #[test]
    fn a_disharmonic_language_prints_the_tendency_and_says_rare_is_not_wrong() {
        let mut genome = head_final();
        genome.syntax.adpositions = AdpositionOrder::Prepositions;

        let text = render_grammar(&genome).expect("renders");
        assert!(text.contains("Notes on harmony"), "{text}");
        assert!(text.contains("usually goes with postpositions"), "{text}");
        assert!(
            text.contains("does not refuse it"),
            "the sketch must say the tool is describing, not policing: {text}"
        );
        // And nothing about the grammar is an error. Asserted on the *profile's* own
        // report rather than the whole genome's: this test's genome has no phonemes,
        // which is an error of its own and has nothing to do with its syntax.
        assert!(
            genome.syntax.validate().errors().next().is_none(),
            "no combination of syntactic parameters may be an error"
        );
    }

    #[test]
    fn an_unstated_parameter_prints_a_dash_rather_than_vanishing() {
        let mut genome = head_final();
        genome.syntax.evidentiality = Evidentiality::Unspecified;
        let text = render_grammar(&genome).expect("renders");
        assert!(
            text.lines()
                .any(|l| l.contains("Evidentiality") && l.contains('—')),
            "an undecided parameter must still have a row: {text}"
        );
    }

    #[test]
    fn rendering_twice_produces_identical_bytes() {
        let genome = head_final();
        assert_eq!(
            render_grammar(&genome).expect("renders"),
            render_grammar(&genome).expect("renders")
        );
    }
}
