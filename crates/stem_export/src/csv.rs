//! CLDF-shaped CSV (`DESIGN.md` §19.3, §19.5).
//!
//! The column names are CLDF's `FormTable` terms used with CLDF semantics, so a
//! future CLDF export is a metadata file rather than a redesign. `FormTable` alone
//! is a conformant Wordlist; `parameters.csv` and `languages.csv` describe a *set*
//! of languages, and at M2 there is one.
//!
//! # Every field is quoted, always
//!
//! RFC 4180 permits it, and it makes the bytes a function of the *schema* rather
//! than of the content. Quote-only-when-necessary means a gloss that gains a comma
//! silently changes the file, which is a golden test that fails for a reason
//! nobody can see. Escaping is doubling (`""`), never a backslash.
//!
//! Hand-rolled rather than taking a `csv` dependency: it is about forty lines,
//! `CLAUDE.md` forbids a heavyweight dependency for a small need, and a crate's
//! quoting *policy* can shift across versions and churn a golden this design calls
//! byte-stable.

use std::fmt::Write;

use stem_core::{Result, StemmaError, ValidationReport};
use stem_genome::LanguageGenome;

/// The CLDF `FormTable` header, plus Stemma's two extensions.
///
/// `Stemma_Source`, **not** `Source`: CLDF's `Source` is a bibliographic
/// reference, and reusing the name would produce a file that looks conformant and
/// is not.
const HEADER: &[&str] = &[
    "ID",
    "Language_ID",
    "Parameter_ID",
    "Concepticon_ID",
    "Form",
    "Segments",
    "Template",
    "Gloss",
    "Part_Of_Speech",
    "Cognateset_ID",
    "Stemma_Source",
];

/// Writes the header row.
///
/// Split from [`write_lexicon_csv_rows`] because at M5 several languages are
/// concatenated into one table, which must emit **one** header. Free to do now,
/// expensive once a golden has been baselined against the fused version.
pub fn write_lexicon_csv_header(out: &mut String) -> Result<()> {
    write_row(out, HEADER.iter().copied())
}

/// Writes one language's rows, with no header.
pub fn write_lexicon_csv_rows(out: &mut String, genome: &LanguageGenome) -> Result<()> {
    let inventory = &genome.phonemes;

    for entry in genome.lexicon.iter() {
        let gloss = entry.display_gloss().ok_or_else(|| missing_gloss(entry))?;

        // Space-separated IPA, per CLDF. This is the column that makes the export
        // scientifically useful rather than decorative: Stemma has the segmentation
        // for free where a field linguist must reconstruct it.
        let mut segments = String::new();
        for (i, id) in entry.segments().enumerate() {
            if i > 0 {
                segments.push(' ');
            }
            segments.push_str(&inventory.require(id)?.ipa);
        }

        let concept = entry.concept.as_ref().map(|k| k.as_str()).unwrap_or("");
        // Empty for a concept with no verified anchor, and for an unresolved key.
        let concepticon = entry
            .concept()
            .and_then(|c| c.concepticon_id)
            .map(|id| id.to_string())
            .unwrap_or_default();

        write_row(
            out,
            [
                // `ID` composes language and word: two sister languages both hold
                // `w_0001`, and a fused M5 table would collide without the scope.
                format!("{}-{}", genome.id, entry.id).as_str(),
                genome.id.as_str(),
                concept,
                concepticon.as_str(),
                // `Form` is the romanisation — the orthographic form CLDF means —
                // while `Segments` is IPA. The asymmetry is deliberate and is
                // exactly what a future session would "fix" wrongly.
                entry.written(inventory)?.as_str(),
                segments.as_str(),
                entry.template().as_str(),
                gloss,
                entry.part_of_speech.name(),
                entry.cognate_set.as_str(),
                entry.source.name(),
            ]
            .into_iter(),
        )?;
    }

    Ok(())
}

/// Writes a complete single-language CSV: header then rows.
pub fn write_lexicon_csv(out: &mut String, genome: &LanguageGenome) -> Result<()> {
    write_lexicon_csv_header(out)?;
    write_lexicon_csv_rows(out, genome)
}

/// One row, every field quoted, `\n`-terminated.
///
/// `\n` and not `\r\n`: RFC 4180 says CRLF, but every consumer accepts LF, the
/// repo pins `*.csv` to `eol=lf`, and mixing them would make the goldens
/// platform-dependent — which §9.4 forbids.
fn write_row<'a>(out: &mut String, fields: impl Iterator<Item = &'a str>) -> Result<()> {
    for (i, field) in fields.enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for c in field.chars() {
            if c == '"' {
                out.push('"');
            }
            out.push(c);
        }
        out.push('"');
    }
    writeln!(out).map_err(fmt_err)
}

fn fmt_err(error: std::fmt::Error) -> StemmaError {
    let mut report = ValidationReport::new();
    report.error("export.write_failed", error.to_string());
    StemmaError::Invalid("rendering the table".to_owned(), report)
}

fn missing_gloss(entry: &stem_lexicon::WordEntry) -> StemmaError {
    let mut report = ValidationReport::new();
    report.error(
        "lexicon.no_gloss",
        format!(
            "`{}` has no gloss and names no concept the built-in list holds, so its Gloss \
             column would be empty",
            entry.id
        ),
    );
    StemmaError::Invalid("rendering the table".to_owned(), report)
}
