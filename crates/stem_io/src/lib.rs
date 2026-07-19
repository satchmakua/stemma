//! Project file loading and saving.
//!
//! RON is the primary project format and JSON the interchange format
//! (`DESIGN.md` §19.2). RON is chosen for authored files because it keeps Rust's
//! enum syntax — `kind: vowel` rather than `"kind": "vowel"` — and permits
//! comments, which matters when the file *is* the language and a conlanger wants
//! to explain a choice next to it.
//!
//! Everything here is generic over `serde`. This crate deliberately does not
//! depend on `stem_genome`: persistence should not know the shape of the domain,
//! only how to move it across the filesystem boundary.

pub mod project;

pub use project::{Format, load, load_str, save, to_string};
