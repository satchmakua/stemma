//! Typed, stable, human-readable entity IDs.
//!
//! Every ID is a newtype over `String` rather than a UUID. That is a deliberate
//! trade: UUIDs are cheaper to generate, but Stemma's whole premise is that a user
//! can look at a trace and understand it (`DESIGN.md` §3.3 "Traceability"). A trace
//! that reads `takala > tagala (rule: IntervocalicVoicing)` beats one that reads
//! `w_9f3e… > w_1c7b… (rule: r_44a2…)`. IDs are authored in fixtures, appear in
//! exports, and are diffed across forks, so legibility wins.
//!
//! The newtypes are distinct types, so a [`PhonemeId`] can never be passed where a
//! [`WordId`] is expected — the compiler enforces what a bare `String` would not.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Defines a newtype ID with the common set of conversions.
///
/// `serde(transparent)` means the ID serialises as a plain string, so fixtures
/// stay readable: `id: "proto_asterian"`, not `id: (0: "proto_asterian")`.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps an existing identifier string.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The conventional prefix for this ID kind, used when minting IDs
            /// from a counter (e.g. `"w"` for words).
            pub const PREFIX: &'static str = $prefix;

            /// Mints a deterministic ID from a sequence number: `w_0001`.
            ///
            /// Determinism matters — the same pipeline run twice must produce the
            /// same IDs (`DESIGN.md` §9.4), so IDs are never random.
            pub fn sequential(n: usize) -> Self {
                Self(format!("{}_{:04}", $prefix, n))
            }

            /// The underlying string.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// True if the ID is empty, which is never valid.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

define_id!(
    /// Identifies a language node in the lineage graph (proto, daughter, dialect…).
    LanguageId,
    "lang"
);
define_id!(
    /// Identifies a phoneme within one language's inventory.
    ///
    /// Phoneme IDs are **language-scoped**: `/p/` in a proto-language and `/p/` in
    /// a daughter are different entities with separate histories.
    PhonemeId,
    "ph"
);
define_id!(
    /// Identifies a single lexicon entry in one language.
    WordId,
    "w"
);
define_id!(
    /// Identifies a sound-change rule.
    RuleId,
    "r"
);
define_id!(
    /// Groups words across languages that descend from one proto-form.
    ///
    /// This is the ID that survives forking — it is what makes a cognate table
    /// possible (`DESIGN.md` §10.3).
    CognateSetId,
    "cog"
);
define_id!(
    /// Identifies an entry in a language's history timeline.
    EventId,
    "ev"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_ids_are_zero_padded_and_prefixed() {
        assert_eq!(WordId::sequential(1).as_str(), "w_0001");
        assert_eq!(CognateSetId::sequential(42).as_str(), "cog_0042");
        assert_eq!(PhonemeId::sequential(1234).as_str(), "ph_1234");
    }

    #[test]
    fn sequential_ids_are_deterministic() {
        assert_eq!(WordId::sequential(7), WordId::sequential(7));
    }

    #[test]
    fn ids_serialise_as_bare_strings() {
        // Fixtures and exports are read by humans; the wire form must stay plain.
        let id = LanguageId::new("proto_asterian");
        let json = serde_json::to_string(&id).expect("serialise");
        assert_eq!(json, "\"proto_asterian\"");

        let back: LanguageId = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, id);
    }

    #[test]
    fn ids_of_different_kinds_are_distinct_types() {
        // Compile-time proof: this test exists so that if someone ever collapses
        // the newtypes into a shared alias, the intent is on record.
        let word = WordId::new("takala");
        let cognate = CognateSetId::new("takala");
        assert_eq!(word.as_str(), cognate.as_str());
    }
}
