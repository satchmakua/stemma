//! Rendering one word's **meaning** history (M9, `DESIGN.md` §10.2).
//!
//! §10.2's worked trace has two halves. The sound-change half —
//! `*takala → tagala → tagal → taɣal` — is `stem_soundchange::render_derivation`
//! and has rendered since M3. The other half is the line that trace ends on:
//!
//! ```text
//! Semantic shift: "star" → "omen" in priestly register
//! ```
//!
//! This module renders that, and [`render_word_history`] composes the two into the
//! complete §10.2 story.
//!
//! # Composition, not modification
//!
//! `render_derivation` is **not touched**. Its bytes are frozen inside the §21 demo
//! canary, and `stem_soundchange` must never name a semantic type (a source-scan
//! guard enforces it). So this lives here, in `stem_genome`, for the
//! [`crate::render_paradigm`] precedent: a renderer that needs *two halves of the
//! genome* — the lexicon entry and the semantic space that names its senses —
//! belongs in the library above both, where the M11 UI can render identical bytes.
//!
//! # An undrifted word renders nothing
//!
//! [`render_sense_history`] returns `String::new()` when a word has no recorded
//! history, which is what keeps every pre-M9 `stemma trace` output byte-identical
//! and M5's `trace-word star == trace w_0001` equality true.

use std::fmt::Write;

use stem_core::{Result, SemanticNodeId, StemmaError};
use stem_lexicon::WordEntry;

use crate::LanguageGenome;

/// The complete §10.2 story for one word: its sound-change derivation, then its
/// meaning history.
///
/// A pure function; no clock, no map, no float. Newline-terminated.
pub fn render_word_history(genome: &LanguageGenome, entry: &WordEntry) -> Result<String> {
    let mut out =
        stem_soundchange::render_derivation(entry, &genome.applied_rules, &genome.phonemes)?;
    out.push_str(&render_sense_history(genome, entry));
    Ok(out)
}

/// The meaning half alone — **empty when the word has no recorded drift**.
///
/// Resolves each sense id to its gloss through the genome's own space, and reads
/// each step's mechanism, register, date and prose from `applied_drifts` rather than
/// from the stored step, so a renamed event cannot leave a stale label behind.
pub fn render_sense_history(genome: &LanguageGenome, entry: &WordEntry) -> String {
    let Some(history) = &entry.sense_history else {
        return String::new();
    };
    if history.steps.is_empty() {
        return String::new();
    }

    // A sense id rendered as its gloss, falling back to the bare id when the space
    // does not name it — a truthful last resort, never a fabricated gloss.
    let gloss_of = |id: &SemanticNodeId| -> String {
        genome
            .semantics
            .node(id)
            .map(|n| n.gloss.clone())
            .unwrap_or_else(|| id.as_str().to_owned())
    };
    let glosses = |ids: &[SemanticNodeId]| -> String {
        if ids.is_empty() {
            return "∅".to_owned();
        }
        ids.iter().map(gloss_of).collect::<Vec<_>>().join(", ")
    };
    let ids = |ids: &[SemanticNodeId]| -> String {
        ids.iter()
            .map(|i| i.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    // `write!` into a String is infallible; the derivation renderer takes the same
    // view. Any error here would be an allocation failure, not a domain fault.
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  sense      {:<20}  {}",
        glosses(&history.input),
        ids(&history.input)
    );

    for (i, step) in history.steps.iter().enumerate() {
        let event = genome
            .applied_drifts
            .iter()
            .find(|e| e.id == step.event)
            .filter(|_| (step.index as usize) < genome.applied_drifts.len());

        let _ = writeln!(out, "  │");
        match event {
            Some(event) => {
                let register = match &event.register {
                    Some(register) => format!(" · {register}"),
                    None => String::new(),
                };
                let _ = writeln!(
                    out,
                    "  {i}  {}  {}{register} · {}y",
                    step.event,
                    event.mechanism.name(),
                    event.chronology_years
                );
                let _ = writeln!(
                    out,
                    "  │    {} > {}",
                    glosses(&step.removed),
                    glosses(&step.added)
                );
                if !event.description.is_empty() {
                    let _ = writeln!(out, "  │    \"{}\"", event.description);
                }
            }
            // The step names an event the genome's log does not hold. Say so
            // rather than printing a confident half-line — the same honesty
            // `render_derivation` shows for a rule that did not apply.
            None => {
                let _ = writeln!(
                    out,
                    "  {i}  {}  (event not in this language's log)",
                    step.event
                );
                let _ = writeln!(
                    out,
                    "  │    {} > {}",
                    glosses(&step.removed),
                    glosses(&step.added)
                );
            }
        }
    }

    let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();
    let _ = writeln!(out, "  │");
    let _ = writeln!(out, "  means      {:<20}  {}", glosses(&held), ids(&held));
    out
}

/// A `std::fmt::Error` means an allocation failure, not a domain error.
#[allow(dead_code)]
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "sense history",
        message: "formatting a sense history into a string failed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_core::{EventId, LanguageId, PhonemeId, WordId};
    use stem_lexicon::{
        DriftEvent, DriftMechanism, Lexicon, PartOfSpeech, SemanticNode, SemanticSpace,
        SenseHistory, SenseRef, SenseShift, WordSource, scoped_cognate_set,
    };
    use stem_phonology::{Root, Syllable};

    fn node(id: &str, gloss: &str) -> SemanticNode {
        SemanticNode {
            id: SemanticNodeId::new(id),
            gloss: gloss.to_owned(),
            concept: None,
            note: String::new(),
        }
    }

    /// A word that drifted star → divine sign → omen, with the events on the log.
    fn drifted() -> (LanguageGenome, WordEntry) {
        let entry = WordEntry {
            id: WordId::new("w_0001"),
            concept: None,
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![PhonemeId::new("ph_t"), PhonemeId::new("ph_a")],
                    stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: scoped_cognate_set(&LanguageId::new("x"), 1),
            source: WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
            senses: vec![SenseRef {
                node: SemanticNodeId::new("sn_omen"),
                gloss: "omen".to_owned(),
            }],
            sense_history: Some(SenseHistory {
                input: vec![SemanticNodeId::new("sn_star")],
                steps: vec![
                    SenseShift {
                        event: EventId::new("ev_0001"),
                        index: 0,
                        removed: vec![SemanticNodeId::new("sn_star")],
                        added: vec![SemanticNodeId::new("sn_divine_sign")],
                    },
                    SenseShift {
                        event: EventId::new("ev_0002"),
                        index: 1,
                        removed: vec![SemanticNodeId::new("sn_divine_sign")],
                        added: vec![SemanticNodeId::new("sn_omen")],
                    },
                ],
            }),
        };

        let event = |id: &str, mechanism, years, description: &str| DriftEvent {
            id: EventId::new(id),
            name: "shift".to_owned(),
            description: description.to_owned(),
            word: WordId::new("w_0001"),
            mechanism,
            chronology_years: years,
            register: Some("priestly".to_owned()),
            remove: Vec::new(),
            add: Vec::new(),
        };

        let mut genome = LanguageGenome::proto("x", "X")
            .with_semantics(SemanticSpace {
                nodes: vec![
                    node("sn_star", "star"),
                    node("sn_divine_sign", "divine sign"),
                    node("sn_omen", "omen"),
                ],
            })
            .with_lexicon(Lexicon::from_entries([entry.clone()]));
        genome.applied_drifts = vec![
            event(
                "ev_0001",
                DriftMechanism::Metaphor,
                180,
                "the object read as intent",
            ),
            event("ev_0002", DriftMechanism::Metonymy, 340, ""),
        ];
        (genome, entry)
    }

    #[test]
    fn the_semantic_block_names_each_step_its_mechanism_and_its_register() {
        let (genome, entry) = drifted();
        let text = render_sense_history(&genome, &entry);
        assert!(text.contains("sense") && text.contains("star"), "{text}");
        assert!(text.contains("metaphor · priestly · 180y"), "{text}");
        assert!(text.contains("metonymy · priestly · 340y"), "{text}");
        assert!(text.contains("star > divine sign"), "{text}");
        assert!(text.contains("divine sign > omen"), "{text}");
        assert!(
            text.contains("the object read as intent"),
            "the authorial reason is rendered: {text}"
        );
        assert!(text.contains("means") && text.contains("omen"), "{text}");
    }

    /// The byte-identity guarantee: a word that never drifted renders **nothing**,
    /// so every pre-M9 `stemma trace` output is unchanged.
    #[test]
    fn a_word_with_no_drift_renders_an_empty_semantic_block() {
        let (genome, mut entry) = drifted();
        entry.sense_history = None;
        assert_eq!(render_sense_history(&genome, &entry), "");
    }

    #[test]
    fn the_renderer_is_a_pure_function() {
        let (genome, entry) = drifted();
        assert_eq!(
            render_sense_history(&genome, &entry),
            render_sense_history(&genome, &entry)
        );
    }

    /// An id the space does not name falls back to the bare id — truthful, never a
    /// fabricated gloss.
    #[test]
    fn an_unknown_sense_renders_as_its_id_rather_than_an_invented_gloss() {
        let (mut genome, entry) = drifted();
        genome.semantics = SemanticSpace::new();
        let text = render_sense_history(&genome, &entry);
        assert!(text.contains("sn_star"), "{text}");
        assert!(!text.contains("\"star\""), "no invented gloss: {text}");
    }
}
