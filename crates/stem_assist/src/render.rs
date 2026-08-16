//! Printing a verdict, and the one place a model's prose is shown (M22).
//!
//! Pure, deterministic, no map/float/clock — the `script_view` precedent.
//!
//! # Labelling prose as prose
//!
//! The roadmap's clause is *"prose it writes is labelled as prose"*. That is not a
//! formatting nicety. A rationale and a validation report look alike on a terminal —
//! both are sentences about a language — and the difference between them is the whole
//! of §3.2: one is the engine's finding, the other is somebody's argument for a change
//! the engine has not yet looked at.
//!
//! So the rationale is fenced under a heading that says whose words they are and that
//! nothing in them was read, and it is printed **before** the verdict, in the order
//! the two things actually happened: first somebody claimed something, then the engine
//! checked. A rationale printed underneath an "accepted" would read as though the
//! engine had endorsed it.

use std::fmt::Write;

use stem_core::{Result, StemmaError};

use crate::{Proposal, Verdict};

/// Renders a proposal and the engine's verdict on it.
pub fn render_verdict(proposal: &Proposal, verdict: &Verdict) -> Result<String> {
    let mut out = String::new();

    writeln!(out, "Proposal `{}` — for {}", proposal.id, proposal.target).map_err(fmt_err)?;
    writeln!(
        out,
        "  {}  ·  {}",
        proposal.artefact.summary(),
        proposal.artefact.kind().name()
    )
    .map_err(fmt_err)?;
    if !proposal.provenance.author.is_empty() {
        writeln!(
            out,
            "  written by {}{}",
            proposal.provenance.author,
            if proposal.provenance.method.is_empty() {
                String::new()
            } else {
                format!(" ({})", proposal.provenance.method)
            }
        )
        .map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    // The rationale, fenced and attributed. Printed FIRST, because that is the order
    // it happened in: a claim, and then a check of the claim.
    if !proposal.rationale.is_empty() {
        writeln!(out, "  ── the proposer's own words, not the engine's ──").map_err(fmt_err)?;
        for line in proposal.rationale.lines() {
            writeln!(out, "  │ {line}").map_err(fmt_err)?;
        }
        writeln!(out, "  ── nothing above was parsed, matched, or stored ──").map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
    }

    // And then the engine's answer, in the engine's own words at its own severities.
    writeln!(out, "  {}", verdict.summary).map_err(fmt_err)?;
    if !verdict.report.issues.is_empty() {
        writeln!(out).map_err(fmt_err)?;
        for issue in &verdict.report.issues {
            writeln!(out, "  {issue}").map_err(fmt_err)?;
        }
    }

    Ok(out)
}

/// `render_paradigm`'s policy verbatim.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "verdict",
        message: "formatting a verdict into a string failed".to_owned(),
    }
}
