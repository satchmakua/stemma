//! Syntactic change (ROADMAP M19, `DESIGN.md` §7.4's closing claim).
//!
//! # The one hard requirement
//!
//! ROADMAP M19's acceptance ends: *"the causal chain from the rule to the syntactic
//! shift is on the record — **not asserted by the author**."*
//!
//! That single clause decides the whole design. It would be easy — and worthless — to
//! let an author write "at year 600, word order becomes SVO". The claim this milestone
//! exists to make is that the shift happened *because* a sound change destroyed the
//! case marking that made free order recoverable, and a claim like that is only worth
//! anything if the program can check it.
//!
//! So the split is:
//!
//! - The **author** proposes a consequence: *when the ergative marker is gone, order
//!   fixes to SVO.* That is a claim about this culture's grammar and no engine can
//!   derive it — real languages that lose case go several different ways.
//! - The **engine** establishes the antecedent: it composes the affix onto this
//!   language's own nouns, runs the language's own recorded sound changes over them,
//!   and looks at what is left. If the marker still surfaces anywhere, the change is
//!   **refused** and says so. If it is gone, the change applies and the record names
//!   *which rule erased it* — a `RuleId` the author never wrote down.
//!
//! That is the same relationship a `RuleSet` has to the sound-change engine and a
//! `DriftSet` has to `apply_drift`: authored data, applied or refused by code. It is
//! also, deliberately, the shape M22's LLM assistant will need — a model proposes the
//! same artefact a human writes, and the engine accepts or refuses it by the ordinary
//! path.
//!
//! # Why probing is the right measurement
//!
//! To ask "has the ergative eroded?", this module **composes the affix onto every
//! noun in the lexicon and runs the language's recorded history over the result**.
//! That is not a simulation of something else — it is exactly what a speaker of this
//! stage produces when they inflect a noun, so it is the question itself rather than a
//! proxy for it.
//!
//! Running the affix's citation form *alone* would have been cheaper and wrong: sound
//! changes are conditioned by environment, and an affix in isolation has neither the
//! stem before it nor the word boundary in the right place. `-a` survives after a
//! consonant and vanishes after a vowel under the same rule, and only a real word can
//! show that.
//!
//! **A marker counts as lost only when it surfaces nowhere.** Erosion in some
//! environments and not others is allomorphy, which M8 already models and which is not
//! a syntactic event.
//!
//! # What is deliberately not built
//!
//! ROADMAP M19 names three examples: case erosion forcing stricter word order, topic
//! markers becoming articles, and serial verbs becoming auxiliaries. **Only the first
//! is implemented.** The other two are not a matter of effort: this project has no
//! article, no auxiliary and no serial verb in its model, so "becomes an article"
//! could only be a string. Shipping a `Trigger::TopicMarkerLost` with a
//! `Consequence::BecomesArticle` that changed nothing observable would be scaffolding
//! pretending to be a feature — the thing `docs/adr/0008` refused for
//! `LineageEdgeKind` and M4 refused for `WordEntry.ancestor`. They arrive when there
//! is a category for them to become.

use serde::{Deserialize, Serialize};
use stem_core::{Issue, LanguageId, Result, RuleId, Severity, StemmaError, ValidationReport};
use stem_lexicon::{Lexicon, Morpheme, MorphemeRole, PartOfSpeech, WordEntry, compose};
use stem_syntax::{AdpositionOrder, Alignment, SyntaxProfile, WordOrder};

use crate::LanguageGenome;

// ------------------------------------------------------------------ the authored

/// A condition the **engine** verifies against the language's own history.
///
/// Never a date and never a bare assertion: a trigger the author could simply declare
/// true would make the causal chain a claim rather than a finding, which is the one
/// thing M19's acceptance forbids.
///
/// `#[non_exhaustive]`, because the two examples this milestone leaves unbuilt are
/// exactly the shape of a future variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Trigger {
    /// This morpheme no longer surfaces on **any** word of the language.
    ///
    /// Checked by composing it onto every noun and running the recorded sound
    /// changes; see the module docs on why that is the measurement and not a proxy
    /// for one.
    CaseMarkerLost {
        /// The morpheme, by id.
        morpheme: stem_core::MorphemeId,
    },
}

/// What an author claims follows, once the trigger holds.
///
/// The consequence is **authored**, and that is not a weakness in the design. A
/// language that loses its case marking may fix its word order, or lean on
/// adpositions, or develop agreement; which one it does is a fact about a people, and
/// compiling a choice in would be Stemma asserting the history of a language it did
/// not invent — M15's "no ecology→vocabulary inference table", one layer up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Consequence {
    /// Constituent order becomes this.
    WordOrder(WordOrder),
    /// Case alignment becomes this.
    Alignment(Alignment),
    /// Adposition placement becomes this.
    Adpositions(AdpositionOrder),
}

impl Consequence {
    /// What this change did, in words, for the record.
    ///
    /// In this crate rather than at a call site: the enums are `#[non_exhaustive]`,
    /// so a downstream `match` would need a wildcard arm and a new variant would stop
    /// being a compile error where it has to be handled (`Formation::summary`'s
    /// precedent, the fourth time).
    pub fn effect(&self) -> String {
        match self {
            Self::WordOrder(order) => format!("word order became {}", order.name()),
            Self::Alignment(alignment) => {
                format!("alignment became {}", alignment.row().value)
            }
            Self::Adpositions(adpositions) => {
                format!("adpositions became {}", adpositions.row().value)
            }
        }
    }

    /// Applies it to a profile.
    fn apply_to(&self, profile: &mut SyntaxProfile) {
        match self {
            Self::WordOrder(order) => profile.word_order = *order,
            Self::Alignment(alignment) => profile.alignment = *alignment,
            Self::Adpositions(adpositions) => profile.adpositions = *adpositions,
        }
    }
}

/// One proposed syntactic change: a verifiable condition and an authored consequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntacticChange {
    /// Stable id, e.g. `"sx_0001"`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Authorial prose: why this consequence follows. **Not interpreted**, and the
    /// place to argue for a claim the engine cannot check.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// The condition the engine verifies.
    pub when: Trigger,
    /// What the author says follows.
    pub then: Consequence,
    /// Simulated years, for the §10.4 timeline. Descriptive; nothing gates on it,
    /// because a date is exactly the kind of trigger this design refuses.
    #[serde(default)]
    pub chronology_years: i32,
}

/// An ordered set of proposed syntactic changes — the `RuleSet`/`DriftSet` shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntacticChangeSet {
    /// Stable identity for the file.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional prose.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// **Ordered**, and never re-sorted: a later change may depend on an earlier
    /// one having fired, exactly as a later sound change may feed on an earlier one.
    pub changes: Vec<SyntacticChange>,
}

// -------------------------------------------------------------------- the record

/// One syntactic change that actually happened, with what caused it.
///
/// The §3.3 record for grammar. `caused_by` is the field the milestone is about: it
/// is a [`RuleId`] the **engine** found by replaying the language's own derivations,
/// never something the author wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedShift {
    /// Which change fired.
    pub change: String,
    /// Its display name.
    pub name: String,
    /// What the engine verified, in words.
    pub trigger: String,
    /// **The sound change that destroyed the marker** — established by replaying this
    /// language's own recorded derivations, not asserted.
    ///
    /// `None` when the marker was already absent before any rule ran, which is a
    /// different fact and must not be dressed up as a causal chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<RuleId>,
    /// What changed in the profile.
    pub effect: String,
    /// Simulated years, copied from the change.
    #[serde(default)]
    pub chronology_years: i32,
}

// ------------------------------------------------------------------- the finding

/// What the engine found when it checked one trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Whether the condition holds.
    pub holds: bool,
    /// What was measured, in words — printed whether it held or not, because "why
    /// did this *not* apply?" is the diagnostic M10 established and the one an author
    /// actually needs.
    pub evidence: String,
    /// The rule that erased the marker, when one did.
    pub caused_by: Option<RuleId>,
}

/// Checks one trigger against `genome`'s own history.
///
/// Pure and RNG-free. It re-derives a measurement and stores nothing.
pub fn check(genome: &LanguageGenome, trigger: &Trigger) -> Result<Finding> {
    match trigger {
        Trigger::CaseMarkerLost { morpheme } => {
            let affix = genome
                .morphology
                .morphemes
                .iter()
                .find(|m| &m.id == morpheme)
                .filter(|m| m.role != MorphemeRole::Stem)
                .ok_or_else(|| StemmaError::not_found("affix morpheme", morpheme))?;
            case_marker_lost(genome, affix)
        }
    }
}

/// The measurement: inflect every noun with `affix`, run this language's recorded
/// history over the results, and see whether anything of the affix is left.
fn case_marker_lost(genome: &LanguageGenome, affix: &Morpheme) -> Result<Finding> {
    // The probe words: every noun in the lexicon, in stored order, each with the
    // affix attached exactly as `stemma say` would attach it. Lexicon order makes
    // the result deterministic; using *every* noun makes "lost" mean lost, since a
    // marker surviving in one environment has not been lost at all.
    let mut probes: Vec<WordEntry> = Vec::new();
    let mut spans: Vec<(u32, u32)> = Vec::new();
    for (i, entry) in genome
        .lexicon
        .iter()
        .filter(|e| e.part_of_speech == PartOfSpeech::Noun)
        .enumerate()
    {
        // **The proto-form, not the current one.** `Derivation::input` is "the form
        // entering the first rule this word ever met", so composing from it and then
        // running the whole recorded history reproduces what a speaker of this stage
        // actually says. Composing from `phonemic_form` — which has *already* been
        // through those rules — would apply the history twice and report erosion that
        // never happened.
        let proto = entry
            .trace
            .as_ref()
            .map(|t| t.input.clone())
            .unwrap_or_else(|| entry.phonemic_form.clone());
        let stem = Morpheme {
            id: stem_core::MorphemeId::new(entry.id.as_str()),
            role: MorphemeRole::Stem,
            gloss: entry.display_gloss().unwrap_or("?").to_owned(),
            form: proto,
            part_of_speech: PartOfSpeech::Noun,
        };
        let (form, refs) = compose(&stem, &[affix]);
        let span = refs
            .iter()
            .find(|r| r.morpheme == affix.id)
            .map(|r| (r.start, r.end))
            .expect("compose emits a ref for every part");
        spans.push(span);

        let mut probe = entry.clone();
        probe.id = stem_core::WordId::sequential(i + 1);
        probe.phonemic_form = form;
        probe.trace = None;
        probe.morphemes = refs;
        probes.push(probe);
    }

    if probes.is_empty() {
        return Ok(Finding {
            holds: false,
            evidence: format!(
                "this language has no nouns to inflect, so whether `{}` survives cannot \
                 be measured",
                affix.id
            ),
            caused_by: None,
        });
    }

    // The language's own recorded history, re-run over the probes. `applied_rules`
    // is past tense (the genome's own contract), so this reproduces what a speaker of
    // *this* stage says when they inflect a noun.
    let evolved = stem_soundchange::apply_rules(
        &genome.applied_rules,
        0,
        &genome.phonemes,
        &genome.prosody,
        &Lexicon::from_entries(probes.clone()),
    )?;

    // Where does the affix still surface? First survivor wins the report, so the
    // evidence names a concrete word rather than a count.
    let mut survivor: Option<(String, String)> = None;
    let mut earliest: Option<(usize, RuleId)> = None;

    for ((probe, (start, end)), evolved_entry) in
        probes.iter().zip(&spans).zip(evolved.lexicon.iter())
    {
        let surface = evolved_entry.morpheme_surface(*start as usize, *end as usize);
        if !surface.is_empty() {
            if survivor.is_none() {
                let written = |ids: &[stem_core::PhonemeId]| -> String {
                    ids.iter()
                        .filter_map(|id| evolved.inventory.get(id))
                        .map(|p| p.written())
                        .collect()
                };
                survivor = Some((
                    probe.display_gloss().unwrap_or("?").to_owned(),
                    written(&surface),
                ));
            }
            continue;
        }
        // Gone here. Which step emptied it? Replay this word's own derivation one
        // step at a time — `render_paradigm`'s technique, and the only honest way to
        // name a cause: the rule is read off the record, not off the change file.
        if let Some(trace) = &evolved_entry.trace {
            for k in 0..trace.steps.len() {
                let partial = stem_lexicon::Derivation {
                    input: trace.input.clone(),
                    steps: trace.steps[..=k].to_vec(),
                };
                if partial
                    .surface_of_input_span(*start as usize, *end as usize)
                    .is_empty()
                {
                    let rule = trace.steps[k].rule.clone();
                    // The EARLIEST step to erase it anywhere is the cause: a later
                    // rule finishing the job off in one more word is not what killed
                    // the marker.
                    if earliest.as_ref().is_none_or(|(seen, _)| k < *seen) {
                        earliest = Some((k, rule));
                    }
                    break;
                }
            }
        }
    }

    match survivor {
        Some((gloss, form)) => Ok(Finding {
            holds: false,
            evidence: format!(
                "`{}` still surfaces as `-{form}` on at least one word (\"{gloss}\"), so it \
                 has not been lost — a marker that erodes in some environments and not \
                 others is allomorphy, not a syntactic event",
                affix.id
            ),
            caused_by: None,
        }),
        None => {
            let caused_by = earliest.map(|(_, rule)| rule);
            let evidence = match &caused_by {
                Some(rule) => format!(
                    "`{}` surfaces on no word of this language; `{rule}` is the recorded \
                     sound change that erased it",
                    affix.id
                ),
                None => format!(
                    "`{}` surfaces on no word of this language, and no recorded sound \
                     change removed it — it was already absent",
                    affix.id
                ),
            };
            Ok(Finding {
                holds: true,
                evidence,
                caused_by,
            })
        }
    }
}

// -------------------------------------------------------------------- the verb

/// Applies `changes` to `genome`, producing the next stage of the lineage.
///
/// The syntactic twin of `evolve` (forms) and `with_drift` (meanings). Each change is
/// checked against the language's own history; one whose trigger does not hold is
/// **refused and reported**, never partially applied.
///
/// Returns the daughter and a report. The report carries one Note per change, applied
/// or not, because "why did this not fire?" is the question an author has when nothing
/// happened — M10's diagnostic discipline, for grammar.
pub fn apply_shifts(
    genome: &LanguageGenome,
    changes: &SyntacticChangeSet,
    id: impl Into<String>,
    name: impl Into<String>,
    years: i32,
) -> Result<(LanguageGenome, ValidationReport)> {
    let mut daughter = genome.clone();
    daughter.id = LanguageId::new(id);
    daughter.name = name.into();
    daughter.parent = Some(genome.id.clone());
    daughter.lineage_depth_years = genome.lineage_depth_years + years;

    let mut report = ValidationReport::new();

    for change in &changes.changes {
        // Checked against the DAUGHTER, so a change may depend on one that fired
        // before it — the same feeding relationship an ordered rule set has.
        let finding = check(&daughter, &change.when)?;
        if !finding.holds {
            report.note(
                "shift_did_not_apply",
                format!("`{}` did not apply: {}", change.id, finding.evidence),
            );
            continue;
        }

        change.then.apply_to(&mut daughter.syntax);
        daughter.applied_shifts.push(AppliedShift {
            change: change.id.clone(),
            name: change.name.clone(),
            trigger: finding.evidence.clone(),
            caused_by: finding.caused_by.clone(),
            effect: change.then.effect(),
            chronology_years: change.chronology_years,
        });
        report.note(
            "shift_applied",
            format!("`{}` applied: {}", change.id, change.then.effect()),
        );
    }

    if daughter.applied_shifts.len() == genome.applied_shifts.len() {
        report.push(Issue::new(
            Severity::Warning,
            "no_shift_applied",
            "no proposed change met its condition, so this daughter differs from its \
             parent only in identity; the notes above say why each was refused",
        ));
    }

    Ok((daughter, report))
}

/// Renders a language's syntactic history — the §3.3 record, read back.
///
/// A pure renderer, the `render_grammar` precedent. Never empty: a language whose
/// grammar has not changed is told so, because "nothing happened" and "nothing was
/// recorded" are different facts.
pub fn render_shifts(genome: &LanguageGenome) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "Syntactic history — {}", genome.name);
    let _ = writeln!(out);

    if genome.applied_shifts.is_empty() {
        let _ = writeln!(
            out,
            "  No recorded syntactic change. This language's grammar is as it was"
        );
        let _ = writeln!(out, "  written; nothing has shifted it.");
        return out;
    }

    for (i, shift) in genome.applied_shifts.iter().enumerate() {
        let _ = writeln!(out, "  {i}  {}  ({})", shift.name, shift.change);
        let _ = writeln!(out, "     {}", shift.effect);
        let _ = writeln!(out, "     because {}", shift.trigger);
        match &shift.caused_by {
            Some(rule) => {
                let _ = writeln!(
                    out,
                    "     the cause is on the record: sound change `{rule}`"
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "     no sound change is recorded as the cause; the marker was \
                     already absent"
                );
            }
        }
        let _ = writeln!(out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_core::MorphemeId;

    fn change_set(consequence: Consequence) -> SyntacticChangeSet {
        SyntacticChangeSet {
            id: "sx".to_owned(),
            name: "Test changes".to_owned(),
            description: String::new(),
            changes: vec![SyntacticChange {
                id: "sx_0001".to_owned(),
                name: "Order fixes as case erodes".to_owned(),
                note: String::new(),
                when: Trigger::CaseMarkerLost {
                    morpheme: MorphemeId::new("m_erg"),
                },
                then: consequence,
                chronology_years: 600,
            }],
        }
    }

    #[test]
    fn a_change_set_round_trips_through_ron() {
        let set = change_set(Consequence::WordOrder(WordOrder::Svo));
        let text = ron::ser::to_string(&set).expect("serialise");
        let back: SyntacticChangeSet = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, set);
    }

    #[test]
    fn a_misspelled_change_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(id: "x", name: "X", chnages: [])"#;
        assert!(ron::from_str::<SyntacticChangeSet>(text).is_err());
    }

    /// A trigger naming a morpheme this language does not declare is an error, not a
    /// silent false: the author has written something that can never fire, and saying
    /// "did not apply" would hide the typo.
    #[test]
    fn a_trigger_naming_an_undeclared_morpheme_is_an_error() {
        let genome = LanguageGenome::proto("x", "X");
        let err = check(
            &genome,
            &Trigger::CaseMarkerLost {
                morpheme: MorphemeId::new("m_nowhere"),
            },
        )
        .expect_err("no such morpheme");
        assert!(err.to_string().contains("m_nowhere"), "{err}");
    }

    #[test]
    fn a_consequence_describes_itself_for_the_record() {
        assert_eq!(
            Consequence::WordOrder(WordOrder::Svo).effect(),
            "word order became SVO"
        );
        assert_eq!(
            Consequence::Alignment(Alignment::Neutral).effect(),
            "alignment became neutral"
        );
    }

    #[test]
    fn a_language_with_no_recorded_shift_says_so() {
        let text = render_shifts(&LanguageGenome::proto("x", "X"));
        assert!(text.contains("No recorded syntactic change"), "{text}");
    }
}
