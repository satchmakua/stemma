//! The constrained copilot (ROADMAP M22, `DESIGN.md` §6.5).
//!
//! # The constraint, and why it decides the architecture
//!
//! *No LLM output may mutate a language without passing through validation* (§3.2).
//! That has been enforced since M0, when there was nothing to enforce it against,
//! because it decides **where logic may live** — and now that there is something, it
//! decides the shape of this crate completely.
//!
//! The strongest possible enforcement of "the model is not the source of truth" is
//! that **the model is not in the process**. So there is no HTTP client here, no API
//! key, no `async`, and nothing that can fail at a network boundary. Stemma writes a
//! [`Briefing`] — everything a model needs to know about a language, rendered
//! deterministically from the genome — and reads back a [`Proposal`]: a file, in the
//! same formats a human writes, which the engine then applies **by the ordinary code
//! path** or refuses.
//!
//! That the model runs elsewhere is not a limitation dressed up as a principle. It is
//! what makes the guarantee checkable: the path from a model's output to a stored form
//! does not merely avoid skipping the engine, it *runs through a file a human can read
//! first*.
//!
//! # The envelope is the point
//!
//! A [`Proposal`] is an **artefact plus its provenance plus its prose**, and the three
//! are kept apart:
//!
//! - [`Proposal::artefact`] is the only thing the engine ever sees. It is a
//!   `RuleSet`, a `DriftSet` or a concept list — exactly what an author writes, with
//!   no field marking it as machine-made, because the engine must not be able to tell.
//! - [`Proposal::rationale`] is the model's own words. **Never interpreted**, always
//!   printed under a heading that says no part of it reached the engine.
//! - [`Proposal::provenance`] says who wrote it, for the reader — not for the code.
//!
//! Without the envelope a model's reasoning ends up in a `note:` field *inside* the
//! rule set, stored in the language file, indistinguishable from the author's own. The
//! envelope is what keeps prose labelled as prose (the roadmap's phrase).
//!
//! # The review cannot be a second opinion
//!
//! [`review`] does not re-implement the engine's checks. It **runs the real
//! application against a clone and throws the result away** — M16's validate-a-clone
//! rule, and here it makes disagreement impossible by construction. A gate that could
//! say "accepted" where `accept` then refused would be theatre, and
//! `a_review_never_disagrees_with_what_accepting_would_do` holds it to that.

pub mod brief;
pub mod render;

pub use brief::render_briefing;
pub use render::render_verdict;

use serde::{Deserialize, Serialize};
use stem_core::{LanguageId, Result, StemmaError, ValidationReport};
use stem_genome::LanguageGenome;

// ------------------------------------------------------------------- the ask

/// What kind of artefact a briefing asks for.
///
/// Exactly the three the roadmap names, and no more: *the model proposes a `RuleSet`,
/// a `DriftSet`, or a concept list — the same authored artefacts a human writes.*
/// Adding a fourth means giving the engine a fourth apply path to run it through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProposalKind {
    /// An ordered set of sound changes, in M10's `.sc` syntax.
    Rules,
    /// A set of meaning shifts (M9).
    Drift,
    /// Concepts this language names and the built-in list does not hold (M12).
    Concepts,
}

impl ProposalKind {
    /// Its name, for the briefing header.
    pub fn name(self) -> &'static str {
        match self {
            Self::Rules => "a sound-change rule set",
            Self::Drift => "a semantic drift set",
            Self::Concepts => "a list of project concepts",
        }
    }

    /// The command that will consume it once accepted — so the briefing can say what
    /// will actually happen rather than leaving it implied.
    pub fn applied_by(self) -> &'static str {
        match self {
            Self::Rules => "stemma apply-rules",
            Self::Drift => "stemma drift",
            Self::Concepts => "stemma declare-concept",
        }
    }
}

// -------------------------------------------------------------- the envelope

/// Who wrote a proposal. **For the reader, never for the code.**
///
/// Nothing in the engine branches on this field, and nothing may be added that does.
/// A rule set proposed by a model and one written by a person are the same artefact
/// and get the same treatment — if the engine could tell them apart, there would be
/// two code paths, and the second one is precisely what §3.2 forbids.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Who or what produced it: a model name, a person, a script.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// How it was produced — free text, uninterpreted. `"from `stemma brief`"`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub method: String,
}

/// The thing the engine actually gets. One variant per apply path.
///
/// `#[non_exhaustive]`, and the labels live here for the `ScriptKind` reason: a
/// downstream wildcard arm would silently accept a variant nothing knows how to apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Artefact {
    /// An ordered rule set, ready for `apply_rules`.
    Rules(stem_soundchange::RuleSet),
    /// A drift set, ready for `apply_drift`.
    Drift(stem_lexicon::DriftSet),
    /// Project concepts, ready for `Edit::DeclareConcept`.
    Concepts(Vec<stem_lexicon::ProjectConcept>),
}

impl Artefact {
    /// Which kind this is.
    pub fn kind(&self) -> ProposalKind {
        match self {
            Self::Rules(_) => ProposalKind::Rules,
            Self::Drift(_) => ProposalKind::Drift,
            Self::Concepts(_) => ProposalKind::Concepts,
        }
    }

    /// A one-line description of what it contains, for the verdict.
    pub fn summary(&self) -> String {
        match self {
            Self::Rules(set) => format!("`{}` — {} rule(s)", set.id, set.rules.len()),
            Self::Drift(set) => format!("`{}` — {} event(s)", set.id, set.events.len()),
            Self::Concepts(list) => format!("{} concept(s)", list.len()),
        }
    }
}

/// A proposal: an artefact, who wrote it, and why they say they wrote it.
///
/// Written to a `.ron` or `.json` file and loaded with `stem_io` like everything else,
/// because a file a human can read before applying is the whole safety story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    /// A stable id for this proposal, so a verdict can name it.
    pub id: String,
    /// Which language it was written for.
    ///
    /// Checked against the genome before anything else happens: a rule set written for
    /// a different language may well be *valid* and still be nonsense here, and that
    /// is a mistake a tired author makes at least as often as a model does.
    pub target: LanguageId,
    /// Who wrote it. Never branched on.
    #[serde(default)]
    pub provenance: Provenance,
    /// **The model's own words.** Never parsed, never matched, never stored in a
    /// language file. Rendered under a heading that says so.
    ///
    /// It exists so that reasoning has somewhere to go that is *not* a `note:` field
    /// inside the rule set — which is where it would otherwise end up, in the language
    /// file, indistinguishable from the author's.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    /// The only thing the engine sees.
    pub artefact: Artefact,
}

// --------------------------------------------------------------- the verdict

/// What happened when a proposal was tried.
///
/// Not persisted (the `CognateTable` precedent) — it describes one attempt.
#[derive(Debug, Clone)]
pub struct Verdict {
    /// Whether the engine would accept it.
    pub accepted: bool,
    /// Everything the engine said, at the engine's own severities.
    pub report: ValidationReport,
    /// One sentence stating the outcome and the reason.
    pub summary: String,
}

/// Reviews a proposal against the language it targets — a **dry run of the real
/// thing**.
///
/// This does not re-check the artefact against a private copy of the engine's rules.
/// It performs the actual application against a clone and discards the result, so a
/// verdict cannot differ from what [`accept`] would do. M16's validate-a-clone rule,
/// and the reason the gate is not theatre.
///
/// # Failure
///
/// None. A proposal the engine refuses comes back as a [`Verdict`] with
/// `accepted: false` and the refusal in `report` — refusal is an outcome to be
/// reported, not an error to be raised. The only `Err` is a proposal that names a
/// language this is not.
pub fn review(proposal: &Proposal, genome: &LanguageGenome) -> Result<Verdict> {
    check_target(proposal, genome)?;

    // The dry run. `evolve` / `with_drift` / `apply_edit` all validate-then-commit and
    // none of them mutates its argument, so the clone is discarded and the language on
    // disk is untouched whatever happens here.
    let attempt = attempt(proposal, genome);
    Ok(verdict_of(proposal, attempt))
}

/// Applies a proposal, producing the next stage of the lineage.
///
/// The same call [`review`] makes, with the result **kept** instead of discarded — so
/// there is exactly one apply path and a review cannot promise something this does not
/// deliver.
///
/// # Failure
///
/// A refused proposal is an `Err`, because the caller asked for a language and there
/// is not one. Use [`review`] to ask the question without wanting the answer.
pub fn accept(
    proposal: &Proposal,
    genome: &LanguageGenome,
    id: &str,
    name: &str,
    elapsed_years: i32,
) -> Result<(LanguageGenome, ValidationReport)> {
    check_target(proposal, genome)?;

    match &proposal.artefact {
        // Nothing here does any linguistics. Every arm hands the artefact to the
        // function an authored file would have reached, with no branch on provenance
        // and no second implementation of anything.
        Artefact::Rules(set) => genome.evolve(id, name, set, elapsed_years),
        Artefact::Drift(set) => genome.drift(id, name, set, elapsed_years),
        Artefact::Concepts(list) => {
            // One `Edit` per concept, through the M16 gate — so a proposal that
            // declares five concepts and gets the fourth wrong is refused whole, by
            // `?`, exactly as the CLI would refuse it one command at a time.
            let mut next = genome.clone();
            let mut report = ValidationReport::new();
            for concept in list {
                let outcome = stem_genome::apply_edit(
                    &next,
                    &stem_genome::Edit::DeclareConcept {
                        key: concept.key.clone(),
                        gloss: concept.gloss.clone(),
                        part_of_speech: concept.part_of_speech,
                        note: concept.note.clone(),
                    },
                )?;
                // `apply_edit` reports what an edit *introduced*, at Warning and Note
                // only — an edit that introduced an Error was refused above.
                for issue in outcome.introduced {
                    report.push(issue);
                }
                next = outcome.genome;
            }
            next.id = LanguageId::new(id);
            next.name = name.to_owned();
            Ok((next, report))
        }
    }
}

/// The dry run, as a `Result` the verdict is built from.
fn attempt(proposal: &Proposal, genome: &LanguageGenome) -> Result<ValidationReport> {
    accept(
        proposal,
        genome,
        // A throwaway identity. It never reaches disk: the genome this produces is
        // dropped at the end of this function, and only the report survives.
        "review_dry_run",
        "review (dry run)",
        0,
    )
    .map(|(_, report)| report)
}

/// A proposal written for a different language is refused before anything is tried.
///
/// Not a validation issue but an outright error: the artefact might be perfectly valid
/// and still be about somebody else's phonology, and applying it would produce a
/// language that traced back to rules nobody wrote for it.
fn check_target(proposal: &Proposal, genome: &LanguageGenome) -> Result<()> {
    if proposal.target == genome.id {
        return Ok(());
    }
    let mut report = ValidationReport::new();
    report.error(
        "wrong_target",
        format!(
            "proposal `{}` was written for `{}`, and this language is `{}`; a rule set \
             is written against one inventory and means something else against another",
            proposal.id, proposal.target, genome.id
        ),
    );
    Err(StemmaError::Invalid(
        format!("proposal `{}` does not target this language", proposal.id),
        report,
    ))
}

fn verdict_of(proposal: &Proposal, attempt: Result<ValidationReport>) -> Verdict {
    match attempt {
        Ok(report) => Verdict {
            accepted: true,
            summary: format!(
                "accepted — {} applies cleanly and would produce a traced, reproducible \
                 language",
                proposal.artefact.summary()
            ),
            report,
        },
        // `StemmaError::Invalid` carries the engine's own report, so the refusal is
        // reported in the engine's words at the engine's severities rather than being
        // paraphrased here.
        Err(StemmaError::Invalid(message, report)) => Verdict {
            accepted: false,
            summary: format!("refused — {message}; nothing was applied"),
            report,
        },
        Err(other) => {
            let mut report = ValidationReport::new();
            report.error("proposal_failed", other.to_string());
            Verdict {
                accepted: false,
                summary: format!("refused — {other}; nothing was applied"),
                report,
            }
        }
    }
}
