//! The shared error type.
//!
//! Note the split of responsibility with [`crate::validate`]: an *error* means the
//! engine could not proceed (the file is missing, the RON is malformed). A
//! *validation issue* means the data parsed fine but says something questionable
//! about a language. Stemma reports the latter as a profile rather than a hard
//! failure, because "this language has 80 consonants and 2 vowels" is unusual, not
//! wrong (`DESIGN.md` §17 — guide the creator, don't police them).

use thiserror::Error;

/// Shorthand for a fallible Stemma operation.
pub type Result<T, E = StemmaError> = std::result::Result<T, E>;

/// Everything that can go wrong in the engine.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StemmaError {
    /// A file could not be read or written.
    #[error("could not {action} `{path}`: {source}")]
    Io {
        /// What was being attempted, e.g. `"read"`.
        action: &'static str,
        /// The path involved.
        path: String,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// A project file was syntactically invalid.
    #[error("could not parse `{path}` as {format}: {message}")]
    Parse {
        /// The path involved.
        path: String,
        /// The format that was expected, e.g. `"RON"`.
        format: &'static str,
        /// The parser's own message.
        message: String,
    },

    /// A project file could not be written out.
    #[error("could not serialise to {format}: {message}")]
    Serialize {
        /// The target format.
        format: &'static str,
        /// The serialiser's own message.
        message: String,
    },

    /// The data parsed but is structurally unusable — a dangling reference, a
    /// duplicate ID, an empty inventory. This is validation escalated to fatal.
    #[error("{0} is invalid:\n{1}")]
    Invalid(String, crate::validate::ValidationReport),

    /// A file extension the loader does not recognise.
    #[error("unsupported file format for `{path}` (expected .ron or .json)")]
    UnsupportedFormat {
        /// The path involved.
        path: String,
    },

    /// Something was looked up by ID and was not there.
    #[error("no {kind} with id `{id}`")]
    NotFound {
        /// The entity kind, e.g. `"phoneme"`.
        kind: &'static str,
        /// The ID that missed.
        id: String,
    },
}

impl StemmaError {
    /// Builds an [`StemmaError::Io`] from a path and the operation attempted.
    pub fn io(
        action: &'static str,
        path: impl AsRef<std::path::Path>,
        source: std::io::Error,
    ) -> Self {
        Self::Io {
            action,
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// Builds a [`StemmaError::NotFound`].
    pub fn not_found(kind: &'static str, id: impl std::fmt::Display) -> Self {
        Self::NotFound {
            kind,
            id: id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_names_the_path_and_the_action() {
        let err = StemmaError::io(
            "read",
            "fixtures/missing.ron",
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        );
        let msg = err.to_string();
        assert!(msg.contains("read"), "{msg}");
        assert!(msg.contains("fixtures/missing.ron"), "{msg}");
    }

    #[test]
    fn not_found_names_the_kind() {
        let err = StemmaError::not_found("phoneme", "ph_0009");
        assert_eq!(err.to_string(), "no phoneme with id `ph_0009`");
    }
}
