//! Phonological distinctive features (`DESIGN.md` §7.1): what a sound *is*, not
//! what letter writes it.
//!
//! # The feature set is closed, and generated from one macro invocation
//!
//! An open, auto-registering registry cannot tell a new feature from a misspelled
//! one, so `"+voicee"` becomes a real feature that no segment carries and every
//! rule keyed on it silently matches nothing, forever — exactly the failure
//! `DESIGN.md` §16.5 exists to catch. A closed set makes that a load error naming
//! the file, the token, and the nearest real feature. **It converts the worst
//! failure mode into the best one.**
//!
//! Extending the set is a source change, not a data change. That is cheap and safe
//! because **features are never stored by index**: on disk they are always names,
//! so appending a variant cannot change what any saved file means. See
//! `docs/adr/0004-closed-feature-set.md`.
//!
//! # Three values, not two
//!
//! A cell is `+`, `−`, or **absent**. Absent means "the question does not arise" —
//! a plain alveolar has no rounding value — and it is *not* the same as `−`. That
//! distinction is forced by geometry: `[round]` is only defined on labials,
//! `[high]`/`[low]`/`[back]` only on dorsals. Storing /f/ as `[−strident]` rather
//! than absent would silently swell the class `[−strident]` from the dental
//! fricatives to every non-sibilant in the language, and nothing would report it.
//!
//! Reference stays binary: a rule may mention only `+` and `−`. "The class of
//! segments for which stridency is undefined" is not an attested natural class,
//! and there is deliberately no way to ask for it.

use serde::{Deserialize, Serialize};

/// Generates the feature enum and its lookup tables from a single list.
///
/// The tables must never disagree, and a hand-written `ALL` can silently disagree
/// with the enum: append `Tense` to the enum, forget `ALL`, and `iter()` — which
/// drives serialisation — omits it, so **saving the project silently drops
/// `+tense` from every phoneme**. One macro input makes that unrepresentable.
macro_rules! features {
    ($( $(#[$attr:meta])* $variant:ident => $name:literal ),+ $(,)?) => {
        /// One binary distinctive feature.
        ///
        /// Variants are **append-only**. Declaration order is the canonical
        /// rendering order (see [`FeatureBundle::iter`]) and is frozen by
        /// `feature_names_and_order_are_frozen`. Reordering or renaming rewrites
        /// every rendered bundle in every export.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Feature { $( $(#[$attr])* $variant ),+ }

        impl Feature {
            /// Every feature, in declaration order.
            pub const ALL: &'static [Feature] = &[ $( Feature::$variant ),+ ];

            /// How many features there are.
            pub const COUNT: usize = Self::ALL.len();

            /// The `snake_case` name used on disk and in diagnostics. The *only*
            /// on-disk form — no discriminant is ever written.
            pub fn name(self) -> &'static str {
                match self { $( Feature::$variant => $name ),+ }
            }

            /// Parses a bare (unsigned) feature name.
            pub fn from_name(name: &str) -> Option<Feature> {
                match name { $( $name => Some(Feature::$variant), )+ _ => None }
            }

            /// This feature's bit. Private: bit positions are an implementation
            /// detail and must never reach output.
            fn bit(self) -> u64 { 1u64 << (self as u64) }
        }
    };
}

features! {
    // --- Major class: what kind of sound is this at all? ---
    /// Can head a syllable. This — not [`crate::SegmentKind`] — is what a rule
    /// means by "vowel". A syllabic nasal /n̩/ is `[+syllabic, +consonantal]`.
    Syllabic => "syllabic",
    /// Produced with a radical constriction in the vocal tract. **Glides /w j/ are
    /// `[-consonantal]`** even though they fill consonant slots; see
    /// [`crate::SegmentKind`], which answers the different, phonotactic question.
    Consonantal => "consonantal",
    /// Spontaneous voicing is possible. Separates obstruents from everything else;
    /// `DESIGN.md` §11.1's intervocalic-voicing rule targets `[-sonorant]`.
    Sonorant => "sonorant",
    /// Constriction is open enough for frictionless airflow. Covers vowels,
    /// glides, and lateral approximants — but **not** trills, which have
    /// intermittent closure.
    Approximant => "approximant",

    // --- Manner (§7.2: lenition, cluster simplification) ---
    /// Airflow through the *oral* cavity is not blocked. Nasals are
    /// `[-continuant]`: the oral cavity is closed and the air escapes nasally.
    /// Laterals are treated as `[+continuant]` here (airflow is not blocked, it is
    /// merely lateral); that is a live dispute in the literature and this is the
    /// side Stemma takes.
    Continuant => "continuant",
    /// The velum is lowered. `DESIGN.md` §7.2's nasal place assimilation targets
    /// `[+nasal]`.
    Nasal => "nasal",
    /// Airflow escapes around the side of the tongue.
    Lateral => "lateral",
    /// Produced with intermittent closure — an /r/-type trill. Present so that
    /// trill-hood is a *positive* statement: encoding it as "absence of
    /// approximant" would make absence mean two different things.
    Trill => "trill",

    // --- Laryngeal (§7.2: intervocalic voicing, final devoicing) ---
    /// The vocal folds vibrate.
    Voice => "voice",

    // --- Place (§7.2: assimilation, palatalization, labialization) ---
    /// Articulated with the lips. Rounded vowels are `[+labial]`: rounding is a
    /// labial gesture, which is what makes rounding harmony and labialization one
    /// class rather than two.
    Labial => "labial",
    /// Articulated with the tongue tip or blade.
    Coronal => "coronal",
    /// Articulated with the tongue body. **All vowels are `[+dorsal]`**, as are
    /// velars and both glides — without that, `[+dorsal]` is not a usable class
    /// and `DESIGN.md` §7.1's own example rule ("front vowels trigger
    /// palatalization of preceding velars") cannot be written.
    Dorsal => "dorsal",

    // --- Dorsal dependents: §7.1's "height" and "backness" ---
    /// Tongue body raised.
    High => "high",
    /// Tongue body lowered.
    Low => "low",
    /// Tongue body retracted.
    Back => "back",

    // --- Labial dependent: §7.1's "rounding" ---
    /// Lips rounded. Shared by rounded vowels and labialised consonants /kʷ/.
    Round => "round",
}

/// The bitsets are `u64`, so the feature set may never exceed 64 members without a
/// deliberate widening. Nothing on disk refers to a bit, so the width is free;
/// making the ceiling a build error rather than a runtime hazard is not.
const _: () = assert!(Feature::COUNT <= 64);

/// The value a segment gives a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sign {
    /// The feature is absent from this segment: `[-voice]`.
    Minus,
    /// The feature is present in this segment: `[+voice]`.
    Plus,
}

impl Sign {
    /// The character written on disk.
    pub fn as_char(self) -> char {
        match self {
            Self::Minus => '-',
            Self::Plus => '+',
        }
    }
}

/// A segment's feature matrix: for each feature, `+`, `-`, or **absent**.
///
/// Storage is two bitsets with the invariant `pos & !spec == 0`: `spec` says which
/// features are valued, `pos` says which of those are `+`. Every method preserves
/// it.
///
/// M1 does not exploit the three-valued distinction — it only stores it
/// faithfully, and the required-feature checks in [`crate::inventory`] make sure
/// absence is never *accidental*. M3 needs the distinction, and retrofitting it
/// would be a rewrite.
///
/// **`Ord` is deliberately not derived.** An ordering would be over bit positions,
/// i.e. over enum declaration order — so sorting bundles would put an internal
/// numbering into user-visible output, and widening the storage later would
/// silently reorder every export. Nothing in M1 sorts bundles. If M3 needs a
/// canonical order it sorts by the frozen [`Feature::ALL`] sequence, explicitly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct FeatureBundle {
    spec: u64,
    pos: u64,
}

impl FeatureBundle {
    /// A bundle that values nothing.
    pub const EMPTY: Self = Self { spec: 0, pos: 0 };

    /// The value this bundle gives `feature`, or `None` if it does not value it.
    pub fn get(self, feature: Feature) -> Option<Sign> {
        if self.spec & feature.bit() == 0 {
            return None;
        }
        Some(if self.pos & feature.bit() != 0 {
            Sign::Plus
        } else {
            Sign::Minus
        })
    }

    /// Whether this bundle gives `feature` exactly `sign`.
    pub fn is(self, feature: Feature, sign: Sign) -> bool {
        self.get(feature) == Some(sign)
    }

    /// Whether the feature is valued at all.
    pub fn is_specified(self, feature: Feature) -> bool {
        self.spec & feature.bit() != 0
    }

    /// Sets `feature` to `sign`, returning the previous value if there was one.
    pub fn set(&mut self, feature: Feature, sign: Sign) -> Option<Sign> {
        let previous = self.get(feature);
        self.spec |= feature.bit();
        match sign {
            Sign::Plus => self.pos |= feature.bit(),
            Sign::Minus => self.pos &= !feature.bit(),
        }
        previous
    }

    /// Builder form of [`Self::set`].
    #[must_use]
    pub fn with(mut self, feature: Feature, sign: Sign) -> Self {
        self.set(feature, sign);
        self
    }

    /// Removes `feature`'s value entirely, returning the previous one.
    ///
    /// The inverse of [`Self::set`], and it exists for exactly one reason:
    /// assimilation is a *copy*, so when the donor leaves a cell absent the
    /// recipient's cell must become absent too. On this project's own reference
    /// inventory that is load-bearing rather than theoretical — /m/ taking /t/'s
    /// place must land byte-identically on /n/, and /t/ has no rounding value
    /// while /m/ has `-round`. A copy that could only ever *add* would leave
    /// `-round` behind, and the result would then match no phoneme and no
    /// reference row.
    ///
    /// Preserves the `pos & !spec == 0` invariant.
    pub fn unset(&mut self, feature: Feature) -> Option<Sign> {
        let previous = self.get(feature);
        self.spec &= !feature.bit();
        self.pos &= !feature.bit();
        previous
    }

    /// Builder form of [`Self::unset`].
    #[must_use]
    pub fn without(mut self, feature: Feature) -> Self {
        self.unset(feature);
        self
    }

    /// Overwrites every valued cell of `delta` onto `self`, leaving the rest
    /// alone. `DESIGN.md` §11.1's `voice = true`.
    ///
    /// Purely additive over valued cells — it can never *remove* one. That
    /// asymmetry with [`Self::copy_node`] is deliberate: a literal change states
    /// what the segment becomes, while a copy states where the segment's values
    /// come from, and only the second can legitimately transfer absence.
    #[must_use]
    pub fn overlay(mut self, delta: FeatureBundle) -> Self {
        for (feature, sign) in delta.iter() {
            self.set(feature, sign);
        }
        self
    }

    /// Transfers every cell of `node` from `donor`, **absence included**.
    ///
    /// Returns `None` — meaning the copy does not apply — when `donor` values none
    /// of `node.articulators()`. Lexurgy documents the opposite behaviour as a
    /// trap: a matrix of bare feature variables copies the absent value too and
    /// yields "a nasal with no place of articulation". Since `docs/adr/0004` has
    /// already committed that absent is not minus, the correct semantics here is
    /// stricter — a donor with no place has no place to give, so the site simply
    /// does not apply and says so in the trace.
    ///
    /// Note the distinction that matters: copying an *individual* absence within a
    /// node is correct and required (/m/ taking /t/'s place must lose its rounding
    /// value); copying a *wholly unvalued* node is a no-op that would strand the
    /// target with no place at all.
    #[must_use]
    pub fn copy_node(self, donor: FeatureBundle, node: FeatureNode) -> Option<Self> {
        if !node.articulators().iter().any(|&f| donor.is_specified(f)) {
            return None;
        }
        let mut out = self;
        for &feature in node.features() {
            match donor.get(feature) {
                Some(sign) => {
                    out.set(feature, sign);
                }
                None => {
                    out.unset(feature);
                }
            }
        }
        Some(out)
    }

    /// Every valued feature, in frozen [`Feature::ALL`] order.
    ///
    /// Iteration order is a fixed compiled-in sequence, never a map's. This is what
    /// makes load then save a stable fixpoint, and it satisfies `CLAUDE.md`'s
    /// "never let a `HashMap` reach output" structurally rather than by discipline.
    pub fn iter(self) -> impl Iterator<Item = (Feature, Sign)> {
        Feature::ALL
            .iter()
            .filter_map(move |&f| self.get(f).map(|sign| (f, sign)))
    }

    /// How many features are valued.
    pub fn len(self) -> usize {
        self.spec.count_ones() as usize
    }

    /// Whether the bundle values nothing.
    pub fn is_empty(self) -> bool {
        self.spec == 0
    }

    /// The canonical text form: `"+syllabic -consonantal +sonorant"`.
    ///
    /// Used by `stemma features` and by golden tests. Stable by construction.
    pub fn render(self) -> String {
        self.iter()
            .map(|(f, s)| format!("{}{}", s.as_char(), f.name()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Does this bundle satisfy `pattern` — agree with it on every feature
    /// `pattern` values?
    ///
    /// **This is the one forward-looking affordance in M1 and nothing in M1 calls
    /// it.** It is here because it *is* the argument that the representation
    /// survives M3, made executable: "voiceless stops voice between vowels" is
    /// `subsumes([-sonorant, -continuant, -voice])`, two bit tests over a bundle
    /// M1 already stores, and `features_select_the_intervocalic_voicing_target`
    /// proves it against the real fixture today.
    ///
    /// Note the semantics: the segment must **value** every feature the pattern
    /// values. An unvalued feature is not a match. That is why a core set of
    /// features is mandatory — so "unvalued" always means "genuinely does not
    /// arise" and never "the author forgot".
    pub fn subsumes(self, pattern: FeatureBundle) -> bool {
        (self.spec & pattern.spec) == pattern.spec && ((self.pos ^ pattern.pos) & pattern.spec) == 0
    }
}

/// A closed group of features that move together under assimilation (§7.1's
/// feature geometry).
///
/// Copying an arbitrary `Vec<Feature>` is the wrong primitive, and the reference
/// fixture proves it: copying only `[labial, coronal, dorsal]` from /p/ onto /n/
/// yields a `[+labial]` segment with no rounding value, which resolves to no
/// symbol *and* trips `phonology.missing_required_feature` — an **Error**. The
/// dependents are not extras; they are what makes the node well-formed. Making the
/// node the unit of copy makes the ill-formed case unrepresentable rather than
/// merely diagnosed.
///
/// Closed for `docs/adr/0004`'s reason: an open node registry cannot tell `plaec`
/// from `place`, and a misspelled node would copy nothing, forever, silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureNode {
    /// Place of articulation: the articulators **and their dependents**.
    Place,
    /// The laryngeal node. One feature today; it is here so that a voicing
    /// assimilation rule ("obstruents agree in voicing with a following
    /// obstruent") is expressible without a second mechanism.
    Laryngeal,
}

impl FeatureNode {
    /// Every cell this node carries, in frozen [`Feature::ALL`] order.
    ///
    /// **Derived from the inventory validator's own requirement tables rather than
    /// restated.** `REQUIRED_OF_DORSAL` makes `{high, low, back, round}` mandatory
    /// given `+dorsal`, and `REQUIRED_OF_LABIAL` makes `{round}` mandatory given
    /// `+labial`, so the place node is exactly the articulators plus their union.
    /// One table, so the node and the validator cannot drift apart.
    pub fn features(self) -> &'static [Feature] {
        match self {
            Self::Place => &[
                Feature::Labial,
                Feature::Coronal,
                Feature::Dorsal,
                Feature::High,
                Feature::Low,
                Feature::Back,
                Feature::Round,
            ],
            Self::Laryngeal => &[Feature::Voice],
        }
    }

    /// The cells the donor must value for a copy to mean anything.
    ///
    /// Dependents may legitimately be absent on the donor — a plain alveolar has
    /// no rounding — and their absence is transferred faithfully.
    pub fn articulators(self) -> &'static [Feature] {
        match self {
            Self::Place => &[Feature::Labial, Feature::Coronal, Feature::Dorsal],
            Self::Laryngeal => &[Feature::Voice],
        }
    }

    /// The `snake_case` name, for diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Laryngeal => "laryngeal",
        }
    }
}

/// Why a `features:` list could not be read.
///
/// Every variant names the offending token, because the whole point of closing the
/// feature set is that a mistake becomes diagnosable rather than silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureParseError {
    /// A token with no leading `+` or `-`.
    MissingSign {
        /// The token as written.
        token: String,
    },
    /// A token signed with U+2212 MINUS SIGN or U+00B1 PLUS-MINUS.
    NonAsciiSign {
        /// The token as written.
        token: String,
    },
    /// A token naming no known feature.
    UnknownFeature {
        /// The token as written.
        token: String,
        /// The closest real feature name, when there is one within two edits.
        suggestion: Option<&'static str>,
    },
    /// The same feature valued twice in one list.
    DuplicateFeature {
        /// The feature named twice.
        name: &'static str,
    },
}

impl std::fmt::Display for FeatureParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSign { token } => write!(
                f,
                "feature `{token}` needs a leading `+` or `-`; every feature in a \
                 `features:` list is signed"
            ),
            Self::NonAsciiSign { token } => write!(
                f,
                "feature `{token}` is signed with a non-ASCII character; write \
                 ASCII `+` or `-` (linguistics prose uses U+2212 and U+00B1, and \
                 copy-pasting one here would create a feature literally named \
                 `{token}`)"
            ),
            Self::UnknownFeature { token, suggestion } => match suggestion {
                Some(name) => write!(f, "unknown feature `{token}`; did you mean `{name}`?"),
                None => write!(f, "unknown feature `{token}`"),
            },
            Self::DuplicateFeature { name } => write!(
                f,
                "feature `{name}` is given a value twice; last-wins would make a \
                 contradiction look meaningful, so it is an error"
            ),
        }
    }
}

impl std::error::Error for FeatureParseError {}

/// The signed-name wire form.
///
/// `serde(try_from)` means every error above surfaces as a serde error carrying
/// the position in the file, which is what turns a typo into a one-line fix.
impl TryFrom<Vec<String>> for FeatureBundle {
    type Error = FeatureParseError;

    fn try_from(tokens: Vec<String>) -> Result<Self, Self::Error> {
        let mut bundle = Self::EMPTY;
        for token in &tokens {
            let (sign, name) = split_sign(token)?;
            let feature =
                Feature::from_name(name).ok_or_else(|| FeatureParseError::UnknownFeature {
                    token: token.clone(),
                    suggestion: nearest_feature_name(name),
                })?;
            if bundle.set(feature, sign).is_some() {
                return Err(FeatureParseError::DuplicateFeature {
                    name: feature.name(),
                });
            }
        }
        Ok(bundle)
    }
}

/// Splits `"+voice"` into `(Plus, "voice")`.
///
/// A bare name is rejected rather than assumed `+`. `DESIGN.md` §11.1 writes rule
/// targets as `[vowel, front]` with no signs, and silently reading those as `+`
/// would give the file two grammars — one of which round-trips differently.
fn split_sign(token: &str) -> Result<(Sign, &str), FeatureParseError> {
    match token.as_bytes().first() {
        // Safe to slice at 1: the arm already matched an ASCII first *byte*.
        Some(b'+') => Ok((Sign::Plus, &token[1..])),
        Some(b'-') => Ok((Sign::Minus, &token[1..])),
        _ if token.starts_with('\u{2212}') || token.starts_with('\u{00B1}') => {
            Err(FeatureParseError::NonAsciiSign {
                token: token.to_owned(),
            })
        }
        _ => Err(FeatureParseError::MissingSign {
            token: token.to_owned(),
        }),
    }
}

/// The closest feature name to a typo, for diagnostics.
///
/// A typo is the overwhelmingly likely cause of an unknown feature name, and "did
/// you mean `voice`?" is what makes a closed feature set a help rather than merely
/// a restriction (`docs/adr/0004`).
///
/// Delegates to [`stem_core::suggest`], which M2 moved this logic into so that
/// `stem_lexicon`'s concept keys — the workspace's second closed namespace — get
/// the identical behaviour rather than a second copy that drifts.
fn nearest_feature_name(name: &str) -> Option<&'static str> {
    stem_core::suggest::nearest(name, Feature::ALL.iter().map(|f| f.name()))
}

/// Emits in frozen [`Feature::ALL`] order, whatever order the file authored them
/// in. Parse is order-insensitive and emit is canonical, so load then save is a
/// value-stable fixpoint.
impl From<FeatureBundle> for Vec<String> {
    fn from(bundle: FeatureBundle) -> Self {
        bundle
            .iter()
            .map(|(feature, sign)| format!("{}{}", sign.as_char(), feature.name()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(tokens: &[&str]) -> FeatureBundle {
        FeatureBundle::try_from(tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid feature list")
    }

    /// These names are the on-disk vocabulary. Renaming one silently invalidates
    /// every fixture that uses it, so the whole sequence is pinned literally.
    #[test]
    fn feature_names_and_order_are_frozen() {
        let names: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).collect();
        assert_eq!(
            names,
            [
                "syllabic",
                "consonantal",
                "sonorant",
                "approximant",
                "continuant",
                "nasal",
                "lateral",
                "trill",
                "voice",
                "labial",
                "coronal",
                "dorsal",
                "high",
                "low",
                "back",
                "round",
            ]
        );
    }

    /// The guard against the silent-data-loss mode: a variant added to the enum
    /// but not to `ALL` would be dropped from every save.
    #[test]
    fn the_all_table_is_complete_and_positionally_aligned() {
        assert_eq!(Feature::ALL.len(), 16);
        assert_eq!(Feature::COUNT, 16);
        for (i, &feature) in Feature::ALL.iter().enumerate() {
            assert_eq!(feature as usize, i, "{} is out of position", feature.name());
        }
        let mut names: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Feature::COUNT, "duplicate feature name");
    }

    /// The bitset ceiling. Enforced at compile time by the `const` assertion above
    /// `Sign`; this restates it as a test so the reason is discoverable from the
    /// test list rather than only from a `const _: () =` line.
    #[test]
    fn the_feature_set_fits_the_bitset() {
        const { assert!(Feature::COUNT <= 64) };
    }

    #[test]
    fn an_unknown_feature_name_is_an_error_not_a_new_feature() {
        let err = FeatureBundle::try_from(vec!["+nonsense".to_owned()]).unwrap_err();
        assert!(matches!(err, FeatureParseError::UnknownFeature { .. }));
    }

    #[test]
    fn a_misspelled_feature_suggests_the_nearest_real_one() {
        let err = FeatureBundle::try_from(vec!["+voicee".to_owned()]).unwrap_err();
        match err {
            FeatureParseError::UnknownFeature { suggestion, .. } => {
                assert_eq!(suggestion, Some("voice"));
            }
            other => panic!("expected UnknownFeature, got {other:?}"),
        }
        assert!(err_text("+voicee").contains("did you mean `voice`?"));
    }

    fn err_text(token: &str) -> String {
        FeatureBundle::try_from(vec![token.to_owned()])
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_unicode_minus_sign_is_rejected_with_an_explanation() {
        let err = FeatureBundle::try_from(vec!["\u{2212}voice".to_owned()]).unwrap_err();
        assert!(matches!(err, FeatureParseError::NonAsciiSign { .. }));
        assert!(err.to_string().contains("ASCII"), "{err}");
    }

    #[test]
    fn a_bare_feature_name_is_rejected_rather_than_assumed_positive() {
        let err = FeatureBundle::try_from(vec!["voice".to_owned()]).unwrap_err();
        assert!(
            matches!(err, FeatureParseError::MissingSign { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn the_same_feature_twice_is_an_error() {
        let err =
            FeatureBundle::try_from(vec!["+voice".to_owned(), "-voice".to_owned()]).unwrap_err();
        assert!(
            matches!(err, FeatureParseError::DuplicateFeature { .. }),
            "{err:?}"
        );
    }

    /// The single most damaging error this model can make, asserted directly.
    #[test]
    fn an_absent_feature_is_not_a_minus_feature() {
        let b = bundle(&["+voice"]);
        assert_eq!(b.get(Feature::Voice), Some(Sign::Plus));
        assert_eq!(b.get(Feature::Round), None, "absent must not read as minus");
        assert!(!b.is(Feature::Round, Sign::Minus));
        assert!(!b.is_specified(Feature::Round));
    }

    #[test]
    fn bundles_render_in_frozen_feature_order_whatever_the_authored_order() {
        let authored = bundle(&["-voice", "+syllabic", "+dorsal"]);
        assert_eq!(authored.render(), "+syllabic -voice +dorsal");

        let other_order = bundle(&["+dorsal", "-voice", "+syllabic"]);
        assert_eq!(authored, other_order);
        assert_eq!(authored.render(), other_order.render());
    }

    #[test]
    fn setting_a_feature_preserves_the_bitset_invariant() {
        let mut b = FeatureBundle::EMPTY;
        for &feature in Feature::ALL {
            for sign in [Sign::Plus, Sign::Minus, Sign::Plus] {
                b.set(feature, sign);
                assert_eq!(b.pos & !b.spec, 0, "invariant broken on {}", feature.name());
                assert_eq!(b.get(feature), Some(sign));
            }
        }
        assert_eq!(b.len(), Feature::COUNT);
    }

    #[test]
    fn setting_a_feature_returns_the_previous_value() {
        let mut b = FeatureBundle::EMPTY;
        assert_eq!(b.set(Feature::Voice, Sign::Plus), None);
        assert_eq!(b.set(Feature::Voice, Sign::Minus), Some(Sign::Plus));
    }

    #[test]
    fn subsumption_ignores_features_the_pattern_does_not_value() {
        let segment = bundle(&["-sonorant", "-continuant", "-voice", "+labial"]);
        assert!(segment.subsumes(bundle(&["-sonorant", "-continuant", "-voice"])));
    }

    #[test]
    fn subsumption_fails_when_the_segment_does_not_value_a_pattern_feature() {
        let segment = bundle(&["-sonorant", "-continuant"]);
        assert!(
            !segment.subsumes(bundle(&["-voice"])),
            "an unvalued feature must not count as a match"
        );
    }

    #[test]
    fn subsumption_fails_on_a_disagreeing_sign() {
        let segment = bundle(&["+voice"]);
        assert!(!segment.subsumes(bundle(&["-voice"])));
    }

    #[test]
    fn an_empty_pattern_is_subsumed_by_everything() {
        assert!(bundle(&["+voice"]).subsumes(FeatureBundle::EMPTY));
        assert!(FeatureBundle::EMPTY.subsumes(FeatureBundle::EMPTY));
    }

    #[test]
    fn the_wire_form_round_trips_through_signed_names() {
        let original = bundle(&["+syllabic", "-consonantal", "+dorsal", "-back"]);
        let tokens: Vec<String> = original.into();
        assert_eq!(tokens, ["+syllabic", "-consonantal", "+dorsal", "-back"]);
        assert_eq!(FeatureBundle::try_from(tokens).unwrap(), original);
    }

    #[test]
    fn an_empty_bundle_renders_as_nothing() {
        assert_eq!(FeatureBundle::EMPTY.render(), "");
        assert!(FeatureBundle::EMPTY.is_empty());
        assert_eq!(FeatureBundle::EMPTY.len(), 0);
    }
}

#[cfg(test)]
mod m3_tests {
    use super::*;

    fn bundle(tokens: &[&str]) -> FeatureBundle {
        FeatureBundle::try_from(tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid feature list")
    }

    #[test]
    fn unset_removes_a_cell_rather_than_setting_it_minus() {
        let mut b = bundle(&["+voice", "-round"]);
        assert_eq!(b.unset(Feature::Round), Some(Sign::Minus));
        assert_eq!(b.get(Feature::Round), None, "absent, not minus");
        assert!(!b.is_specified(Feature::Round));
        // Removing a cell that was never there is a no-op.
        assert_eq!(b.unset(Feature::Round), None);
    }

    #[test]
    fn unset_preserves_the_bitset_invariant() {
        let mut b = bundle(&["+voice", "+labial", "+round"]);
        for feature in Feature::ALL {
            b.unset(*feature);
            assert_eq!(b.pos & !b.spec, 0, "invariant broken by {}", feature.name());
        }
        assert!(b.is_empty());
    }

    #[test]
    fn overlay_adds_and_overwrites_but_never_removes() {
        let base = bundle(&["-voice", "-round", "+labial"]);
        let after = base.overlay(bundle(&["+voice"]));
        assert_eq!(after.get(Feature::Voice), Some(Sign::Plus), "overwritten");
        assert_eq!(after.get(Feature::Round), Some(Sign::Minus), "untouched");
        assert_eq!(after.get(Feature::Labial), Some(Sign::Plus), "untouched");
    }

    /// The exact case the reference fixture produces: /n/ takes /p/'s place and
    /// must land byte-identically on /m/.
    #[test]
    fn copying_a_place_node_transfers_absence_as_well_as_values() {
        // /m/ is [+labial -coronal -dorsal -round]; /n/ is [-labial +coronal
        // -dorsal] with no rounding value at all.
        let n_place = bundle(&["-labial", "+coronal", "-dorsal"]);
        let p_place = bundle(&["+labial", "-coronal", "-dorsal", "-round"]);
        let t_place = bundle(&["-labial", "+coronal", "-dorsal"]);

        // n + p's place -> gains -round.
        let assimilated = n_place
            .copy_node(p_place, FeatureNode::Place)
            .expect("p has place");
        assert_eq!(assimilated.get(Feature::Labial), Some(Sign::Plus));
        assert_eq!(assimilated.get(Feature::Round), Some(Sign::Minus));

        // m + t's place -> LOSES its rounding value, because /t/ has none.
        let m_place = p_place;
        let back = m_place
            .copy_node(t_place, FeatureNode::Place)
            .expect("t has place");
        assert_eq!(back.get(Feature::Coronal), Some(Sign::Plus));
        assert_eq!(
            back.get(Feature::Round),
            None,
            "a copy must be able to remove a cell, or the result matches no phoneme"
        );
        assert_eq!(back, t_place, "the result is byte-identical to /t/'s place");
    }

    #[test]
    fn a_donor_with_no_place_does_not_bind() {
        let target = bundle(&["-labial", "+coronal", "-dorsal"]);
        let placeless = bundle(&["+voice"]);
        assert_eq!(
            target.copy_node(placeless, FeatureNode::Place),
            None,
            "a donor with no place has no place to give"
        );
    }

    /// The distinction that matters: an individual absence inside a valued node is
    /// copied; a wholly unvalued node refuses.
    #[test]
    fn a_donor_valuing_one_articulator_binds_even_if_dependents_are_absent() {
        let target = bundle(&["+labial", "-coronal", "-dorsal", "-round"]);
        let donor = bundle(&["-labial", "+coronal", "-dorsal"]);
        assert!(target.copy_node(donor, FeatureNode::Place).is_some());
    }

    #[test]
    fn the_place_node_covers_its_articulators_and_their_dependents() {
        let place = FeatureNode::Place.features();
        for required in [
            Feature::Labial,
            Feature::Coronal,
            Feature::Dorsal,
            Feature::High,
            Feature::Low,
            Feature::Back,
            Feature::Round,
        ] {
            assert!(place.contains(&required), "{} missing", required.name());
        }
        assert_eq!(FeatureNode::Laryngeal.features(), &[Feature::Voice]);
        assert_eq!(FeatureNode::Place.name(), "place");
    }

    #[test]
    fn a_feature_node_round_trips_in_snake_case() {
        let text = ron::ser::to_string(&FeatureNode::Place).expect("serialise");
        assert_eq!(text, "place");
        assert_eq!(
            ron::from_str::<FeatureNode>(&text).expect("deserialise"),
            FeatureNode::Place
        );
    }
}
