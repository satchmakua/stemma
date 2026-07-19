//! The Markdown dictionary (`DESIGN.md` §19.1).

use std::fmt::Write;

use stem_core::{Result, StemmaError, ValidationReport};
use stem_genome::LanguageGenome;

/// Renders a language's lexicon as a Markdown dictionary, appending to `out`.
///
/// Fails only on conditions `validate` reports at **Error** severity — a segment
/// outside the inventory, an entry with no gloss. Nothing that merely warns can
/// make an export refuse: an entry naming an unknown concept renders its raw key
/// and its own gloss. A validator and an engine that disagree about whether a
/// language works is the defect M1's review caught, and this is the second
/// direction of it.
pub fn write_lexicon_markdown(out: &mut String, genome: &LanguageGenome) -> Result<()> {
    let inventory = &genome.phonemes;

    writeln!(out, "# {} — lexicon", escape(&genome.name)).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    let lineage = match &genome.parent {
        Some(parent) => format!("daughter of `{parent}`, +{}y", genome.lineage_depth_years),
        None => "proto-language".to_owned(),
    };
    writeln!(out, "`{}` · {lineage} · seed {}", genome.id, genome.seed).map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    if genome.lexicon.is_empty() {
        writeln!(
            out,
            "This language has no lexicon yet. Run `stemma new-lexicon` to seed one."
        )
        .map_err(fmt_err)?;
        return Ok(());
    }

    writeln!(
        out,
        "{}, seeded from the built-in Swadesh-style concept list.",
        genome.lexicon.summary()
    )
    .map_err(fmt_err)?;
    writeln!(out, "Forms are romanised; IPA in slashes.").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    writeln!(
        out,
        "| Form | IPA | Gloss | POS | Concept | Word | Cognate set |"
    )
    .map_err(fmt_err)?;
    writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |").map_err(fmt_err)?;

    for entry in genome.lexicon.iter() {
        let gloss = entry.display_gloss().ok_or_else(|| missing_gloss(entry))?;
        let concept = match &entry.concept {
            Some(key) => format!("`{}`", escape(key.as_str())),
            None => "—".to_owned(),
        };
        writeln!(
            out,
            "| {} | /{}/ | {} | {} | {concept} | `{}` | `{}` |",
            escape(&entry.written(inventory)?),
            escape(&entry.ipa(inventory)?),
            escape(gloss),
            entry.part_of_speech,
            escape(entry.id.as_str()),
            escape(entry.cognate_set.as_str()),
        )
        .map_err(fmt_err)?;
    }

    writeln!(out).map_err(fmt_err)?;
    writeln!(out, "## Etymology").map_err(fmt_err)?;
    writeln!(out).map_err(fmt_err)?;

    // M3 replaces this paragraph with real derivations. Until a rule has applied,
    // the honest thing to say is that nothing has — saying nothing at all would
    // leave a §19.1 section silently missing.
    match &genome.parent {
        None => {
            writeln!(
                out,
                "{} is the root of its family, so no entry has an ancestor and no sound change",
                escape(&genome.name)
            )
            .map_err(fmt_err)?;
            writeln!(
                out,
                "has applied. Every form above was **coined** — drawn from the phoneme inventory"
            )
            .map_err(fmt_err)?;
            writeln!(
                out,
                "under this language's phonotactic templates, on the `lexicon` RNG stream at seed"
            )
            .map_err(fmt_err)?;
            writeln!(
                out,
                "{}, one root per concept in concept-list order.",
                genome.seed
            )
            .map_err(fmt_err)?;
        }
        Some(parent) => {
            writeln!(
                out,
                "{} descends from `{parent}`. Per-word derivations arrive with the sound-change",
                escape(&genome.name)
            )
            .map_err(fmt_err)?;
            writeln!(out, "engine; until then, cognate sets record the descent.")
                .map_err(fmt_err)?;
        }
    }

    writeln!(out).map_err(fmt_err)?;
    writeln!(
        out,
        "`stemma new-lexicon` on this language reproduces every form in this document."
    )
    .map_err(fmt_err)?;

    Ok(())
}

/// Escapes the characters that would break a Markdown table cell.
///
/// Concept glosses are compiled in and safe; a language `name` and an authored
/// `glosses` entry are not. Backslash first, or escaping the pipe would then have
/// its own backslash escaped.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('|', "\\|")
}

/// A `fmt::Write` failure into a `String` cannot actually happen — `String`'s impl
/// is infallible — but the signature is fallible, so it is mapped rather than
/// unwrapped.
fn fmt_err(error: std::fmt::Error) -> StemmaError {
    let mut report = ValidationReport::new();
    report.error("export.write_failed", error.to_string());
    StemmaError::Invalid("rendering the document".to_owned(), report)
}

fn missing_gloss(entry: &stem_lexicon::WordEntry) -> StemmaError {
    let mut report = ValidationReport::new();
    report.error(
        "lexicon.no_gloss",
        format!(
            "`{}` has no gloss and names no concept the built-in list holds, so it cannot be \
             printed as a dictionary headword",
            entry.id
        ),
    );
    StemmaError::Invalid("rendering the document".to_owned(), report)
}
