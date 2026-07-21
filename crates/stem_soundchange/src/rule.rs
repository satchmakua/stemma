//! Sound-change rules as data (`DESIGN.md` §8.4, §11).
//!
//! Structs, no DSL — §20.4 and ROADMAP both defer the parser to M10, so that the
//! syntax is designed against semantics that already work. These types are the
//! compile target that DSL will lower to, which is why their stored shape is
//! settled with care now.

use serde::{Deserialize, Serialize};
use stem_core::{Issue, RuleId, Severity, Validate, ValidationReport};
use stem_phonology::{Feature, FeatureBundle, FeatureNode, Sign, Stress};

/// What a segment must be for a rule to touch it.
///
/// Two independent predicates over two independent stores: `features` reads the
/// phoneme's own bundle, `stress` reads the **enclosing syllable's** mark. They
/// are never fused into one bundle, because no inventory phoneme carries stress
/// and a fused view would make every resolution fail — the exact defect two prior
/// designs in this project produced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentPattern {
    /// A conjunction of positive requirements over valued cells, tested with
    /// `FeatureBundle::subsumes`. An unvalued cell never satisfies a pattern
    /// (`docs/adr/0004`): there is deliberately no way to ask for "undefined
    /// here".
    #[serde(default)]
    pub features: FeatureBundle,
    /// `None` is "don't care". `Some(Unstressed)` matches only a syllable marked
    /// `Unstressed`, never an unmarked one — absence is not minus, one tier up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<Stress>,
}

impl SegmentPattern {
    /// Whether this pattern constrains nothing, and so matches every segment.
    ///
    /// `FeatureBundle::EMPTY` is subsumed by everything, so a rule that reached
    /// this state by accident is a silent global rule — `rules.empty_target`
    /// exists for it. Gated on the whole pattern, so a stress-only condition is
    /// legitimately non-vacuous.
    pub fn is_vacuous(&self) -> bool {
        self.features.is_empty() && self.stress.is_none()
    }
}

/// One slot of an environment window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvItem {
    /// The word edge, §11.1's `#`. Legal only as the **outermost** item of its
    /// side; anywhere else it is unsatisfiable (`rules.boundary_not_outermost`).
    /// Note that `[Segment(V), Boundary]` in `before` is `# V _` — a perfectly
    /// ordinary "after a word-initial vowel" environment — and is legal.
    Boundary,
    /// A segment matching this pattern.
    Segment(SegmentPattern),
}

/// The context a rule requires, as two adjacency windows.
///
/// **Both lists are written outward from the target**: `before[0]` and `after[0]`
/// are the segments immediately left and right. Symmetric, so
/// [`Position::Before`]`(n)` and [`Position::After`]`(n)` index their list
/// directly and the two can never drift. (§11.1 writes `V _ V` in surface order;
/// the RON writes it outward, and for the one-slot windows M3 ships the two are
/// the same text. A test pins the two-slot case, because outward is exactly the
/// thing a reader assumes the other way round.)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    /// The left context, outward from the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<EnvItem>,
    /// The right context, outward from the target.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<EnvItem>,
}

/// A position a change may name, as a **fixed offset from the target**.
///
/// Offsets rather than named captures, and this is the load-bearing choice: an
/// environment is a fixed-length adjacency window, so every position a change
/// names resolves to a concrete snapshot index the moment the target index is
/// chosen — *before* any matching begins. There is nothing to bind, so there is
/// no binding order to specify and no not-yet-filled slice to index into. A prior
/// design evaluated a target-side reference to `after(0)` before the environment
/// scan had populated it; here that state is unrepresentable.
///
/// The cost is no unbounded search — no vowel harmony, no Grassmann's law. That
/// is deferred openly (M3-SPEC §12). `Position` being an enum is what makes
/// `Nearest { matching }` additive later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    /// `before[n]` — the segment `n + 1` to the left of the target.
    Before(u8),
    /// `after[n]` — the segment `n + 1` to the right of the target.
    After(u8),
}

/// What happens to a matched segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    /// Overwrite literal cells on the target. §11.1's `voice = true`. Never
    /// removes a cell; never "repairs" the result. A change does exactly what it
    /// says, and an ill-formed output is caught loudly rather than silently
    /// patched — a prior design's repair pass deleted a just-written `+round` off
    /// a `-labial` segment with no diagnostic.
    Set(FeatureBundle),
    /// Copy a whole feature node from another position, **including absence**.
    ///
    /// §7.2's "nasals assimilate to the place of a following stop" is
    /// `place := place of after(0)`, **not** `place = labial`. Every example in
    /// §11.1 sets a literal, so a model designed from §11.1 alone gets this
    /// wrong; M1's research called it the requirement most likely to be missed
    /// and most expensive to retrofit. The ROADMAP names it.
    Copy {
        /// Where the features come from.
        from: Position,
        /// Which closed node moves.
        node: FeatureNode,
    },
    /// Remove the target segment. §7.2's syncope and apocope.
    Delete,
}

/// One ordered, traceable sound change (`DESIGN.md` §8.4).
///
/// Six of §8.4's nine fields, plus `chronology` reshaped to the one scalar
/// §10.4's timeline reads. `exceptions` and `probability` are deferred with their
/// extension shapes named in `M3-SPEC` §12; each arrives as a `#[serde(default)]`
/// field whose empty value is what every M3 rule already means, so no stored rule
/// changes meaning when they land. (`probability`, when it ships, ships as
/// `probability_permille: u32` — never `f32`, per `CLAUDE.md`'s integer rule.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundChangeRule {
    /// Stable identity, referenced by every trace.
    pub id: RuleId,
    /// Required. An unnamed rule in a derivation printout is unreadable.
    pub name: String,
    /// Optional prose.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Simulated years at which this change occurred — the one field of §8.5's
    /// `HistoricalEvent` that §10.4's timeline actually needs, on the rule so the
    /// timeline is renderable from `applied_rules` alone at M5.
    ///
    /// **A label, never a sort key.** Application order is `applied_rules` index,
    /// full stop; `rules.chronology_not_monotonic` warns when the two disagree so
    /// the timeline cannot silently contradict the derivation.
    #[serde(default)]
    pub chronology_years: i32,
    /// What the rule touches.
    pub target: SegmentPattern,
    /// Where it touches it.
    #[serde(default)]
    pub environment: Environment,
    /// What it does there.
    pub change: Change,
}

/// An ordered rule sequence, as its own file (§16.2's `rules_simple.ron`).
///
/// A `RuleSet` is a **set**: ids are unique within it and `duplicate_id` is an
/// Error. A genome's `applied_rules` is a **log**: ids repeat by nature, because
/// real sound histories repeat changes. The two deliberately do not share a
/// validator — see [`crate::check::check_applied_log`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    /// Stable identity for the file.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional prose.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// **Ordered.** Never a set, never a map, never re-sorted (§11.3). Order is
    /// chronology, and it is the whole point of the milestone.
    pub rules: Vec<SoundChangeRule>,
}

impl SoundChangeRule {
    /// The environment window an item of `position` indexes, if the position is
    /// inside the declared window.
    pub(crate) fn env_item_at(&self, position: Position) -> Option<&EnvItem> {
        match position {
            Position::Before(n) => self.environment.before.get(usize::from(n)),
            Position::After(n) => self.environment.after.get(usize::from(n)),
        }
    }
}

impl Validate for SoundChangeRule {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.id.is_empty() {
            report.error("empty_id", "the rule has an empty id");
        }
        if self.name.trim().is_empty() {
            report.push(
                Issue::new(
                    Severity::Error,
                    "empty_name",
                    "the rule has no name; an unnamed rule in a derivation printout is unreadable",
                )
                .about(&self.id),
            );
        }

        // `FeatureBundle::EMPTY` is subsumed by everything, so an accidentally
        // empty target is a silent global rule. A stress-only condition is a
        // legitimate "any segment of an unstressed syllable" and passes.
        if self.target.is_vacuous() {
            report.push(
                Issue::new(
                    Severity::Error,
                    "empty_target",
                    "the target constrains nothing, so this rule would touch every segment \
                     of every word; state at least one feature or a stress condition",
                )
                .about(&self.id),
            );
        }

        if let Change::Set(delta) = &self.change {
            if delta.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "empty_change",
                        "the change sets nothing; this rule is a no-op",
                    )
                    .about(&self.id),
                );
            }
            // A partial place write strands the dependents the required-feature
            // tables will demand. Heuristic and Warning-only: the target may
            // already carry them.
            let writes_labial = delta.is(Feature::Labial, Sign::Plus);
            let writes_dorsal = delta.is(Feature::Dorsal, Sign::Plus);
            if (writes_labial && !delta.is_specified(Feature::Round))
                || (writes_dorsal
                    && !(delta.is_specified(Feature::High)
                        && delta.is_specified(Feature::Low)
                        && delta.is_specified(Feature::Back)
                        && delta.is_specified(Feature::Round)))
            {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "place_write_may_be_incomplete",
                        "this change writes an articulator without all of its dependents; \
                         segments that do not already value them will be refused as \
                         ill-formed (consider `copy` with a place node instead)",
                    )
                    .about(&self.id),
                );
            }
        }

        // A boundary anywhere but the outermost slot of its side is unsatisfiable:
        // nothing sits beyond the edge of a word.
        for (side, items) in [
            ("before", &self.environment.before),
            ("after", &self.environment.after),
        ] {
            for (i, item) in items.iter().enumerate() {
                if matches!(item, EnvItem::Boundary) && i + 1 != items.len() {
                    report.push(
                        Issue::new(
                            Severity::Error,
                            "boundary_not_outermost",
                            format!(
                                "`{side}[{i}]` is a word boundary with another slot beyond \
                                 it; nothing sits past the edge of a word"
                            ),
                        )
                        .about(&self.id),
                    );
                }
            }
        }

        if let Change::Copy { from, .. } = &self.change {
            match self.env_item_at(*from) {
                None => {
                    report.push(
                        Issue::new(
                            Severity::Error,
                            "change_references_unmatched_position",
                            format!(
                                "the change copies from {from:?}, but the environment \
                                 declares no slot there — a copy may only read a position \
                                 the environment matched"
                            ),
                        )
                        .about(&self.id),
                    );
                }
                Some(EnvItem::Boundary) => {
                    report.push(
                        Issue::new(
                            Severity::Error,
                            "copy_from_boundary",
                            "the change copies from a word boundary, which has no features \
                             to donate",
                        )
                        .about(&self.id),
                    );
                }
                Some(EnvItem::Segment(_)) => {}
            }
        }

        report
    }
}

impl Validate for RuleSet {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.rules.is_empty() {
            report.note("empty", "the rule set declares no rules");
        }

        for rule in &self.rules {
            for issue in rule.validate().issues {
                report.push(issue);
            }
        }

        // A `RuleSet` is a set. (An applied log is not — see `check_applied_log`.)
        let mut seen: Vec<&str> = Vec::new();
        for rule in &self.rules {
            if seen.contains(&rule.id.as_str()) {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "duplicate_id",
                        "two rules in one set share this id; a trace could not name which \
                         applied",
                    )
                    .about(&rule.id),
                );
            }
            seen.push(rule.id.as_str());
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(tokens: &[&str]) -> FeatureBundle {
        FeatureBundle::try_from(tokens.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
            .expect("valid feature list")
    }

    fn voicing() -> SoundChangeRule {
        SoundChangeRule {
            id: RuleId::new("r_0001"),
            name: "Intervocalic voicing".to_owned(),
            description: String::new(),
            chronology_years: 250,
            target: SegmentPattern {
                features: bundle(&["-sonorant", "-continuant", "-voice"]),
                stress: None,
            },
            environment: Environment {
                before: vec![EnvItem::Segment(SegmentPattern {
                    features: bundle(&["+syllabic"]),
                    stress: None,
                })],
                after: vec![EnvItem::Segment(SegmentPattern {
                    features: bundle(&["+syllabic"]),
                    stress: None,
                })],
            },
            change: Change::Set(bundle(&["+voice"])),
        }
    }

    #[test]
    fn a_well_formed_rule_validates_cleanly() {
        let report = voicing().validate();
        assert!(report.is_ok(), "{report}");
    }

    #[test]
    fn an_empty_target_is_an_error() {
        let mut rule = voicing();
        rule.target = SegmentPattern::default();
        let report = rule.validate();
        assert!(
            report.errors().any(|i| i.code == "empty_target"),
            "{report}"
        );
    }

    /// A stress-only target is a legitimate "any segment of an unstressed
    /// syllable" and must not trip `empty_target`.
    #[test]
    fn a_rule_targeting_only_a_stress_condition_is_not_an_empty_target() {
        let mut rule = voicing();
        rule.target = SegmentPattern {
            features: FeatureBundle::EMPTY,
            stress: Some(Stress::Unstressed),
        };
        let report = rule.validate();
        assert!(
            !report.issues.iter().any(|i| i.code == "empty_target"),
            "{report}"
        );
    }

    #[test]
    fn a_boundary_with_a_slot_beyond_it_is_an_error() {
        let mut rule = voicing();
        rule.environment.before = vec![
            EnvItem::Boundary,
            EnvItem::Segment(SegmentPattern {
                features: bundle(&["+syllabic"]),
                stress: None,
            }),
        ];
        let report = rule.validate();
        assert!(
            report.errors().any(|i| i.code == "boundary_not_outermost"),
            "{report}"
        );
    }

    /// `[Segment(V), Boundary]` in `before` is `# V _` — after a word-initial
    /// vowel — and is legal.
    #[test]
    fn a_boundary_at_the_outer_end_of_a_window_is_legal() {
        let mut rule = voicing();
        rule.environment.before = vec![
            EnvItem::Segment(SegmentPattern {
                features: bundle(&["+syllabic"]),
                stress: None,
            }),
            EnvItem::Boundary,
        ];
        let report = rule.validate();
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "boundary_not_outermost"),
            "{report}"
        );
    }

    #[test]
    fn a_copy_from_an_undeclared_position_is_an_error() {
        let mut rule = voicing();
        rule.change = Change::Copy {
            from: Position::After(1),
            node: FeatureNode::Place,
        };
        let report = rule.validate();
        assert!(
            report
                .errors()
                .any(|i| i.code == "change_references_unmatched_position"),
            "{report}"
        );
    }

    #[test]
    fn a_copy_from_a_boundary_is_an_error() {
        let mut rule = voicing();
        rule.environment.after = vec![EnvItem::Boundary];
        rule.change = Change::Copy {
            from: Position::After(0),
            node: FeatureNode::Place,
        };
        let report = rule.validate();
        assert!(
            report.errors().any(|i| i.code == "copy_from_boundary"),
            "{report}"
        );
    }

    #[test]
    fn an_applied_log_may_repeat_a_rule_id_but_a_rule_set_may_not() {
        let set = RuleSet {
            id: "rs".to_owned(),
            name: "Twice".to_owned(),
            description: String::new(),
            rules: vec![voicing(), voicing()],
        };
        let report = set.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_id"),
            "{report}"
        );

        // The log-side check deliberately lacks that error; asserted over in
        // `check.rs`'s tests where `check_applied_log` lives.
    }

    #[test]
    fn a_misspelled_rule_field_fails_to_load_rather_than_defaulting() {
        // A silently-defaulted `enviroment:` would turn a conditioned change into
        // a global one — the worst possible misread of a rule file.
        let text = r#"(
            id: "r_x",
            name: "Bad",
            target: (features: ["+nasal"]),
            enviroment: (after: [boundary]),
            change: delete,
        )"#;
        assert!(ron::from_str::<SoundChangeRule>(text).is_err());
    }

    #[test]
    fn a_rule_round_trips_through_ron() {
        let rule = voicing();
        let text = ron::ser::to_string(&rule).expect("serialise");
        assert_eq!(
            ron::from_str::<SoundChangeRule>(&text).expect("parse"),
            rule
        );
    }
}
