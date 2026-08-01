//! Rule validation that needs the language (`M3-SPEC` §9.3–§9.4).
//!
//! Free functions, not `Validate` impls: `Validate::validate(&self)` takes no
//! context, and two prior panels under-costed changing that signature. Same shape
//! as `phonotactics::check_against_inventory`.

use stem_core::{Issue, Severity, Validate, ValidationReport};
use stem_lexicon::Lexicon;
use stem_phonology::PhonemeInventory;
use stem_phonology::prosody::{Prosody, StressPolicy};

use crate::resolve::Resolver;
use crate::rule::{Change, EnvItem, SoundChangeRule};

/// The checks that need both the rules and the language they will run over.
pub fn check_against_language(
    rules: &[SoundChangeRule],
    inventory: &PhonemeInventory,
    prosody: &Prosody,
    lexicon: &Lexicon,
) -> ValidationReport {
    let mut report = ValidationReport::new();

    // Hand-authored marks are authoritative lexical stress (`prosody.rs`): under
    // an Unspecified policy the engine writes no marks, but it *reads* whatever
    // the file carries, so a hand-marked lexicon can fire a stress-conditioned
    // rule. Warning "can never fire" there would be the validator contradicting
    // the engine — the disagreement §9.3's other checks were single-sourced to
    // prevent.
    let any_hand_marks = lexicon.iter().any(|entry| {
        entry
            .phonemic_form
            .syllables
            .iter()
            .any(|s| s.stress.is_some())
    });

    for rule in rules {
        // Warning, not Error: a segment an earlier rule mints may match — the
        // acceptance fixture's lenition rule is exactly that case, targeting a
        // class that is empty in the proto-language. Erroring here would be the
        // validator disagreeing with the engine.
        if !rule.target.features.is_empty()
            && !inventory
                .iter()
                .any(|p| p.features.subsumes(rule.target.features))
        {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "target_matches_nothing",
                    "no phoneme of this inventory matches the target; the rule can only \
                     ever apply to a segment an earlier rule creates",
                )
                .about(&rule.id),
            );
        }

        // Same reasoning, weaker, per environment slot.
        for item in rule
            .environment
            .before
            .iter()
            .chain(rule.environment.after.iter())
        {
            if let EnvItem::Segment(pattern) = item
                && !pattern.features.is_empty()
                && !inventory
                    .iter()
                    .any(|p| p.features.subsumes(pattern.features))
            {
                report.push(
                    Issue::new(
                        Severity::Note,
                        "environment_matches_nothing",
                        "no phoneme of this inventory matches an environment slot; the \
                         condition can only be met by a segment an earlier rule creates",
                    )
                    .about(&rule.id),
                );
            }
        }

        // The dishonesty guard: a stress-conditioned rule over a language with no
        // stress system can never fire, and saying nothing would let the file
        // claim the stronger rule while implementing neither.
        let mentions_stress = rule.target.stress.is_some()
            || rule
                .environment
                .before
                .iter()
                .chain(rule.environment.after.iter())
                .any(|i| matches!(i, EnvItem::Segment(p) if p.stress.is_some()));
        if mentions_stress && prosody.stress == StressPolicy::Unspecified && !any_hand_marks {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "stress_without_prosody",
                    format!(
                        "rule `{}` (\"{}\") requires a stress mark, but this language \
                         declares no stress policy and no syllable of its lexicon is \
                         hand-marked, so every syllable is unmarked and the rule can never \
                         fire; declare a policy, mark syllables by hand, or drop the \
                         condition and rename the rule to say what it does",
                        rule.id, rule.name
                    ),
                )
                .about(&rule.id),
            );
        }
    }

    // Static pre-flight for `Set` rules: for each inventory phoneme the target
    // matches, compute the output and resolve it — **with the engine's own
    // resolution function**, so the two cannot disagree. Cheap at ≤40 segments,
    // and it tells the user before they run.
    for rule in rules {
        if let Change::Set(delta) = &rule.change {
            let mut probe = Resolver::new(inventory);
            for phoneme in inventory.iter() {
                if !phoneme.features.subsumes(rule.target.features) {
                    continue;
                }
                let out = phoneme.features.overlay(*delta);
                if probe.resolve(out, phoneme.frequency_weight).is_err() {
                    report.push(
                        Issue::new(
                            Severity::Note,
                            "unnameable_output_class",
                            format!(
                                "applying rule `{}` to /{}/ produces a segment neither \
                                 the inventory nor the reference table can name; those \
                                 sites will be refused at run time",
                                rule.id, phoneme.ipa
                            ),
                        )
                        .about(&rule.id),
                    );
                }
            }
        }
    }

    // The timeline label must not silently contradict the derivation order.
    for window in rules.windows(2) {
        if window[1].chronology_years < window[0].chronology_years {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "chronology_not_monotonic",
                    format!(
                        "rule `{}` is dated {} years but follows `{}` at {}; application \
                         order is list order, and the M5 timeline would contradict the \
                         derivation",
                        window[1].id,
                        window[1].chronology_years,
                        window[0].id,
                        window[0].chronology_years
                    ),
                )
                .about(&window[1].id),
            );
        }
    }

    report
}

/// The checks that apply to a genome's `applied_rules` — everything the per-rule
/// and against-language checks do, **minus duplicate-id**. A log is not a set:
/// real histories apply intervocalic voicing twice, in different strata.
pub fn check_applied_log(
    applied: &[SoundChangeRule],
    inventory: &PhonemeInventory,
    prosody: &Prosody,
    lexicon: &Lexicon,
) -> ValidationReport {
    let mut report = ValidationReport::new();
    for rule in applied {
        for issue in rule.validate().issues {
            report.push(issue);
        }
    }
    for issue in check_against_language(applied, inventory, prosody, lexicon).issues {
        // `target_matches_nothing` is computed against the *current* inventory,
        // which for an applied log includes every mint — so it stays meaningful.
        report.push(issue);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{Environment, SegmentPattern};
    use stem_core::RuleId;
    use stem_phonology::prosody::{Stress, WordEdge};
    use stem_phonology::{FeatureBundle, Phoneme, SegmentKind};

    fn bundle(tokens: &[&str]) -> FeatureBundle {
        FeatureBundle::try_from(tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid feature list")
    }

    fn nasal_only_inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_n", "n", SegmentKind::Consonant).with_features(bundle(&[
                "-syllabic",
                "+consonantal",
                "+sonorant",
                "-approximant",
                "-continuant",
                "+nasal",
                "-lateral",
                "-trill",
                "+voice",
                "-labial",
                "+coronal",
                "-dorsal",
            ])),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel).with_features(bundle(&[
                "+syllabic",
                "-consonantal",
                "+sonorant",
                "+approximant",
                "+continuant",
                "-nasal",
                "-lateral",
                "-trill",
                "+voice",
                "-labial",
                "-coronal",
                "+dorsal",
                "-high",
                "+low",
                "+back",
                "-round",
            ])),
        ])
    }

    fn stress_rule() -> SoundChangeRule {
        SoundChangeRule {
            id: RuleId::new("r_s"),
            name: "Final unstressed vowel loss".to_owned(),
            description: String::new(),
            chronology_years: 0,
            target: SegmentPattern {
                features: bundle(&["+syllabic"]),
                stress: Some(Stress::Unstressed),
            },
            environment: Environment {
                before: vec![],
                after: vec![EnvItem::Boundary],
            },
            change: Change::Delete,
        }
    }

    /// A hand-marked lexicon under an Unspecified policy: the engine reads the
    /// marks and the rule fires, so the "can never fire" warning must not.
    fn hand_marked_lexicon() -> Lexicon {
        Lexicon::from_entries([stem_lexicon::WordEntry {
            id: stem_core::WordId::new("w_0001"),
            concept: Some(stem_lexicon::ConceptKey::new("STAR")),
            phonemic_form: stem_phonology::Root {
                syllables: vec![stem_phonology::Syllable {
                    pattern: "V".to_owned(),
                    segments: vec![stem_core::PhonemeId::new("ph_a")],
                    stress: Some(Stress::Primary),
                }],
            },
            glosses: vec!["star".to_owned()],
            part_of_speech: stem_lexicon::PartOfSpeech::Noun,
            cognate_set: stem_core::CognateSetId::new("cog_x_0001"),
            source: stem_lexicon::WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
        }])
    }

    #[test]
    fn a_stress_conditioned_rule_over_a_language_with_no_prosody_warns() {
        let report = check_against_language(
            &[stress_rule()],
            &nasal_only_inventory(),
            &Prosody::new(),
            &Lexicon::new(),
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "stress_without_prosody"),
            "{report}"
        );
    }

    #[test]
    fn the_same_rule_under_a_declared_policy_does_not_warn() {
        let prosody = Prosody::fixed(WordEdge::Left, 0);
        let report = check_against_language(
            &[stress_rule()],
            &nasal_only_inventory(),
            &prosody,
            &Lexicon::new(),
        );
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "stress_without_prosody"),
            "{report}"
        );
    }

    #[test]
    fn a_hand_marked_lexicon_makes_the_rule_fireable_so_no_warning() {
        let report = check_against_language(
            &[stress_rule()],
            &nasal_only_inventory(),
            &Prosody::new(),
            &hand_marked_lexicon(),
        );
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "stress_without_prosody"),
            "the engine reads hand-authored marks, so the validator must not claim \
             the rule can never fire: {report}"
        );
    }

    /// The fixture's own shape: lenition targets a class empty in the
    /// proto-language, because its only input is a segment voicing mints. That is
    /// a Warning, never an Error — erroring would be the validator disagreeing
    /// with the engine.
    #[test]
    fn a_target_empty_in_the_inventory_warns_rather_than_erroring() {
        let mut rule = stress_rule();
        rule.target = SegmentPattern {
            features: bundle(&["-sonorant", "-continuant", "+voice", "+dorsal"]),
            stress: None,
        };
        rule.environment = Environment::default();
        let report = check_against_language(
            &[rule],
            &nasal_only_inventory(),
            &Prosody::new(),
            &Lexicon::new(),
        );
        assert!(report.is_ok(), "{report}");
        assert!(
            report
                .warnings()
                .any(|i| i.code == "target_matches_nothing"),
            "{report}"
        );
    }

    #[test]
    fn a_backwards_chronology_warns() {
        let mut first = stress_rule();
        first.id = RuleId::new("r_1");
        first.chronology_years = 500;
        let mut second = stress_rule();
        second.id = RuleId::new("r_2");
        second.chronology_years = 100;

        let prosody = Prosody::fixed(WordEdge::Left, 0);
        let report = check_against_language(
            &[first, second],
            &nasal_only_inventory(),
            &prosody,
            &Lexicon::new(),
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "chronology_not_monotonic"),
            "{report}"
        );
    }

    #[test]
    fn an_applied_log_tolerates_a_repeated_rule_id() {
        let prosody = Prosody::fixed(WordEdge::Left, 0);
        let report = check_applied_log(
            &[stress_rule(), stress_rule()],
            &nasal_only_inventory(),
            &prosody,
            &Lexicon::new(),
        );
        assert!(
            !report.issues.iter().any(|i| i.code == "duplicate_id"),
            "a log is not a set; real histories repeat changes: {report}"
        );
    }
}
