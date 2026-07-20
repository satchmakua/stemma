//! Prosody: stress, and the rule that assigns it.
//!
//! **Stress is syllable-scoped, not segment-scoped**, and it never enters a
//! [`crate::FeatureBundle`]. No inventory phoneme carries stress, so a fused
//! segment view would fail to resolve for *every* rule — the exact failure two
//! prior designs in this project produced. This honours the standing commitment in
//! this crate's own `lib.rs` and in [`crate::generate`]'s note that a root
//! "already knows its own syllable structure, which is where stress will hang".

use serde::{Deserialize, Serialize};

use crate::generate::Root;

/// A syllable's prominence.
///
/// Two variants, not three. `Secondary` has no producer at M3, and a variant with
/// no producer is scaffolding. Absence of a mark — `None` on
/// [`crate::Syllable::stress`] — is the third state, and it is genuinely distinct:
/// "no prosodic analysis has been performed" is not "unstressed".
///
/// That is `docs/adr/0004`'s absent-is-not-minus rule one tier up, and it is the
/// single mechanism that stops "final **unstressed** vowel loss" from silently
/// degrading into unconditioned final-vowel loss on a language with no prosody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stress {
    /// Carries no prominence.
    Unstressed,
    /// Carries the word's main prominence.
    Primary,
}

impl Stress {
    /// The `snake_case` name, for diagnostics and exports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unstressed => "unstressed",
            Self::Primary => "primary",
        }
    }
}

impl std::fmt::Display for Stress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Which edge a fixed stress rule counts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordEdge {
    /// Count in from the start of the word.
    Left,
    /// Count in from the end of the word.
    Right,
}

/// How this language places stress.
///
/// [`StressPolicy::Fixed`] covers initial (`Left, 0`), final (`Right, 0`),
/// penultimate (`Right, 1`) and antepenultimate (`Right, 2`). Quantity-insensitive
/// fixed-position stress is one of the commonest systems in the world's languages
/// — this is a real typological class, not a stand-in.
///
/// Weight-sensitive stress is deferred: it needs a syllable-weight predicate, which
/// needs a coda/nucleus distinction, which needs the resyllabifier M3 does not
/// ship. Adding the variant later is purely additive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StressPolicy {
    /// This language declares no stress system.
    ///
    /// Assignment is a no-op, every syllable stays unmarked, and every
    /// stress-conditioned rule declines to match — which
    /// `rules.stress_without_prosody` reports rather than leaving silent. The
    /// default, because it is what every pre-M3 file means.
    #[default]
    Unspecified,
    /// Primary stress `offset` syllables in from `from`; every other syllable
    /// unstressed.
    ///
    /// An offset past the end of the word falls on the nearest existing syllable,
    /// which is what real fixed-position systems do.
    Fixed {
        /// The edge to count from.
        from: WordEdge,
        /// How many syllables in from that edge.
        offset: u8,
    },
}

/// The prosodic system of one language.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prosody {
    /// Where stress falls.
    #[serde(default)]
    pub stress: StressPolicy,
}

impl Prosody {
    /// A language that declares no stress system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a fixed-position stress system.
    pub fn fixed(from: WordEdge, offset: u8) -> Self {
        Self {
            stress: StressPolicy::Fixed { from, offset },
        }
    }

    /// Whether this is the default — no stress system — for `skip_serializing_if`.
    pub fn is_unspecified(&self) -> bool {
        *self == Self::default()
    }

    /// A one-line summary for CLI output.
    pub fn summary(&self) -> String {
        match self.stress {
            StressPolicy::Unspecified => "no stress system declared".to_owned(),
            StressPolicy::Fixed { from, offset } => {
                let edge = match from {
                    WordEdge::Left => "initial",
                    WordEdge::Right => "final",
                };
                match (from, offset) {
                    (_, 0) => format!("fixed stress, {edge}"),
                    (WordEdge::Right, 1) => "fixed stress, penultimate".to_owned(),
                    (WordEdge::Right, 2) => "fixed stress, antepenultimate".to_owned(),
                    _ => format!("fixed stress, {offset} in from the {edge:?} edge"),
                }
            }
        }
    }

    /// Marks every syllable of `root`, returning whether anything was written.
    ///
    /// **All-or-nothing per word: a root that already carries any stress mark is
    /// left entirely alone.** Three things follow, and all three are deliberate.
    ///
    /// It is *idempotent*, so splitting one rule sequence across two `apply-rules`
    /// invocations gives the same language as running it once — without this, the
    /// second run would re-derive stress over a syllable count that apocope had
    /// already changed, silently implementing the automatic stress shift M3
    /// explicitly does not model. That would be a §9.4 violation.
    ///
    /// It makes a hand-authored stress mark authoritative, so lexical stress is
    /// available with no extra field.
    ///
    /// And it means stress is assigned *once in a word's life*, not once per run.
    /// After apocope drops a syllable the survivors keep the marks they had; the
    /// word is not re-analysed. That is a stated modelling limit, not a bug.
    ///
    /// Pure, total, and **RNG-free** — it consumes no stream and moves no digest.
    pub fn assign(&self, root: &mut Root) -> bool {
        let StressPolicy::Fixed { from, offset } = self.stress else {
            return false;
        };
        if root.syllables.is_empty() {
            return false;
        }
        if root.syllables.iter().any(|s| s.stress.is_some()) {
            return false;
        }

        let last = root.syllables.len() - 1;
        // An offset past the end lands on the nearest existing syllable, which is
        // what real fixed-position systems do.
        let target = match from {
            WordEdge::Left => usize::from(offset).min(last),
            WordEdge::Right => last.saturating_sub(usize::from(offset)),
        };

        for (i, syllable) in root.syllables.iter_mut().enumerate() {
            syllable.stress = Some(if i == target {
                Stress::Primary
            } else {
                Stress::Unstressed
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Syllable;
    use stem_core::PhonemeId;

    fn root(syllable_count: usize) -> Root {
        Root {
            syllables: (0..syllable_count)
                .map(|i| Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![PhonemeId::new(format!("ph_{i}"))],
                    stress: None,
                })
                .collect(),
        }
    }

    fn marks(root: &Root) -> Vec<Option<Stress>> {
        root.syllables.iter().map(|s| s.stress).collect()
    }

    #[test]
    fn an_unspecified_policy_writes_nothing() {
        let mut r = root(3);
        assert!(!Prosody::new().assign(&mut r));
        assert_eq!(marks(&r), [None, None, None], "unmarked is not unstressed");
    }

    #[test]
    fn initial_stress_falls_on_the_first_syllable() {
        let mut r = root(3);
        assert!(Prosody::fixed(WordEdge::Left, 0).assign(&mut r));
        assert_eq!(
            marks(&r),
            [
                Some(Stress::Primary),
                Some(Stress::Unstressed),
                Some(Stress::Unstressed)
            ]
        );
    }

    #[test]
    fn penultimate_stress_counts_in_from_the_right() {
        let mut r = root(3);
        assert!(Prosody::fixed(WordEdge::Right, 1).assign(&mut r));
        assert_eq!(
            marks(&r),
            [
                Some(Stress::Unstressed),
                Some(Stress::Primary),
                Some(Stress::Unstressed)
            ]
        );
    }

    /// The case that makes "final unstressed vowel loss" different from "final
    /// vowel loss": a monosyllable's only vowel is *stressed*, so it survives.
    #[test]
    fn a_monosyllable_takes_primary_stress_whatever_the_offset() {
        for policy in [
            Prosody::fixed(WordEdge::Left, 0),
            Prosody::fixed(WordEdge::Right, 0),
            Prosody::fixed(WordEdge::Left, 5),
            Prosody::fixed(WordEdge::Right, 5),
        ] {
            let mut r = root(1);
            assert!(policy.assign(&mut r));
            assert_eq!(marks(&r), [Some(Stress::Primary)], "{policy:?}");
        }
    }

    /// Idempotence is what makes splitting a rule sequence across two runs produce
    /// the same language as running it once.
    #[test]
    fn assignment_is_all_or_nothing_so_a_second_pass_changes_nothing() {
        let policy = Prosody::fixed(WordEdge::Left, 0);
        let mut r = root(3);
        assert!(policy.assign(&mut r));
        let after_first = marks(&r);

        // Drop a syllable, as apocope would.
        r.syllables.pop();
        assert!(
            !policy.assign(&mut r),
            "a word that already carries marks must be left alone"
        );
        assert_eq!(
            marks(&r),
            after_first[..2],
            "survivors keep the marks they had; the word is not re-analysed"
        );
    }

    /// A hand-authored mark is authoritative, which is how lexical stress works
    /// without a second field.
    #[test]
    fn a_hand_authored_mark_suppresses_assignment_entirely() {
        let mut r = root(3);
        r.syllables[2].stress = Some(Stress::Primary);
        assert!(!Prosody::fixed(WordEdge::Left, 0).assign(&mut r));
        assert_eq!(marks(&r), [None, None, Some(Stress::Primary)]);
    }

    #[test]
    fn an_empty_root_is_left_alone() {
        let mut r = Root { syllables: vec![] };
        assert!(!Prosody::fixed(WordEdge::Left, 0).assign(&mut r));
    }

    #[test]
    fn a_prosody_round_trips_through_ron() {
        let original = Prosody::fixed(WordEdge::Right, 1);
        let text = ron::ser::to_string(&original).expect("serialise");
        assert_eq!(
            ron::from_str::<Prosody>(&text).expect("deserialise"),
            original
        );
        assert!(Prosody::new().is_unspecified());
    }
}
