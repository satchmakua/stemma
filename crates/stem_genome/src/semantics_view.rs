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

use stem_core::{Result, SemanticNodeId};
use stem_lexicon::WordEntry;

use crate::LanguageGenome;

/// The complete §10.2 story for one word: its sound-change derivation, then what it
/// is made of (M14), then its meaning history.
///
/// A pure function; no clock, no map, no float. Newline-terminated.
///
/// The etymology sits between form and meaning because that is what it joins: it
/// explains the *shape* by naming the words that went in, and their glosses are what
/// the meaning history then acts on. Both halves render `String::new()` for a word
/// that has neither, so every pre-M9 and pre-M14 `stemma trace` output is unmoved to
/// the byte — the composition-not-modification rule this module was created under.
pub fn render_word_history(genome: &LanguageGenome, entry: &WordEntry) -> Result<String> {
    let mut out =
        stem_soundchange::render_derivation(entry, &genome.applied_rules, &genome.phonemes)?;
    out.push_str(&crate::etymology::render_etymology(genome, entry)?);
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
    let from = glosses(&history.input);
    let _ = writeln!(
        out,
        "  sense      {from}{}  {}",
        crate::pad(&from, 20),
        ids(&history.input)
    );

    for (i, step) in history.steps.iter().enumerate() {
        // Resolved by **index**, exactly as `render_derivation` resolves a rule
        // step against `applied_rules`. `applied_drifts` is a log, not a set — ids
        // may repeat across strata — so an id-first lookup would render the *first*
        // event's mechanism and date for a later step that happens to share its id.
        // The id is then checked as a consistency guard: a mismatch means the log
        // and the stored history disagree, and the `None` arm says so rather than
        // printing a confident wrong line.
        let event = genome
            .applied_drifts
            .get(step.index as usize)
            .filter(|e| e.id == step.event);

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
    let now = glosses(&held);
    let _ = writeln!(
        out,
        "  means      {now}{}  {}",
        crate::pad(&now, 20),
        ids(&held)
    );
    out
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
            bases: Vec::new(),
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

    /// **A log is not a set.** `applied_drifts` may hold the same event id twice —
    /// two strata applying one drift file is exactly the case its "ids may repeat"
    /// contract permits. A step must therefore resolve by its stored **index**, or a
    /// later step renders the *earlier* event's mechanism, register and date.
    ///
    /// Regression test: an id-first lookup shows `metaphor · 180y` for both steps.
    #[test]
    fn a_repeated_event_id_across_strata_renders_each_step_from_its_own_index() {
        let (mut genome, mut entry) = drifted();

        // A second stratum reusing `ev_0001` for a genuinely different shift.
        genome.applied_drifts.push(DriftEvent {
            id: EventId::new("ev_0001"),
            name: "a later, unrelated shift".to_owned(),
            description: String::new(),
            word: WordId::new("w_0001"),
            mechanism: DriftMechanism::Pejoration,
            chronology_years: 900,
            register: None,
            remove: Vec::new(),
            add: Vec::new(),
        });
        // The word records it at index 2 — the position, not the name, is what
        // identifies which event ran.
        if let Some(history) = &mut entry.sense_history {
            history.steps.push(SenseShift {
                event: EventId::new("ev_0001"),
                index: 2,
                removed: vec![SemanticNodeId::new("sn_omen")],
                added: Vec::new(),
            });
        }

        let text = render_sense_history(&genome, &entry);
        assert!(
            text.contains("pejoration") && text.contains("900y"),
            "step 2 must render from index 2, not from the first `ev_0001`:\n{text}"
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
