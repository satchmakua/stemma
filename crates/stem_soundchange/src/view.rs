//! Rendering derivations: §11.2's JSON shape and `stemma trace`'s text.
//!
//! §11.2's example is a **view**, and it ships as one — storing `"before": "k"`
//! beside a `PhonemeId` is exactly the desynchronising second source of truth M2
//! outlawed. Nothing here is denormalised: the trace stores `RuleId` + `index`,
//! and rendering requires `applied_rules`, which lives on the same genome and
//! which M4's descendant carries by construction.

use std::fmt::Write;

use serde::Serialize;
use stem_core::{PhonemeId, Result, StemmaError, ValidationReport};
use stem_lexicon::{RuleApplication, SymbolResolution, WordEntry};
use stem_phonology::prosody::Stress;
use stem_phonology::{PhonemeInventory, Root};

use crate::rule::SoundChangeRule;

/// §11.2's trace object, field for field.
#[derive(Debug, Serialize)]
pub struct TraceView {
    /// `WordEntry.id`.
    pub word_id: String,
    /// The pre-rule form, `written()` — `"takala"`.
    pub input: String,
    /// The rule's `name` — `"Intervocalic voicing"`.
    pub rule: String,
    /// One entry per site.
    pub matches: Vec<MatchView>,
    /// The post-rule form, `written()` — `"tagala"`.
    pub output: String,
}

/// One match, rendered.
#[derive(Debug, Serialize)]
pub struct MatchView {
    /// `[at, at + 1)` over the pre-rule segment sequence — `[2, 3]`.
    pub span: [u32; 2],
    /// The segment before, `written()` — `"k"`.
    pub before: String,
    /// The segment after, `written()`; `"∅"` on a deletion — `"g"`.
    pub after: String,
    /// Left and right context joined around `_` — `"a _ a"`.
    pub environment: String,
    /// Not in §11.2, added because it is the fact §3.3 most demands and the one a
    /// rendered before/after pair cannot carry: this application created a new
    /// phoneme.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub innovated: Option<String>,
}

/// Renders one rule application as §11.2's object.
///
/// `input` is the form as it stood **before** this application — an intermediate
/// from [`Derivation::replay`], not the stored form.
pub fn view(
    application: &RuleApplication,
    input: &Root,
    entry: &WordEntry,
    rules: &[SoundChangeRule],
    inventory: &PhonemeInventory,
) -> Result<TraceView> {
    let rule_name = rules
        .get(application.index as usize)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| application.rule.to_string());

    let mut output = input.clone();
    let mut sites: Vec<_> = application.sites.iter().collect();
    sites.sort_by_key(|s| std::cmp::Reverse(s.at));
    for site in &sites {
        match &site.after {
            Some(id) => {
                output.replace_at(site.at as usize, id.clone());
            }
            None => {
                output.remove_at(site.at as usize);
            }
        }
    }

    let mut matches = Vec::with_capacity(application.sites.len());
    for site in &application.sites {
        let written =
            |id: &PhonemeId| -> Result<String> { Ok(inventory.require(id)?.written().to_owned()) };
        let context = |slots: &[Option<PhonemeId>]| -> Result<Vec<String>> {
            slots
                .iter()
                .map(|slot| match slot {
                    Some(id) => written(id),
                    None => Ok("#".to_owned()),
                })
                .collect()
        };

        // Outward-recorded, surface-rendered: left slots reverse into reading
        // order.
        let mut left = context(&site.left)?;
        left.reverse();
        let right = context(&site.right)?;
        let environment = format!("{} _ {}", left.join(" "), right.join(" "))
            .trim()
            .to_owned();

        matches.push(MatchView {
            span: [site.at, site.at + 1],
            before: written(&site.before)?,
            after: match &site.after {
                Some(id) => written(id)?,
                None => "∅".to_owned(),
            },
            environment,
            innovated: match &site.resolution {
                Some(SymbolResolution::Innovated { ipa, .. }) => Some(ipa.clone()),
                _ => None,
            },
        });
    }

    Ok(TraceView {
        word_id: entry.id.to_string(),
        input: input.written(inventory)?,
        rule: rule_name,
        matches,
        output: output.written(inventory)?,
    })
}

/// Renders a word's whole derivation for `stemma trace` and the M11 UI.
///
/// Takes `applied_rules` because a trace stores `RuleId` and nothing else —
/// nothing is denormalised, so a renamed rule cannot leave a stale label behind —
/// and because "did not apply" is computed as `applied_rules` minus the rules
/// named in `steps`, which needs the list.
pub fn render_derivation(
    entry: &WordEntry,
    applied_rules: &[SoundChangeRule],
    inventory: &PhonemeInventory,
) -> Result<String> {
    let mut out = String::new();
    let push = |out: &mut String, line: &str| -> Result<()> {
        writeln!(out, "{line}").map_err(|e| {
            let mut report = ValidationReport::new();
            report.error("trace.write_failed", e.to_string());
            StemmaError::Invalid("rendering the derivation".to_owned(), report)
        })
    };

    let gloss = entry.display_gloss().unwrap_or("?");

    let Some(derivation) = &entry.trace else {
        push(
            &mut out,
            &format!(
                "{}  \"{}\"  {}",
                entry.written(inventory)?,
                gloss,
                entry.cognate_set
            ),
        )?;
        push(&mut out, "")?;
        push(
            &mut out,
            "no derivation recorded — this word has never been passed through a sound \
             change; `stemma apply-rules` is what writes one",
        )?;
        return Ok(out);
    };

    let input = &derivation.input;
    push(
        &mut out,
        &format!(
            "*{}  \"{}\"  {}",
            input.written(inventory)?,
            gloss,
            entry.cognate_set
        ),
    )?;
    push(&mut out, "")?;
    push(
        &mut out,
        &format!(
            "  proto      {:<12}  /{}/",
            input.written(inventory)?,
            input.ipa(inventory)?
        ),
    )?;
    if input.syllables.iter().any(|s| s.stress.is_some()) {
        push(
            &mut out,
            &format!(
                "  │ stress   {}",
                stressed_syllabification(input, inventory)?
            ),
        )?;
    }
    push(&mut out, "  │")?;

    let intermediates = derivation.replay();
    let mut step_cursor = 0usize;
    let mut current: &Root = input;

    for (index, rule) in applied_rules.iter().enumerate() {
        let step = derivation
            .steps
            .get(step_cursor)
            .filter(|s| s.index as usize == index);

        match step {
            None => {
                push(
                    &mut out,
                    &format!("  {index}  {}  {} — did not apply", rule.id, rule.name),
                )?;
                push(&mut out, "  │")?;
            }
            Some(application) => {
                push(&mut out, &format!("  {index}  {}  {}", rule.id, rule.name))?;
                let rendered = view(application, current, entry, applied_rules, inventory)?;
                for m in &rendered.matches {
                    push(
                        &mut out,
                        &format!(
                            "  │    {} > {}  [{},{})   environment  {}",
                            m.before, m.after, m.span[0], m.span[1], m.environment
                        ),
                    )?;
                    if let Some(glyph) = &m.innovated {
                        push(
                            &mut out,
                            &format!("  │    /{glyph}/ is new to this language  (reference table)"),
                        )?;
                    }
                }
                for site in &application.sites {
                    if let Some(pattern) = &site.emptied_syllable {
                        push(
                            &mut out,
                            &format!(
                                "  │    a syllable (pattern {pattern}) emptied and was removed"
                            ),
                        )?;
                    }
                }
                for blocked in &application.blocked {
                    push(
                        &mut out,
                        &format!(
                            "  │    matched at [{},{}) and was refused: {:?}",
                            blocked.at,
                            blocked.at + 1,
                            blocked.reason
                        ),
                    )?;
                }
                let after = &intermediates[step_cursor];
                push(
                    &mut out,
                    &format!(
                        "  │    → {:<10}  /{}/",
                        after.written(inventory)?,
                        after.ipa(inventory)?
                    ),
                )?;
                push(&mut out, "  │")?;
                current = &intermediates[step_cursor];
                step_cursor += 1;
            }
        }
    }

    let final_form = derivation
        .replay()
        .pop()
        .unwrap_or_else(|| derivation.input.clone());
    push(
        &mut out,
        &format!(
            "  modern     {:<12}  /{}/",
            final_form.written(inventory)?,
            final_form.ipa(inventory)?
        ),
    )?;

    Ok(out)
}

/// `TA.ka.la` — syllables joined with dots, the primary-stressed one uppercased.
fn stressed_syllabification(root: &Root, inventory: &PhonemeInventory) -> Result<String> {
    let mut parts = Vec::with_capacity(root.syllables.len());
    for syllable in &root.syllables {
        let mut text = String::new();
        for id in &syllable.segments {
            text.push_str(inventory.require(id)?.written());
        }
        if syllable.stress == Some(Stress::Primary) {
            text = text.to_uppercase();
        }
        parts.push(text);
    }
    Ok(parts.join("."))
}
