//! The phoneme: one contrastive sound in one language's inventory.

use std::fmt;

use serde::{Deserialize, Serialize};
use stem_core::PhonemeId;

use crate::features::FeatureBundle;

/// Which slot of a syllable template a segment may fill.
///
/// **`SegmentKind` is a phonotactic slot class, not the feature `[consonantal]`.**
///
/// That distinction is the whole answer to the glide question. /w/ and /j/ are
/// phonologically `[-consonantal]` — they have no radical constriction — yet they
/// fill onset and coda slots and never nuclei. So Proto-Asterian is *correct* to
/// declare `kind: consonant` for both, and it would be wrong to infer
/// `[+consonantal]` from that declaration. A syllabic nasal /n̩/ is the mirror
/// case: `kind: vowel` (it heads a syllable) and `[+consonantal]` (it is a nasal).
/// Both statements are true at once.
///
/// `kind` answers exactly one question: *which slot may this segment fill?*
/// [`crate::Feature::Consonantal`] answers a different one, and
/// [`crate::Feature::Syllabic`] is what a rule means by "vowel".
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
///
/// **`deny_unknown_fields` is deliberate here**, despite the forward-compatibility
/// rule that governs new *declared* fields. Without it, one transposed letter —
/// `frequency_wieght` — is silently accepted, the real field takes its default,
/// and the language quietly becomes a different language: measured, that typo on
/// the fixture's most-frequent vowel changed 17 of 20 generated roots with no
/// diagnostic at any severity. That is precisely the failure `DESIGN.md` §9.4
/// exists to prevent, and it is worth more than tolerating a field from a future
/// version. Old files still load: every field added since M0 carries
/// `#[serde(default)]`, which is what that rule actually requires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

    /// Which template slot this segment may fill. See [`SegmentKind`] — this is a
    /// phonotactic claim, not a phonetic one.
    pub kind: SegmentKind,

    /// What this segment *is*, phonologically (`DESIGN.md` §7.1, §8.2).
    ///
    /// `#[serde(default)]` yields [`FeatureBundle::EMPTY`], so a project file
    /// written before M1 keeps loading — `CLAUDE.md`'s hard constraint — and then
    /// reports `phonology.features_unspecified` at Warning severity, so it loads
    /// *audibly*. M1's generator does not read this field; M3's rules will read
    /// nothing else.
    #[serde(default)]
    pub features: FeatureBundle,

    /// Relative likelihood of being chosen during root generation.
    ///
    /// **Integer, not `f32`, and this is load-bearing.** The weighted-index
    /// sampler performs no internal normalisation: it builds a prefix-sum array in
    /// iterator order and samples one value against it. With floats, mathematically
    /// identical weight sets produce different draws depending on summation order,
    /// and any quantisation step can silently zero a small weight — leaving a
    /// phoneme present in the inventory and absent from every root, with no
    /// diagnostic. Integer prefix sums are exact, and scaling every weight by the
    /// same factor is provably a no-op (`weights_scaled_by_a_constant_draw_
    /// identically` asserts it). `DESIGN.md` §9.4.
    ///
    /// Weights are *relative*, not probabilities; they need not sum to anything.
    /// The total across one candidate class must fit in a `u32` — see
    /// `phonology.weight_sum_overflow`.
    #[serde(default = "default_weight")]
    pub frequency_weight: u32,
}

/// Ten, not one, so an unweighted phoneme sits mid-scale and an author can make
/// one segment rarer without rescaling the whole inventory.
fn default_weight() -> u32 {
    10
}

impl Phoneme {
    /// Builds a phoneme with a default weight, no romanisation, and no features.
    ///
    /// The empty bundle is deliberate: this constructor is used by tests and by
    /// code that does not care about phonology, and forcing every caller to supply
    /// sixteen feature values would make it unusable. `validate` reports the gap.
    pub fn new(id: impl Into<PhonemeId>, ipa: impl Into<String>, kind: SegmentKind) -> Self {
        Self {
            id: id.into(),
            ipa: ipa.into(),
            romanization: None,
            kind,
            features: FeatureBundle::EMPTY,
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
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.frequency_weight = weight;
        self
    }

    /// Sets the feature bundle.
    #[must_use]
    pub fn with_features(mut self, features: FeatureBundle) -> Self {
        self.features = features;
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
