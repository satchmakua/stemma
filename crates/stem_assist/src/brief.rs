//! The briefing: everything a model is told before it proposes anything (M22).
//!
//! A **pure renderer** over the genome — the `render_grammar` / `script_view`
//! precedent (`docs/adr/0006`): no map, no float, no clock, no RNG, so two runs are
//! byte-identical and the briefing you review is the briefing that was sent.
//!
//! # Why the briefing is half the milestone
//!
//! A model asked "write me some sound changes for Asterian" will write plausible
//! *English* about sound changes and formal nonsense underneath — a rule that
//! assimilates to a following consonant while declaring no following slot, a rule
//! devoicing voiced stops in a language that has none. Those are not lies, they are
//! the failure mode of fluency without grounding, and they are exactly what the gate
//! in [`crate::review`] is for.
//!
//! But a gate that only ever says no is a bad tool. The briefing is the other half:
//! state the inventory with its features, state the rules already applied and what
//! each one did, state the grammar the artefact must be written in, and state which
//! mistakes are refused outright. Everything here is derived from the language, so
//! none of it can be out of date with respect to the thing being proposed against.
//!
//! §6.5's "explain a sound change" lives here, and deterministically: the applied
//! rules are explained from the structs, by the engine's own renderer.

use std::fmt::Write;

use stem_core::{Result, StemmaError};
use stem_genome::LanguageGenome;

use crate::ProposalKind;

/// Renders the briefing for `genome` and `kind`.
pub fn render_briefing(genome: &LanguageGenome, kind: ProposalKind) -> Result<String> {
    let mut out = String::new();
    writeln!(out, "# Briefing — {} ({})", genome.name, genome.id).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "You are asked to propose **{}** for this language. Read the whole briefing \
         first: everything below is generated from the language file itself, so none \
         of it is out of date.",
        kind.name()
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // --- the shared context: what this language is.
    writeln!(out, "## The language").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    let (consonants, vowels): (Vec<_>, Vec<_>) = genome
        .phonemes
        .iter()
        .partition(|p| p.kind != stem_phonology::SegmentKind::Vowel);
    writeln!(
        out,
        "{} phonemes — {} consonant(s), {} vowel(s). Seed {}. {} word(s) in the lexicon.",
        genome.phonemes.len(),
        consonants.len(),
        vowels.len(),
        genome.seed,
        genome.lexicon.len()
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // The full feature matrix, because rules match FEATURES and not letters (§7.1).
    // A model given only the IPA will write `p, t, k` where the project wants
    // `[-sonorant, -continuant, -voice]`, and the enumeration is the thing this
    // project most consistently refuses.
    writeln!(out, "### The inventory, with features").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "**Rules match features, not letters.** A rule is written over a feature \
         bundle — `[-sonorant, -continuant, -voice]`, one rule — never as an \
         enumeration of `p`, `t`, `k`."
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    for phoneme in genome.phonemes.iter() {
        writeln!(
            out,
            "- `{}` /{}/ — {}",
            phoneme.id,
            phoneme.ipa,
            feature_list(&phoneme.features)
        )
        .map_err(fmt_err)?;
    }
    writeln!(out).map_err(fmt_err)?;

    // --- what has already happened to it. §6.5's "explain a sound change", rendered
    //     from the structs rather than described in prose somebody typed.
    writeln!(out, "### What has already happened to it").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    if genome.applied_rules.is_empty() {
        writeln!(
            out,
            "Nothing yet — this is a proto-language, and any rule you propose runs \
             first."
        )
        .map_err(fmt_err)?;
    } else {
        writeln!(
            out,
            "{} rule(s) have run, in this order. Order is chronology (§11.3): a rule \
             you add runs *after* all of these, over whatever they left behind.",
            genome.applied_rules.len()
        )
        .map_err(fmt_err)?;
        writeln!(out).map_err(fmt_err)?;
        for (i, rule) in genome.applied_rules.iter().enumerate() {
            writeln!(out, "{i}. `{}` — {}", rule.id, rule.name).map_err(fmt_err)?;
            if !rule.description.is_empty() {
                writeln!(out, "   {}", rule.description).map_err(fmt_err)?;
            }
        }
    }
    writeln!(out).map_err(fmt_err)?;

    // --- the ask, and the grammar of the answer.
    writeln!(out, "## What to produce").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "One file: a **proposal**, in RON. It is an envelope with three parts, and \
         they must stay separate:"
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "```").map_err(fmt_err)?;
    writeln!(out, "(").map_err(fmt_err)?;
    writeln!(out, "    id: \"a_short_id\",").map_err(fmt_err)?;
    writeln!(out, "    target: \"{}\",", genome.id).map_err(fmt_err)?;
    writeln!(
        out,
        "    provenance: (author: \"...\", method: \"from `stemma brief`\"),"
    )
    .map_err(fmt_err)?;
    writeln!(
        out,
        "    rationale: \"Your reasoning, in prose. Say what you are claiming and \
         why.\","
    )
    .map_err(fmt_err)?;
    writeln!(out, "    artefact: {},", artefact_shape(kind)).map_err(fmt_err)?;
    writeln!(out, ")").map_err(fmt_err)?;
    writeln!(out, "```").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "`rationale` is where your reasoning goes, and **the only place it is \
         welcome**. It is printed for the reader and no part of it is ever parsed, \
         matched, or written into the language file. Do not put reasoning inside the \
         artefact's `note:` fields to make a point — those are stored in the language, \
         and the language should read as though a person wrote it."
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    kind_section(&mut out, kind, genome)?;

    // --- and what will happen to it.
    writeln!(out, "## What happens next").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "`stemma review` runs your artefact through **{}** — the same code path an \
         artefact typed by hand goes through — against a throwaway copy, and reports \
         what the engine said. Nothing is written. `stemma accept` does the same and \
         keeps the result.",
        kind.applied_by()
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "There is no path from what you write to a stored form that skips that step, \
         and nothing branches on the fact that a model wrote it. If the engine refuses \
         your proposal, it is refused whole: nothing is partially applied."
    )
    .map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "So: propose something you can defend, say plainly in `rationale` what you are \
         unsure of, and do not smooth over a gap to make the file parse."
    )
    .map_err(fmt_err)?;

    Ok(out)
}

/// The `artefact:` line's shape for each kind.
fn artefact_shape(kind: ProposalKind) -> &'static str {
    match kind {
        ProposalKind::Rules => "rules(( id: \"...\", name: \"...\", rules: [ ... ] ))",
        ProposalKind::Drift => "drift(( id: \"...\", name: \"...\", events: [ ... ] ))",
        ProposalKind::Concepts => "concepts([ (key: \"...\", gloss: \"...\"), ... ])",
    }
}

/// The part of the briefing that differs by kind: the grammar, and the refusals.
fn kind_section(out: &mut String, kind: ProposalKind, genome: &LanguageGenome) -> Result<()> {
    match kind {
        ProposalKind::Rules => {
            writeln!(out, "### Writing the rules").map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "A rule has a target (a feature bundle), an optional environment, and \
                 a change. The environment is an **adjacency window**: `_` is the \
                 target's position, `#` is a word boundary, and each slot is one \
                 segment."
            )
            .map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(out, "```").map_err(fmt_err)?;
            writeln!(out, "(").map_err(fmt_err)?;
            writeln!(out, "    id: \"r_x01\",").map_err(fmt_err)?;
            writeln!(out, "    name: \"Intervocalic voicing\",").map_err(fmt_err)?;
            writeln!(
                out,
                "    description: \"A voiceless stop voices between two vowels.\","
            )
            .map_err(fmt_err)?;
            writeln!(out, "    chronology_years: 250,").map_err(fmt_err)?;
            writeln!(
                out,
                "    target: (features: [\"-sonorant\", \"-continuant\", \"-voice\"]),"
            )
            .map_err(fmt_err)?;
            writeln!(out, "    environment: (").map_err(fmt_err)?;
            writeln!(
                out,
                "        before: [segment((features: [\"+syllabic\"]))],"
            )
            .map_err(fmt_err)?;
            writeln!(
                out,
                "        after: [segment((features: [\"+syllabic\"]))],"
            )
            .map_err(fmt_err)?;
            writeln!(out, "    ),").map_err(fmt_err)?;
            writeln!(out, "    change: set([\"+voice\"]),").map_err(fmt_err)?;
            writeln!(out, ")").map_err(fmt_err)?;
            writeln!(out, "```").map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "Changes are `set([...])`, `delete`, or `copy(features: ..., from: \
                 after(0))` — `copy` for assimilation, where the value comes from a \
                 neighbour and the rule does not know in advance which."
            )
            .map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "The features are a **closed set**; the ones this language uses are \
                 listed above and there are no others. A feature you invent will not \
                 parse."
            )
            .map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;

            writeln!(out, "### What is refused outright").map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            for (code, why) in [
                (
                    "empty_target",
                    "a target that constrains nothing would touch every segment of \
                     every word",
                ),
                (
                    "change_references_unmatched_position",
                    "a `copy` reads a slot the environment never declared — the \
                     commonest way a rule that *reads* correctly is formally \
                     incoherent",
                ),
                (
                    "copy_from_boundary",
                    "a word boundary has no features to donate",
                ),
                (
                    "boundary_not_outermost",
                    "nothing sits past the edge of a word",
                ),
                (
                    "empty_name",
                    "an unnamed rule is unreadable in a derivation",
                ),
            ] {
                writeln!(out, "- `{code}` — {why}.").map_err(fmt_err)?;
            }
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "Warnings are **not** refusals. A rule whose target matches nothing in \
                 the present inventory is reported and still applied, because an \
                 earlier rule in your own set may mint the segment it needs."
            )
            .map_err(fmt_err)?;
        }
        ProposalKind::Drift => {
            writeln!(out, "### Writing the drift").map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "An event names a word by **meaning**, gives the new sense, and names \
                 the mechanism (`metaphor`, `metonymy`, `broadening`, `narrowing`, \
                 `pejoration`, `amelioration`). A drifted word keeps its cognate set: \
                 you are changing what it means, never what it descends from."
            )
            .map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "This language has {} word(s) to draw on. Name meanings that exist.",
                genome.lexicon.len()
            )
            .map_err(fmt_err)?;
        }
        ProposalKind::Concepts => {
            writeln!(out, "### Writing the concepts").map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "A concept is a `key` (upper snake case, CLDF-safe), a `gloss`, and a \
                 `note` saying why this language needs the meaning. The built-in list \
                 already holds {} meanings — a key that collides with one is reported \
                 and dropped, so propose meanings the list does *not* have.",
                stem_lexicon::CONCEPT_COUNT
            )
            .map_err(fmt_err)?;
            writeln!(out).map_err(fmt_err)?;
            writeln!(
                out,
                "Do **not** invent a Concepticon id. An anchor you cannot verify is \
                 worse than no anchor."
            )
            .map_err(fmt_err)?;
        }
    }
    writeln!(out).map_err(fmt_err)?;
    Ok(())
}

/// A phoneme's features, in the canonical text form `stemma features` prints.
///
/// `FeatureBundle::render` and not a second formatter: the briefing must show a model
/// exactly the syntax the parser accepts, and a private rendering here would drift
/// from it the first time either changed.
fn feature_list(bundle: &stem_phonology::FeatureBundle) -> String {
    let rendered = bundle.render();
    if rendered.is_empty() {
        "(no features declared)".to_owned()
    } else {
        rendered
    }
}

/// `render_paradigm`'s policy verbatim: a `std::fmt::Error` in string building means
/// an allocation failure, not a domain error.
fn fmt_err(_: std::fmt::Error) -> StemmaError {
    StemmaError::Serialize {
        format: "briefing",
        message: "formatting a briefing into a string failed".to_owned(),
    }
}
