//! The comparative cognate table as GitHub-flavored Markdown (`DESIGN.md` §10.3).
//!
//! The Markdown sibling of `stem_genome::render_cognate_table` (the monospace
//! terminal renderer, which stays untouched). Both project the same
//! [`CognateTable`] struct; this one for a document, that one for a terminal.

use std::fmt::Write;

use stem_core::Result;
use stem_genome::CognateTable;

use crate::markdown::{escape, fmt_err};

/// Renders a [`CognateTable`] as a Markdown table, appending to `out`.
///
/// Consumes the struct and **nothing else** — never touches a lexicon, never
/// re-resolves a meaning. The meaning→`cognate_set` join was done once against
/// the reference when `cognate_table` built the struct (`docs/adr/0007`);
/// re-querying here would be a second join M9 meaning drift could
/// desynchronise. Root columns mark each reflex `*` (the reconstruction
/// convention, byte-for-byte the rule `render_cognate_table` uses); a gap
/// renders `—`. Header cells are column `name`s — a document reads better as
/// "Coastal Asterian" than the greppable id the terminal renderer prints.
///
/// No map, no sort, no float, no clock — the bytes are a function of the table
/// alone (§9.4).
pub fn write_cognate_table_markdown(out: &mut String, table: &CognateTable) -> Result<()> {
    // Header: | Meaning | <name> | … |
    out.push_str("| Meaning");
    for column in &table.columns {
        write!(out, " | {}", escape(&column.name)).map_err(fmt_err)?;
    }
    out.push_str(" |\n");

    // Separator: one `---` per column, plus the meaning column.
    out.push_str("| ---");
    for _ in &table.columns {
        out.push_str(" | ---");
    }
    out.push_str(" |\n");

    // One row per meaning.
    for row in &table.rows {
        write!(out, "| {}", escape(&row.meaning)).map_err(fmt_err)?;
        for (column, cell) in table.columns.iter().zip(&row.cells) {
            let rendered = match cell {
                // A root column's reflex is a reconstruction — starred inside the
                // cell so a copied cell stays self-describing.
                Some(form) if column.is_root => format!("*{}", escape(form)),
                Some(form) => escape(form),
                None => "—".to_owned(),
            };
            write!(out, " | {rendered}").map_err(fmt_err)?;
        }
        out.push_str(" |\n");
    }

    // Caption: the `*` convention, and any advisory notes the struct carries (so
    // a self-contained document holds what the CLI's `cognates` sends to stderr).
    out.push('\n');
    out.push_str("*Proto-forms (marked \\*) are reconstructions.*\n");
    for note in &table.notes {
        writeln!(out, "- _{}_", escape(note)).map_err(fmt_err)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_genome::{CognateColumn, CognateRow};

    /// A hand-built table with a root column, a daughter, a gap, and a
    /// `|`-bearing meaning — the renderer's tripwire. Engine-independent:
    /// `CognateTable` is a plain struct, so no fixture and no rule change can
    /// move these bytes; a change here means the *renderer* changed (the M1
    /// canary discipline).
    fn canary_table() -> CognateTable {
        CognateTable {
            columns: vec![
                CognateColumn {
                    id: stem_core::LanguageId::new("proto"),
                    name: "Proto".to_owned(),
                    is_root: true,
                },
                CognateColumn {
                    id: stem_core::LanguageId::new("daughter"),
                    name: "Daughter".to_owned(),
                    is_root: false,
                },
            ],
            reference: stem_core::LanguageId::new("proto"),
            rows: vec![
                CognateRow {
                    meaning: "a|b".to_owned(),
                    cognate_set: Some(stem_core::CognateSetId::new("cog_1")),
                    cells: vec![Some("taka".to_owned()), Some("taga".to_owned())],
                },
                CognateRow {
                    meaning: "gone".to_owned(),
                    cognate_set: Some(stem_core::CognateSetId::new("cog_2")),
                    cells: vec![Some("mi".to_owned()), None],
                },
            ],
            notes: Vec::new(),
        }
    }

    #[test]
    fn the_cognate_table_markdown_canary_matches_its_frozen_bytes() {
        let mut out = String::new();
        write_cognate_table_markdown(&mut out, &canary_table()).unwrap();
        let expected = "\
| Meaning | Proto | Daughter |
| --- | --- | --- |
| a\\|b | *taka | taga |
| gone | *mi | — |

*Proto-forms (marked \\*) are reconstructions.*
";
        assert_eq!(out, expected, "the cognate-table renderer drifted");
    }

    #[test]
    fn the_markdown_marks_only_root_columns_with_a_star() {
        let mut out = String::new();
        write_cognate_table_markdown(&mut out, &canary_table()).unwrap();
        assert!(out.contains("*taka"), "the root reflex is starred");
        assert!(
            out.contains("| taga |"),
            "the daughter reflex is not starred"
        );
    }

    #[test]
    fn a_cognate_table_markdown_gap_renders_an_em_dash() {
        let mut out = String::new();
        write_cognate_table_markdown(&mut out, &canary_table()).unwrap();
        assert!(out.contains("| — |"), "a None cell renders an em dash");
    }

    #[test]
    fn a_pipe_in_a_meaning_does_not_break_the_markdown_table() {
        let mut out = String::new();
        write_cognate_table_markdown(&mut out, &canary_table()).unwrap();
        assert!(out.contains("a\\|b"), "a pipe in a cell is escaped");
    }

    /// A self-contained document must carry the advisory notes the CLI's
    /// `cognates` sends to stderr — so the notes branch has byte-level coverage,
    /// including its own escaping.
    #[test]
    fn a_cognate_table_markdown_renders_its_notes_as_escaped_bullets() {
        let mut table = canary_table();
        table.notes = vec!["`dragon` matched no word".to_owned(), "a | pipe".to_owned()];
        let mut out = String::new();
        write_cognate_table_markdown(&mut out, &table).unwrap();
        // The caption comes first, then one italic bullet per note, escaped.
        assert!(
            out.contains("*Proto-forms (marked \\*) are reconstructions.*\n- _`dragon` matched no word_\n- _a \\| pipe_\n"),
            "notes render as escaped italic bullets after the caption:\n{out}"
        );
    }
}
