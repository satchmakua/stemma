//! Why a language has the vocabulary it has (ROADMAP M15, `DESIGN.md` §7.5, §18.1).
//!
//! # The problem this closes
//!
//! M13 shipped 673 concepts to every language by default, on the argument that a
//! language missing words it should have is a worse failure than one carrying a word
//! its speakers rarely use — because the first failure is *invisible*. That was the
//! right call and it left a debt: the desert language now has a word for `ice`
//! because the wordlist does, which is the tool making a claim about its speakers
//! rather than reporting one.
//!
//! This is the honest form. A gap becomes a **stated fact about the speakers, with a
//! reason on the record**, rather than an accident of which wordlist shipped. And it
//! runs the other way too: a culture that cares about something gets *several* words
//! for it, which is the half of lexical ecology that a wordlist can never express at
//! all.
//!
//! # No inference engine, deliberately
//!
//! A tempting design is a compiled table mapping ecologies to vocabulary — "desert ⇒
//! no SEA, no BOAT, no FISH" — so an author picks `terrain: desert` and the gaps
//! appear. That table would be Stemma asserting a great many claims about human
//! cultures that it cannot support, in a program whose premise is that every claim
//! is traceable to something a person wrote. There are desert peoples with elaborate
//! maritime vocabulary (they trade), and there are inland peoples with none.
//!
//! So a [`CultureTrait`] carries **its own declared consequences**. The author writes
//! "these are high-desert herders, and *therefore* they lack these five meanings and
//! elaborate these two", and the engine applies it and reports it. That is the same
//! relationship a `RuleSet` has to the sound-change engine: authored data, applied by
//! code, with the reasoning visible in the file rather than compiled into the tool.
//! What a trait buys over a flat list of exceptions is that the *reason* is a named,
//! reusable thing — two languages can share `DESERT`, and the report can say which
//! fact about the speakers explains which gap.
//!
//! # What is deliberately not here
//!
//! ROADMAP M15 names four categories: elaborated, ordinary, borrowed-looking, and
//! absent. Three are implemented. **Borrowed-looking is not**, and the reason is
//! that a loanword is not an ecological fact — it is a *contact* fact. Making a word
//! look borrowed means drawing it from a donor language's phonotactics, and without
//! a donor the only thing on offer is "draw from a deliberately wrong template",
//! which produces a word that looks odd for no stated reason. That is exactly the
//! unsupported claim this module is built to avoid, so it waits for a milestone that
//! models contact and can name the donor.
//!
//! # Where this is going
//!
//! `DESIGN.md` §18.1 names `environment: EnvironmentProfile` as a **field of** the
//! alien embodiment profile (M23). This type is that field, built for humans first.
//! Keeping it a plain declarative record — traits, each with consequences — is what
//! lets a bioluminescent species reuse it without the human ecology being wired in.

use serde::{Deserialize, Serialize};
use stem_core::{Issue, Severity, ValidationReport};

use crate::concept::{ConceptKey, Meaning, concept, nearest_concept_key};

/// The number of absent concepts at or above which a language's vocabulary is
/// reported as **heavily shaped** (`DESIGN.md` §17).
///
/// **Read by both the profile band and the validation Note**, so the two cannot
/// disagree about when a shaping is remarkable (`docs/adr/0009`, the
/// `HIGH_ALLOMORPH_COUNT` discipline). Forty out of 673 is around six per cent of
/// the built-in list — enough that a reader should be told, far short of anything
/// wrong. Neither reading is a judgement: a heavily shaped vocabulary is a *more*
/// considered language, not a less plausible one.
pub const LARGE_VOCABULARY_GAP: usize = 40;

/// One meaning this culture makes several distinctions inside.
///
/// The distinctions are **named, not counted**. `words: 4` would coin four entries
/// all glossed "sand", which is four identical dictionary rows and no information;
/// the author who knows the culture is the one who knows what the four are. The
/// count is `senses.len()`, so the data cannot disagree with itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elaboration {
    /// The concept being elaborated.
    pub concept: ConceptKey,
    /// One gloss per word this culture has for it, in authored order — which is the
    /// order they are coined in, and therefore part of the determinism contract.
    ///
    /// **These replace the concept's own gloss entirely.** If a people distinguish
    /// four kinds of sand, those four *are* their vocabulary for sand; leaving a
    /// generic "sand" beside them would assert a fifth, superordinate word that the
    /// author did not claim.
    pub senses: Vec<String>,
}

/// One meaning this culture has no word for, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Absence {
    /// The concept that goes uncoined.
    pub concept: ConceptKey,
    /// Why these speakers have no word for it. **Required, and the point of the
    /// whole module**: a gap with no reason is the accident M15 exists to replace,
    /// so `reason` is not `Option` and `missing_absence_reason` reports an empty one.
    pub reason: String,
}

/// One fact about the speakers, with the vocabulary that follows from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CultureTrait {
    /// A short id, e.g. `"DESERT"`. Shared between languages that share the trait.
    pub id: String,
    /// A display name, e.g. `"High desert"`.
    pub name: String,
    /// Authorial prose: what this fact about the speakers actually is.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// The meanings this trait makes fine-grained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elaborates: Vec<Elaboration>,
    /// The meanings this trait explains the absence of.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lacks: Vec<Absence>,
}

/// This language's ecology and culture, as the vocabulary it implies.
///
/// Lives **in the genome**, for M12's reason exactly: `seed`'s contract is that a
/// language is reproducible from its own file alone, and a profile that shapes which
/// words get coined is part of what produces the lexicon. A sidecar would break it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentProfile {
    /// One line on where and how these people live. Prose; not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// The traits, in authored order — which is the order the report reads in and
    /// the order conflicts resolve in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<CultureTrait>,
}

impl EnvironmentProfile {
    /// True when nothing is modelled — every pre-M15 file, and every language whose
    /// author has not made these claims. `LanguageGenome` uses it for
    /// `skip_serializing_if`, so such a file round-trips byte-identically.
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty() && self.traits.is_empty()
    }

    /// The trait with this id, or `None`. Linear over an authored `Vec`; a map is
    /// forbidden on any path that reaches output (§9.4), and these lists are tiny.
    pub fn culture_trait(&self, id: &str) -> Option<&CultureTrait> {
        self.traits.iter().find(|t| t.id == id)
    }

    /// The trait that explains why `key` goes uncoined, with its reason — or `None`
    /// when this culture has no such claim.
    ///
    /// **First trait wins**, in authored order, so the answer is deterministic when
    /// two traits both explain one gap. Both are true; the report names the first.
    pub fn absence_of(&self, key: &ConceptKey) -> Option<(&CultureTrait, &Absence)> {
        self.traits
            .iter()
            .find_map(|t| t.lacks.iter().find(|a| &a.concept == key).map(|a| (t, a)))
    }

    /// The trait that elaborates `key`, with the elaboration — or `None`.
    ///
    /// First trait wins, as above.
    pub fn elaboration_of(&self, key: &ConceptKey) -> Option<(&CultureTrait, &Elaboration)> {
        self.traits.iter().find_map(|t| {
            t.elaborates
                .iter()
                .find(|e| &e.concept == key)
                .map(|e| (t, e))
        })
    }
}

// ------------------------------------------------------------------- the shaping

/// What this culture does with one meaning.
///
/// Derived, never stored: it is a *view* of the profile against one concept, the
/// `PlausibilityProfile` relationship to the genome. Storing it beside the profile
/// would be a second source of truth that desynchronises the first time a trait is
/// edited (`docs/adr/0007`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Salience<'a> {
    /// One word, with the concept's own gloss. The default for every meaning no
    /// trait mentions — and for every meaning in a language with no profile at all,
    /// which is why a pre-M15 file behaves exactly as it did.
    Ordinary,
    /// Several words, with these glosses in authored order.
    Elaborated(&'a [String]),
    /// No word, because of this trait, for this reason.
    Absent {
        /// The trait that explains it.
        culture_trait: &'a CultureTrait,
        /// Its stated reason.
        reason: &'a str,
    },
}

impl Salience<'_> {
    /// How many words this meaning is coined as: 0 when absent, 1 when ordinary,
    /// `senses.len()` when elaborated.
    pub fn word_count(&self) -> usize {
        match self {
            Self::Ordinary => 1,
            Self::Elaborated(senses) => senses.len(),
            Self::Absent { .. } => 0,
        }
    }
}

/// What `profile` does with `key`.
///
/// **Absence outranks elaboration**, and it is reported when both are claimed
/// (`contested_concept`): a culture cannot have four words for a thing it has no
/// word for, and resolving it the other way would coin words a trait explicitly said
/// these speakers do not have. Picking the *stronger* claim keeps the report's
/// explanation true — the gap is still explained by the trait that made it.
pub fn salience<'a>(profile: &'a EnvironmentProfile, key: &ConceptKey) -> Salience<'a> {
    if let Some((culture_trait, absence)) = profile.absence_of(key) {
        return Salience::Absent {
            culture_trait,
            reason: &absence.reason,
        };
    }
    match profile.elaboration_of(key) {
        Some((_, elaboration)) => Salience::Elaborated(&elaboration.senses),
        None => Salience::Ordinary,
    }
}

/// The salience of every meaning available to a language, in list order.
///
/// One pass, so [`crate::build_shaped_lexicon`] and [`render_environment`]-style
/// callers read the same answer rather than each re-deriving it — the shared-source
/// discipline `docs/adr/0009` requires of any two values that must agree.
///
/// [`render_environment`]: stem_genome::render_environment
pub fn shaping<'a>(profile: &'a EnvironmentProfile, meanings: &[Meaning<'_>]) -> Vec<Salience<'a>> {
    meanings
        .iter()
        .map(|m| salience(profile, &ConceptKey::new(m.key)))
        .collect()
}

/// How many concepts this profile removes and how many it elaborates, over
/// `meanings` — the raw basis the §17 band and the `large_vocabulary_gap` Note both
/// read, so they cannot disagree.
///
/// Returns `(absent, elaborated, extra_words)`: the third is how many *additional*
/// entries elaboration coins beyond one per elaborated concept, so a caller can
/// state the arithmetic rather than recompute it.
pub fn shaping_counts(
    profile: &EnvironmentProfile,
    meanings: &[Meaning<'_>],
) -> (usize, usize, usize) {
    let mut absent = 0;
    let mut elaborated = 0;
    let mut extra = 0;
    for salience in shaping(profile, meanings) {
        match salience {
            Salience::Absent { .. } => absent += 1,
            Salience::Elaborated(senses) => {
                elaborated += 1;
                extra += senses.len().saturating_sub(1);
            }
            Salience::Ordinary => {}
        }
    }
    (absent, elaborated, extra)
}

// ---------------------------------------------------------------- the reporting

/// The environment checks that need the profile and the project's own concepts.
///
/// A free function taking context, for the reason `check_against_inventory` is one:
/// `Validate::validate(&self)` takes no arguments, and a bare [`EnvironmentProfile`]
/// cannot tell an invented concept key from one the project declared.
///
/// Six checks, all reporting rather than policing (§17) — a strange culture is a
/// design, not a fault:
///
/// - `unknown_environment_concept` — a trait names a key on neither list. A
///   **Warning** with a spelling suggestion: the trait is inert, so the author's
///   stated gap silently is not one, which is the failure mode this module exists to
///   prevent.
/// - `contested_concept` — one trait elaborates what another says is absent.
/// - `duplicate_culture_trait` — two traits share an id, so the report cannot name
///   which one explained a gap.
/// - `empty_elaboration` — `senses: []` coins no word, which is an absence stated in
///   the wrong place and with no reason attached.
/// - `missing_absence_reason` — the gap is back to being unexplained.
/// - `large_vocabulary_gap` — a **Note**, paired to the §17 band by
///   [`LARGE_VOCABULARY_GAP`].
pub fn check_against_environment(
    profile: &EnvironmentProfile,
    project: &[crate::concept::ProjectConcept],
    meanings: &[Meaning<'_>],
) -> ValidationReport {
    let mut report = ValidationReport::new();

    let known = |key: &ConceptKey| concept(key).is_some() || project.iter().any(|p| &p.key == key);

    for (i, culture_trait) in profile.traits.iter().enumerate() {
        if profile.traits[..i].iter().any(|t| t.id == culture_trait.id) {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "duplicate_culture_trait",
                    format!(
                        "two culture traits share the id `{}`; the report can only name the \
                         first, so the second explains nothing",
                        culture_trait.id
                    ),
                )
                .about(&culture_trait.id),
            );
        }

        for elaboration in &culture_trait.elaborates {
            if !known(&elaboration.concept) {
                report.push(unknown(
                    &elaboration.concept,
                    &culture_trait.id,
                    "elaborates",
                ));
            }
            if elaboration.senses.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "empty_elaboration",
                        format!(
                            "`{}` elaborates `{}` into no senses, so it coins no word — that is \
                             an absence, and it belongs in `lacks` where it can carry a reason",
                            culture_trait.id, elaboration.concept
                        ),
                    )
                    .about(&elaboration.concept),
                );
            }
        }

        for absence in &culture_trait.lacks {
            if !known(&absence.concept) {
                report.push(unknown(&absence.concept, &culture_trait.id, "lacks"));
            }
            if absence.reason.trim().is_empty() {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "missing_absence_reason",
                        format!(
                            "`{}` says these speakers lack `{}` but gives no reason; an \
                             unexplained gap is the accident this profile exists to replace",
                            culture_trait.id, absence.concept
                        ),
                    )
                    .about(&absence.concept),
                );
            }
            if let Some((elaborating, _)) = profile.elaboration_of(&absence.concept) {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "contested_concept",
                        format!(
                            "`{}` elaborates `{}` while `{}` says these speakers lack it; \
                             absence wins, so the elaboration coins nothing",
                            elaborating.id, absence.concept, culture_trait.id
                        ),
                    )
                    .about(&absence.concept),
                );
            }
        }
    }

    let (absent, elaborated, extra) = shaping_counts(profile, meanings);
    if absent >= LARGE_VOCABULARY_GAP {
        report.note(
            "large_vocabulary_gap",
            format!(
                "this culture profile removes {absent} of {} available meanings; that is a \
                 strong claim about its speakers, and every one of them is explained in \
                 `stemma culture`",
                meanings.len()
            ),
        );
    }
    if absent > 0 || elaborated > 0 {
        report.note(
            "vocabulary_shaped",
            format!(
                "{absent} meaning(s) uncoined, {elaborated} elaborated into {} extra word(s)",
                extra
            ),
        );
    }

    report
}

fn unknown(key: &ConceptKey, trait_id: &str, verb: &str) -> Issue {
    let suggestion = match nearest_concept_key(key) {
        Some(near) => format!("; did you mean `{near}`?"),
        None => String::new(),
    };
    Issue::new(
        Severity::Warning,
        "unknown_environment_concept",
        format!(
            "`{trait_id}` {verb} `{key}`, which is on neither the built-in list nor this \
             project's own concepts, so the claim has no effect{suggestion}"
        ),
    )
    .about(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::meanings;

    fn desert() -> EnvironmentProfile {
        EnvironmentProfile {
            summary: "High desert; herders who trade north.".to_owned(),
            traits: vec![CultureTrait {
                id: "DESERT".to_owned(),
                name: "High desert".to_owned(),
                note: "Two rivers, no coast within a season's travel.".to_owned(),
                elaborates: vec![Elaboration {
                    concept: ConceptKey::new("SAND"),
                    senses: vec![
                        "fine drifting sand".to_owned(),
                        "coarse sand".to_owned(),
                        "sand that carries a track".to_owned(),
                    ],
                }],
                lacks: vec![Absence {
                    concept: ConceptKey::new("SEA"),
                    reason: "no living speaker has seen open water".to_owned(),
                }],
            }],
        }
    }

    #[test]
    fn an_empty_profile_leaves_every_meaning_ordinary() {
        let profile = EnvironmentProfile::default();
        assert!(profile.is_empty());
        for salience in shaping(&profile, &meanings(&[])) {
            assert_eq!(salience, Salience::Ordinary);
            assert_eq!(salience.word_count(), 1);
        }
    }

    #[test]
    fn an_elaborated_concept_is_coined_once_per_named_sense() {
        let profile = desert();
        match salience(&profile, &ConceptKey::new("SAND")) {
            Salience::Elaborated(senses) => {
                assert_eq!(senses.len(), 3);
                assert_eq!(senses[0], "fine drifting sand");
            }
            other => panic!("expected an elaboration, got {other:?}"),
        }
        assert_eq!(salience(&profile, &ConceptKey::new("SAND")).word_count(), 3);
    }

    #[test]
    fn an_absent_concept_names_the_trait_that_explains_it() {
        match salience(&desert(), &ConceptKey::new("SEA")) {
            Salience::Absent {
                culture_trait,
                reason,
            } => {
                assert_eq!(culture_trait.id, "DESERT");
                assert!(reason.contains("open water"), "{reason}");
            }
            other => panic!("expected an absence, got {other:?}"),
        }
    }

    #[test]
    fn a_meaning_no_trait_mentions_stays_ordinary() {
        assert_eq!(
            salience(&desert(), &ConceptKey::new("STAR")),
            Salience::Ordinary
        );
    }

    /// You cannot have three words for a thing you have no word for. Absence is the
    /// stronger claim, so it wins — and the conflict is reported rather than
    /// silently resolved.
    #[test]
    fn absence_outranks_elaboration_and_the_conflict_is_reported() {
        let mut profile = desert();
        profile.traits[0].lacks.push(Absence {
            concept: ConceptKey::new("SAND"),
            reason: "contrived".to_owned(),
        });

        assert_eq!(salience(&profile, &ConceptKey::new("SAND")).word_count(), 0);
        let report = check_against_environment(&profile, &[], &meanings(&[]));
        assert!(
            report.warnings().any(|i| i.code == "contested_concept"),
            "{report}"
        );
        assert!(report.is_ok(), "a contradictory culture is odd, not broken");
    }

    #[test]
    fn a_trait_naming_an_invented_concept_is_reported_with_a_suggestion() {
        let profile = EnvironmentProfile {
            summary: String::new(),
            traits: vec![CultureTrait {
                id: "X".to_owned(),
                name: "X".to_owned(),
                note: String::new(),
                elaborates: Vec::new(),
                lacks: vec![Absence {
                    concept: ConceptKey::new("SEE"), // meant SEA
                    reason: "typo".to_owned(),
                }],
            }],
        };
        // SEE is a real concept, so nothing fires; the *invented* one does.
        let report = check_against_environment(&profile, &[], &meanings(&[]));
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "unknown_environment_concept"),
            "SEE is on the built-in list: {report}"
        );

        let mut invented = profile;
        invented.traits[0].lacks[0].concept = ConceptKey::new("SEAA");
        let report = check_against_environment(&invented, &[], &meanings(&[]));
        let issue = report
            .warnings()
            .find(|i| i.code == "unknown_environment_concept")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(issue.message.contains("SEA"), "{}", issue.message);
    }

    #[test]
    fn an_absence_with_no_reason_is_reported() {
        let mut profile = desert();
        profile.traits[0].lacks[0].reason = "   ".to_owned();
        let report = check_against_environment(&profile, &[], &meanings(&[]));
        assert!(
            report
                .warnings()
                .any(|i| i.code == "missing_absence_reason"),
            "{report}"
        );
    }

    #[test]
    fn an_elaboration_into_no_senses_is_reported_as_a_misplaced_absence() {
        let mut profile = desert();
        profile.traits[0].elaborates[0].senses.clear();
        let report = check_against_environment(&profile, &[], &meanings(&[]));
        assert!(
            report.warnings().any(|i| i.code == "empty_elaboration"),
            "{report}"
        );
    }

    #[test]
    fn two_traits_sharing_an_id_are_reported() {
        let mut profile = desert();
        profile.traits.push(profile.traits[0].clone());
        let report = check_against_environment(&profile, &[], &meanings(&[]));
        assert!(
            report
                .warnings()
                .any(|i| i.code == "duplicate_culture_trait"),
            "{report}"
        );
    }

    /// The band's paired constant, from the report side (`docs/adr/0009`).
    #[test]
    fn a_profile_removing_many_meanings_earns_a_note_at_the_shared_threshold() {
        let all = meanings(&[]);
        let lacks: Vec<Absence> = all[..LARGE_VOCABULARY_GAP]
            .iter()
            .map(|m| Absence {
                concept: ConceptKey::new(m.key),
                reason: "a contrived but stated reason".to_owned(),
            })
            .collect();
        let profile = EnvironmentProfile {
            summary: String::new(),
            traits: vec![CultureTrait {
                id: "BIG".to_owned(),
                name: "Big".to_owned(),
                note: String::new(),
                elaborates: Vec::new(),
                lacks,
            }],
        };
        let (absent, _, _) = shaping_counts(&profile, &all);
        assert_eq!(absent, LARGE_VOCABULARY_GAP);

        let report = check_against_environment(&profile, &[], &all);
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "large_vocabulary_gap"),
            "{report}"
        );
        assert!(
            report.is_ok(),
            "a strongly shaped vocabulary is not a fault"
        );
    }

    #[test]
    fn one_fewer_absence_stays_below_the_threshold() {
        let all = meanings(&[]);
        let lacks: Vec<Absence> = all[..LARGE_VOCABULARY_GAP - 1]
            .iter()
            .map(|m| Absence {
                concept: ConceptKey::new(m.key),
                reason: "stated".to_owned(),
            })
            .collect();
        let profile = EnvironmentProfile {
            summary: String::new(),
            traits: vec![CultureTrait {
                id: "B".to_owned(),
                name: "B".to_owned(),
                note: String::new(),
                elaborates: Vec::new(),
                lacks,
            }],
        };
        let report = check_against_environment(&profile, &[], &all);
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "large_vocabulary_gap"),
            "the band and the Note must switch at exactly the same count: {report}"
        );
    }

    #[test]
    fn the_counts_state_the_arithmetic() {
        let (absent, elaborated, extra) = shaping_counts(&desert(), &meanings(&[]));
        assert_eq!(absent, 1);
        assert_eq!(elaborated, 1);
        assert_eq!(
            extra, 2,
            "three senses is two words beyond the ordinary one"
        );
    }

    #[test]
    fn a_profile_round_trips_through_ron() {
        let profile = desert();
        let text = ron::ser::to_string(&profile).expect("serialise");
        let back: EnvironmentProfile = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, profile);
    }

    #[test]
    fn a_misspelled_profile_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(summary: "x", trates: [])"#;
        assert!(ron::from_str::<EnvironmentProfile>(text).is_err());
    }
}
