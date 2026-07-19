//! The phoneme: one contrastive sound in one language's inventory.

use std::fmt;

use serde::{Deserialize, Serialize};
use stem_core::PhonemeId;

/// The broad class of a segment.
///
/// This is intentionally coarse. The fine-grained description of a sound —
/// place, manner, voicing, height, backness, rounding — belongs in the feature
/// bundle arriving in M1 (`DESIGN.md` §7.1). `SegmentKind` answers only the
/// question the engine needs before features exist: *can this be a syllable
/// nucleus?*
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// A consonant — an onset or coda segment.
    Consonant,
    /// A vowel — a syllable nucleus.
    Vowel,
}

impl SegmentKind {
    /// Whether segments of this kind can carry a syllable.
    ///
    /// A language with no nucleus-bearing segment cannot form a spoken syllable,
    /// which is the check behind `phonology.no_nucleus` (`DESIGN.md` §16.5).
    pub fn is_nucleus(self) -> bool {
        matches!(self, Self::Vowel)
    }

    /// The single-letter code used in phonotactic templates: `C` or `V`.
    pub fn template_symbol(self) -> char {
        match self {
            Self::Consonant => 'C',
            Self::Vowel => 'V',
        }
    }
}

impl fmt::Display for SegmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Consonant => "consonant",
            Self::Vowel => "vowel",
        };
        f.write_str(s)
    }
}

/// One contrastive sound.
///
/// Phonemes are language-scoped: the `/p/` of a proto-language and the `/p/` of
/// its daughter are separate [`Phoneme`] values with separate IDs, because they
/// have separate histories. Forking copies them; it does not share them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phoneme {
    /// Stable ID, unique within the owning inventory.
    pub id: PhonemeId,

    /// The IPA representation, e.g. `"t̪"`. Never empty.
    pub ipa: String,

    /// How the phoneme is written in the language's romanisation, when it differs
    /// from the IPA. `None` means "use the IPA form".
    ///
    /// Real orthography — graphemes with their own history — arrives in M9; this
    /// is the practical stand-in until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub romanization: Option<String>,

    /// Broad segment class.
    pub kind: SegmentKind,

    /// Relative likelihood of being chosen during root generation.
    ///
    /// Weights are *relative*, not probabilities — they need not sum to anything.
    /// Uneven weights are what stop generated lexicons from looking like uniform
    /// noise: real inventories have common and rare segments.
    #[serde(default = "default_weight")]
    pub frequency_weight: f32,
}

fn default_weight() -> f32 {
    1.0
}

impl Phoneme {
    /// Builds a phoneme with a default weight of `1.0` and no romanisation.
    pub fn new(id: impl Into<PhonemeId>, ipa: impl Into<String>, kind: SegmentKind) -> Self {
        Self {
            id: id.into(),
            ipa: ipa.into(),
            romanization: None,
            kind,
            frequency_weight: default_weight(),
        }
    }

    /// Sets the romanisation.
    #[must_use]
    pub fn with_romanization(mut self, romanization: impl Into<String>) -> Self {
        self.romanization = Some(romanization.into());
        self
    }

    /// Sets the relative frequency weight.
    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.frequency_weight = weight;
        self
    }

    /// How this phoneme is written: the romanisation if set, else the IPA.
    pub fn written(&self) -> &str {
        self.romanization.as_deref().unwrap_or(&self.ipa)
    }

    /// Whether this phoneme can be a syllable nucleus.
    pub fn is_nucleus(&self) -> bool {
        self.kind.is_nucleus()
    }
}

impl fmt::Display for Phoneme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}/", self.ipa)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_form_falls_back_to_ipa() {
        let plain = Phoneme::new("ph_p", "p", SegmentKind::Consonant);
        assert_eq!(plain.written(), "p");

        let romanized = Phoneme::new("ph_sh", "ʃ", SegmentKind::Consonant).with_romanization("sh");
        assert_eq!(romanized.written(), "sh");
        assert_eq!(
            romanized.ipa, "ʃ",
            "romanisation must not overwrite the IPA"
        );
    }

    #[test]
    fn only_vowels_are_nuclei() {
        assert!(Phoneme::new("ph_a", "a", SegmentKind::Vowel).is_nucleus());
        assert!(!Phoneme::new("ph_k", "k", SegmentKind::Consonant).is_nucleus());
    }

    #[test]
    fn phonemes_display_in_slashes() {
        assert_eq!(
            Phoneme::new("ph_k", "k", SegmentKind::Consonant).to_string(),
            "/k/"
        );
    }
}
