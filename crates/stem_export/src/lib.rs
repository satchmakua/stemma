//! Rendering a language as a document a person or another tool reads
//! (`DESIGN.md` §19).
//!
//! Split from `stem_io` deliberately, and the split is the point
//! (`docs/adr/0006`). Persistence is a **total, reversible, domain-blind**
//! mapping: `stem_io` is generic over serde and its module docs forbid it from
//! knowing the domain. Rendering is a **lossy, opinionated, domain-specific**
//! projection: a dictionary knows that a gloss column comes before a part of
//! speech, and a word's rendered form cannot even be computed without the
//! language's phoneme inventory. They both end in a file, and that is the only
//! thing they share.
//!
//! # Why `&mut String`
//!
//! Writers append to a `String` through [`std::fmt::Write`], not
//! [`std::io::Write`]. Two reasons, one of them forced.
//!
//! *Composition*: at M6 a family document appends these as sub-renderers into one
//! buffer, and at M11 the UI renders without touching the filesystem at all.
//!
//! *Correctness*: `StemmaError::Io` mandates a `path`, which a renderer writing
//! into a buffer does not have. An `io::Write` signature would need either a new
//! `stem_core` error variant — in the crate `docs/adr/0002` protects — or a
//! fabricated `path: "<output>"`, which is a lie in a user-facing message. With
//! `fmt::Write` into a `String` there is no I/O failure to report, so the `Result`
//! these functions return carries only *real* domain failures: a segment that is
//! not in the inventory, an entry with no gloss.
//!
//! # What must not happen here
//!
//! No map on any path that reaches output. No sort. No float. No locale. No
//! timestamp, input filename, or version string. §9.4 requires the bytes to be a
//! pure function of the genome, and the cheapest way to guarantee a stable sort is
//! not to sort at all. Two tests read these source files to enforce it.

pub mod csv;
pub mod markdown;

pub use csv::{write_lexicon_csv, write_lexicon_csv_header, write_lexicon_csv_rows};
pub use markdown::write_lexicon_markdown;

#[cfg(test)]
mod tests {
    /// The rules in the module docs, enforced by reading the sources. Crude, and
    /// honest about being so — the alternative is a runtime guard over something
    /// that is a coding rule, not a value.
    #[test]
    fn the_renderers_use_no_hash_map_and_no_float() {
        for (name, src) in [
            ("markdown.rs", include_str!("markdown.rs")),
            ("csv.rs", include_str!("csv.rs")),
        ] {
            for (n, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                for banned in ["HashMap", "HashSet", "f32", "f64", ".sort"] {
                    assert!(
                        !code.contains(banned),
                        "{name}:{} uses `{banned}`, which §9.4 forbids on a render path",
                        n + 1
                    );
                }
            }
        }
    }
}
