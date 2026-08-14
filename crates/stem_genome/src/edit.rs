//! Editing a language, once, in one place (ROADMAP M16, `DESIGN.md` §20.5).
//!
//! # Why this module exists at all
//!
//! M11 built a read-only explorer under one rule — *the UI holds no logic* — and the
//! payoff was that the window and the CLI could not disagree about what a word's
//! history is, because there is one renderer and a bug in it shows up in both.
//!
//! Editing is where that rule is most tempting to break and most expensive to lose. A
//! window that mutated a genome directly would be a second implementation of what an
//! edit *means*, and the two would drift: the CLI would refuse an id collision the
//! window quietly allowed, and a file saved from one would not match a file saved from
//! the other. So an edit is a **value** ([`Edit`]) applied by **one function**
//! ([`apply`]), and the UI's whole contribution is to build the value and show the
//! answer. `a_file_saved_from_the_ui_matches_the_equivalent_cli_command` is that claim
//! as a test.
//!
//! # Validate, then commit — never the other way round
//!
//! [`apply`] works on a **clone**, validates it, and returns the original untouched if
//! the edit introduced an error. That ordering is the whole safety property: a
//! rejected edit cannot leave a half-mutated genome behind, which a mutate-then-check
//! design can and does the first time a caller ignores a returned report.
//!
//! # What counts as a rejection
//!
//! Not "the language has errors" — a file may already be broken, and refusing every
//! edit to a broken file is how an editor becomes useless exactly when it is needed.
//! An edit is refused when it **introduces an error that was not there before**
//! ([`new_errors`]). Warnings and Notes never refuse anything: §17's posture is that
//! the tool reports and does not police, and a user is allowed to make their language
//! strange.
//!
//! # Nothing here saves
//!
//! [`apply`] returns a genome; writing it is the caller's separate, explicit act.
//! Undo is the file — there is no autosave, no journal, and no in-memory history
//! stack, because the file on disk is a better undo than any of those and the user
//! already knows how it works.

use stem_core::{Issue, LanguageId, Result, Severity, StemmaError, Validate, ValidationReport};
use stem_lexicon::{ConceptKey, PartOfSpeech, ProjectConcept, authored_word};
use stem_phonology::Root;

use crate::LanguageGenome;

/// One change a person can make to a language.
///
/// A value, not a method call, so that the UI, the CLI and a test all name the same
/// thing — and so a future undo stack or a scripted batch is a `Vec<Edit>` rather than
/// a redesign. `#[non_exhaustive]` so adding an operation is not a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edit {
    /// Replace a word's gloss overrides with this one gloss.
    ///
    /// Overrides, not senses: [`WordEntry::display_gloss`] lets a modelled sense (M9)
    /// shadow an authored label, and an editor that wrote into `senses` would be
    /// forging a semantic history nothing produced. Setting a gloss on a drifted word
    /// therefore has no visible effect until the drift is removed — which is the
    /// truth, and `gloss_shadowed_by_sense` says so rather than letting the user
    /// wonder.
    SetGloss {
        /// The word to relabel.
        word: stem_core::WordId,
        /// Its new gloss. Empty clears the override, restoring the concept's own.
        gloss: String,
    },
    /// Append a hand-authored word.
    AddWord {
        /// The written form, read through [`Root::parse`] against this language's
        /// own inventory — so a word can only be made of sounds the language has.
        form: String,
        /// Its gloss.
        gloss: String,
        /// The concept it realises, or `None` for a word that names none.
        concept: Option<ConceptKey>,
        /// Its part of speech.
        part_of_speech: PartOfSpeech,
    },
    /// Remove a word by id.
    RemoveWord {
        /// The word to remove.
        word: stem_core::WordId,
    },
    /// Declare a project concept (M12), so this language can coin a meaning the
    /// built-in list does not hold.
    DeclareConcept {
        /// Its key.
        key: ConceptKey,
        /// Its gloss.
        gloss: String,
        /// The part of speech a coined word starts with.
        part_of_speech: PartOfSpeech,
        /// Authorial prose: why this language needs this meaning.
        note: String,
    },
}

impl Edit {
    /// A one-line description, for a confirmation line or a log.
    ///
    /// Lives here rather than at each call site because [`Edit`] is
    /// `#[non_exhaustive]`: a downstream `match` would need a wildcard arm, and a
    /// wildcard is what stops a new variant from being a compile error where it has
    /// to be handled (`Formation::summary`'s precedent).
    pub fn summary(&self) -> String {
        match self {
            Self::SetGloss { word, gloss } if gloss.trim().is_empty() => {
                format!("cleared the gloss override on `{word}`")
            }
            Self::SetGloss { word, gloss } => format!("glossed `{word}` as \"{gloss}\""),
            Self::AddWord { form, gloss, .. } => format!("added `{form}` \"{gloss}\""),
            Self::RemoveWord { word } => format!("removed `{word}`"),
            Self::DeclareConcept { key, gloss, .. } => {
                format!("declared concept `{key}` \"{gloss}\"")
            }
        }
    }
}

/// What an accepted edit did.
///
/// Carries the post-edit report so a caller can show warnings the edit *introduced*
/// without them being a refusal — a user who makes their language odd should see the
/// note and keep their change.
#[derive(Debug, Clone)]
pub struct EditOutcome {
    /// The edited language. The caller decides whether to save it.
    pub genome: LanguageGenome,
    /// Warnings and notes the edit introduced, if any. Never errors: an edit that
    /// introduced one was refused.
    pub introduced: Vec<Issue>,
}

/// Applies `edit` to `genome`, or refuses it.
///
/// Returns a new genome; `genome` is never mutated, so a refusal cannot leave a
/// half-applied change behind. **Saving is the caller's separate act** — nothing here
/// touches the disk.
///
/// # Failure
///
/// Two kinds, and they read differently on purpose:
///
/// - The edit cannot be *expressed*: a word id that is not in the lexicon, a form
///   with a sound this language does not have. A [`StemmaError`] naming what was not
///   found, straight from the library that knows.
/// - The edit would *break* the language: an id collision, a word left with no
///   meaning. A [`StemmaError::Invalid`] listing the errors it would have introduced,
///   so the message is the report rather than a summary of it.
pub fn apply(genome: &LanguageGenome, edit: &Edit) -> Result<EditOutcome> {
    let before = genome.validate();
    let mut draft = genome.clone();

    match edit {
        Edit::SetGloss { word, gloss } => {
            let entry = draft
                .lexicon
                .iter_mut()
                .find(|e| &e.id == word)
                .ok_or_else(|| StemmaError::not_found("word", word))?;
            entry.glosses = if gloss.trim().is_empty() {
                Vec::new()
            } else {
                vec![gloss.trim().to_owned()]
            };
        }

        Edit::AddWord {
            form,
            gloss,
            concept,
            part_of_speech,
        } => {
            // Parsed against this language's own inventory, so a word cannot be made
            // of sounds it does not have — the check `unknown_phoneme` would
            // otherwise report *after* the fact.
            let phonemic_form = Root::parse(form, &draft.phonemes)?;
            let ordinal = next_ordinal(&draft);
            // Built by `stem_lexicon`, not here. Minting a `CognateSetId` is the one
            // operation the whole cognate invariant rests on, and
            // `stem_genome_never_mints_a_cognate_set` scans this crate's source to
            // keep it out — so the editor asks the lexicon crate for a word rather
            // than assembling one, which is where that construction already lives for
            // coined, inflected and derived words too.
            let entry = authored_word(
                &draft.id,
                ordinal,
                phonemic_form,
                gloss,
                concept.clone(),
                *part_of_speech,
            );
            draft.lexicon.push(entry);
        }

        Edit::RemoveWord { word } => {
            let before_len = draft.lexicon.len();
            draft.lexicon = stem_lexicon::Lexicon::from_entries(
                draft.lexicon.iter().filter(|e| &e.id != word).cloned(),
            );
            if draft.lexicon.len() == before_len {
                return Err(StemmaError::not_found("word", word));
            }
        }

        Edit::DeclareConcept {
            key,
            gloss,
            part_of_speech,
            note,
        } => {
            if draft.concepts.iter().any(|c| &c.key == key) {
                return Err(StemmaError::not_found(
                    "an undeclared concept key (this language already declares)",
                    key,
                ));
            }
            draft.concepts.push(ProjectConcept {
                key: key.clone(),
                gloss: gloss.trim().to_owned(),
                part_of_speech: *part_of_speech,
                note: note.trim().to_owned(),
            });
        }
    }

    let after = draft.validate();
    let introduced = new_issues(&before, &after);
    let errors: Vec<&Issue> = introduced
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        // The report itself, not a summary of it: `StemmaError::Invalid` carries a
        // `ValidationReport` precisely so a refusal can show the graded issues a
        // reader already knows how to read, in the same shape `stemma validate`
        // prints them.
        let mut refusal = ValidationReport::new();
        for issue in errors {
            refusal.push(issue.clone());
        }
        return Err(StemmaError::Invalid(
            format!("this edit ({})", edit.summary()),
            refusal,
        ));
    }

    Ok(EditOutcome {
        genome: draft,
        introduced: introduced
            .into_iter()
            .filter(|i| i.severity != Severity::Error)
            .collect(),
    })
}

/// The ordinal a newly added word takes.
///
/// `lexicon.len() + 1`, the rule `derive` uses — collision-free for the sequential
/// ids every generator produces. A hand-edited file with gaps can still collide, and
/// that is caught by `duplicate_word_id` becoming a *new* error, which refuses the
/// edit with the id named. Guessing a free id instead would silently paper over a
/// file whose ids the user should know are irregular.
fn next_ordinal(genome: &LanguageGenome) -> usize {
    genome.lexicon.len() + 1
}

/// The issues in `after` that were not in `before`, matched on `(severity, code,
/// subject)`.
///
/// Multiset semantics, not set: two words can carry the same code about different
/// subjects, and a *second* `duplicate_word_id` is a new fault even though the code
/// already appeared. Matching on `subject` is what distinguishes them; matching on the
/// message would tie the refusal rule to prose wording, which is exactly the kind of
/// coupling that turns a message edit into a behaviour change.
pub fn new_issues(before: &ValidationReport, after: &ValidationReport) -> Vec<Issue> {
    let mut remaining: Vec<&Issue> = before.issues.iter().collect();
    let mut introduced = Vec::new();
    for issue in &after.issues {
        match remaining.iter().position(|old| {
            old.severity == issue.severity && old.code == issue.code && old.subject == issue.subject
        }) {
            Some(i) => {
                remaining.swap_remove(i);
            }
            None => introduced.push(issue.clone()),
        }
    }
    introduced
}

/// The errors `after` has that `before` did not — [`new_issues`] filtered.
///
/// The refusal rule, exposed so a caller can ask the question before committing to an
/// edit (a UI greying out a button, a test asserting the rule directly).
pub fn new_errors(before: &ValidationReport, after: &ValidationReport) -> Vec<Issue> {
    new_issues(before, after)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect()
}

/// Moves the rule at `from` to index `to` in a rule set (M16).
///
/// Rule order is chronology and it is **observable** — M3's acceptance turns on
/// `*taka` giving `tag` under one order and `tak` under another — so reordering is a
/// real edit with real consequences, not a cosmetic one. It lives here rather than on
/// [`stem_soundchange::RuleSet`] for the same reason every other edit does: one
/// function, called by both front ends, so the window and the CLI cannot disagree
/// about what "move rule 2 to 0" means.
///
/// A **stable rotation**, not a swap: moving rule 3 to 0 makes it first and pushes
/// 0, 1 and 2 down one, which is what a person dragging a row expects. A swap would
/// silently reorder two rules when the user asked about one.
///
/// # Failure
///
/// An out-of-range index is a [`StemmaError::Invalid`] naming the bound. The set is
/// returned unmodified — the same validate-then-commit ordering [`apply`] uses.
pub fn move_rule(
    rules: &stem_soundchange::RuleSet,
    from: usize,
    to: usize,
) -> Result<stem_soundchange::RuleSet> {
    let len = rules.rules.len();
    if from >= len || to >= len {
        let mut report = ValidationReport::new();
        report.push(Issue::new(
            Severity::Error,
            "rule_index_out_of_range",
            format!(
                "this set has {len} rule(s), numbered 0..{}; asked to move {from} to {to}",
                len.saturating_sub(1)
            ),
        ));
        return Err(StemmaError::Invalid("a rule index".to_owned(), report));
    }
    let mut moved = rules.clone();
    let rule = moved.rules.remove(from);
    moved.rules.insert(to, rule);
    Ok(moved)
}

/// The id a genome would carry after an edit — unchanged, always.
///
/// Editing never renames a language. A [`LanguageId`] is what `parent` edges, the
/// lineage graph and every `CognateSetId` scope point at, so changing it from an
/// editor would orphan a family silently. Renaming is a `fork`.
pub fn identity_is_immutable(genome: &LanguageGenome) -> &LanguageId {
    &genome.id
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_lexicon::{Lexicon, WordSource, scoped_cognate_set};
    use stem_phonology::{Phoneme, PhonemeInventory, SegmentKind};

    fn genome() -> LanguageGenome {
        let inventory = PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel),
        ]);
        let lexicon = stem_lexicon::build_proto_lexicon(
            &LanguageId::new("x"),
            &inventory,
            &stem_phonology::Phonotactics {
                templates: vec![stem_phonology::WeightedTemplate::new("CV")],
                syllables_per_root: vec![stem_phonology::WeightedSyllableCount::new(2)],
            },
            &stem_lexicon::meanings(&[])[..5],
            42,
        )
        .expect("coins");
        LanguageGenome::proto("x", "X")
            .with_phonemes(inventory)
            .with_lexicon(lexicon)
    }

    fn word_id(n: usize) -> stem_core::WordId {
        stem_core::WordId::sequential(n)
    }

    // ------------------------------------------------------------- setting a gloss

    #[test]
    fn setting_a_gloss_changes_only_that_word() {
        let before = genome();
        let outcome = apply(
            &before,
            &Edit::SetGloss {
                word: word_id(1),
                gloss: "everything".to_owned(),
            },
        )
        .expect("accepted");

        let edited = outcome.genome;
        assert_eq!(
            edited.lexicon.require(&word_id(1)).unwrap().display_gloss(),
            Some("everything")
        );
        // Every other word is untouched, byte for byte.
        for (a, b) in before.lexicon.iter().zip(edited.lexicon.iter()).skip(1) {
            assert_eq!(a, b);
        }
        // And the original is untouched: `apply` takes a reference and clones.
        assert_eq!(
            before.lexicon.require(&word_id(1)).unwrap().display_gloss(),
            Some("all")
        );
    }

    #[test]
    fn an_empty_gloss_clears_the_override_and_restores_the_concepts_own() {
        let genome = genome();
        let set = apply(
            &genome,
            &Edit::SetGloss {
                word: word_id(1),
                gloss: "everything".to_owned(),
            },
        )
        .expect("accepted")
        .genome;
        let cleared = apply(
            &set,
            &Edit::SetGloss {
                word: word_id(1),
                gloss: "   ".to_owned(),
            },
        )
        .expect("accepted")
        .genome;
        assert_eq!(
            cleared
                .lexicon
                .require(&word_id(1))
                .unwrap()
                .display_gloss(),
            Some("all"),
            "clearing an override falls back to the concept, not to nothing"
        );
    }

    #[test]
    fn glossing_a_word_that_is_not_there_names_the_id() {
        let err = apply(
            &genome(),
            &Edit::SetGloss {
                word: word_id(999),
                gloss: "x".to_owned(),
            },
        )
        .expect_err("no such word");
        assert!(err.to_string().contains("w_0999"), "{err}");
    }

    // ---------------------------------------------------------------- adding words

    #[test]
    fn adding_a_word_parses_its_form_against_this_languages_inventory() {
        let outcome = apply(
            &genome(),
            &Edit::AddWord {
                form: "taki".to_owned(),
                gloss: "a new thing".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
        )
        .expect("accepted");

        let added = outcome.genome.lexicon.iter().last().expect("appended");
        assert_eq!(added.display_gloss(), Some("a new thing"));
        assert_eq!(
            added
                .segments()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            "ph_t ph_a ph_k ph_i"
        );
        assert_eq!(added.source, WordSource::Authored);
        assert_eq!(
            added.id.as_str(),
            "w_0006",
            "appended after the five coined"
        );
        assert!(
            added.cognate_set.as_str().starts_with("cog_x_"),
            "minted through the one sanctioned site"
        );
    }

    /// The acceptance's "a bad feature": a form using a sound this language has not
    /// got is refused, with the offending text named — the caller is a person typing
    /// into a box.
    #[test]
    fn adding_a_word_with_a_sound_this_language_lacks_is_refused_and_says_which() {
        let err = apply(
            &genome(),
            &Edit::AddWord {
                form: "tazi".to_owned(),
                gloss: "x".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
        )
        .expect_err("no /z/ in this language");
        assert!(err.to_string().contains("zi"), "{err}");
    }

    #[test]
    fn a_word_added_twice_gets_two_ids_and_two_cognate_sets() {
        let once = apply(
            &genome(),
            &Edit::AddWord {
                form: "ta".to_owned(),
                gloss: "one".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
        )
        .expect("accepted")
        .genome;
        let twice = apply(
            &once,
            &Edit::AddWord {
                form: "ta".to_owned(),
                gloss: "two".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
        )
        .expect("accepted")
        .genome;

        assert_eq!(twice.lexicon.len(), 7);
        let ids: std::collections::BTreeSet<&str> =
            twice.lexicon.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids.len(), 7, "no id collides");
        // Homophones are legitimate and reported, never prevented — the M2 rule —
        // so adding a second `ta` must not have introduced an error. Stated as
        // `new_errors` against the starting genome rather than as absolute validity:
        // this toy inventory declares no features, which is an error of its own that
        // has nothing to do with the edit.
        let introduced = new_errors(&genome().validate(), &twice.validate());
        assert!(introduced.is_empty(), "{introduced:?}");
    }

    /// **The acceptance's "id collision".** A file whose ids are irregular can make
    /// `len + 1` collide; the edit is refused rather than written.
    #[test]
    fn an_edit_that_would_collide_an_id_is_refused_and_never_applied() {
        let mut genome = genome();
        // Five words, but the last is `w_0006` — so `len + 1` wants a taken id.
        let mut clash = genome.lexicon.iter().last().unwrap().clone();
        clash.id = word_id(6);
        clash.cognate_set = scoped_cognate_set(&LanguageId::new("x"), 6);
        genome.lexicon = Lexicon::from_entries(
            genome
                .lexicon
                .iter()
                .take(4)
                .cloned()
                .chain(std::iter::once(clash)),
        );
        assert!(
            !genome
                .validate()
                .errors()
                .any(|i| i.code == "lexicon.duplicate_word_id"),
            "no id collides yet — that is what the edit is about to cause"
        );

        let err = apply(
            &genome,
            &Edit::AddWord {
                form: "ta".to_owned(),
                gloss: "x".to_owned(),
                concept: None,
                part_of_speech: PartOfSpeech::Noun,
            },
        )
        .expect_err("w_0006 is taken");
        assert!(err.to_string().contains("duplicate_word_id"), "{err}");
        assert_eq!(genome.lexicon.len(), 5, "the genome is untouched");
    }

    #[test]
    fn removing_a_word_removes_exactly_one() {
        let before = genome();
        let after = apply(&before, &Edit::RemoveWord { word: word_id(2) })
            .expect("accepted")
            .genome;
        assert_eq!(after.lexicon.len(), before.lexicon.len() - 1);
        assert!(after.lexicon.get(&word_id(2)).is_none());
        assert!(after.lexicon.get(&word_id(3)).is_some(), "no renumbering");
    }

    #[test]
    fn removing_a_word_that_is_not_there_is_an_error() {
        assert!(apply(&genome(), &Edit::RemoveWord { word: word_id(99) }).is_err());
    }

    // ----------------------------------------------------------- project concepts

    #[test]
    fn declaring_a_concept_appends_it_to_the_genome() {
        let outcome = apply(
            &genome(),
            &Edit::DeclareConcept {
                key: ConceptKey::new("OBSIDIAN"),
                gloss: "black glass".to_owned(),
                part_of_speech: PartOfSpeech::Noun,
                note: "the trade good".to_owned(),
            },
        )
        .expect("accepted");
        assert_eq!(outcome.genome.concepts.len(), 1);
        assert_eq!(outcome.genome.concepts[0].gloss, "black glass");
    }

    #[test]
    fn declaring_a_concept_twice_is_refused() {
        let once = apply(
            &genome(),
            &Edit::DeclareConcept {
                key: ConceptKey::new("OBSIDIAN"),
                gloss: "black glass".to_owned(),
                part_of_speech: PartOfSpeech::Noun,
                note: String::new(),
            },
        )
        .expect("accepted")
        .genome;
        assert!(
            apply(
                &once,
                &Edit::DeclareConcept {
                    key: ConceptKey::new("OBSIDIAN"),
                    gloss: "again".to_owned(),
                    part_of_speech: PartOfSpeech::Noun,
                    note: String::new(),
                },
            )
            .is_err()
        );
    }

    /// Shadowing a built-in is a **Warning**, so the edit is accepted and the warning
    /// is handed back — §17's posture, applied to editing: the tool reports, the user
    /// decides.
    #[test]
    fn declaring_a_concept_that_shadows_a_builtin_is_accepted_with_the_warning_shown() {
        let outcome = apply(
            &genome(),
            &Edit::DeclareConcept {
                key: ConceptKey::new("STAR"),
                gloss: "a different star".to_owned(),
                part_of_speech: PartOfSpeech::Noun,
                note: String::new(),
            },
        )
        .expect("a warning must not refuse an edit");
        assert!(
            outcome
                .introduced
                .iter()
                .any(|i| i.code == "concepts.shadows_builtin"),
            "{:?}",
            outcome.introduced
        );
    }

    // ------------------------------------------------------------- the refusal rule

    /// An already-broken file must stay editable, or the editor is useless exactly
    /// when it is needed.
    #[test]
    fn an_edit_to_an_already_broken_language_is_still_accepted() {
        let mut broken = genome();
        // Two words with one id — an existing Error.
        let clash = broken.lexicon.iter().next().unwrap().clone();
        broken.lexicon.push(clash);
        assert!(
            broken
                .validate()
                .errors()
                .any(|i| i.code == "lexicon.duplicate_word_id"),
            "broken to start"
        );

        let outcome = apply(
            &broken,
            &Edit::SetGloss {
                word: word_id(2),
                gloss: "still editable".to_owned(),
            },
        )
        .expect("a pre-existing error must not refuse an unrelated edit");
        assert!(
            outcome
                .genome
                .validate()
                .errors()
                .any(|i| i.code == "lexicon.duplicate_word_id"),
            "the pre-existing fault is still there — the edit did not silently repair it"
        );
        assert_eq!(
            outcome
                .genome
                .lexicon
                .require(&word_id(2))
                .unwrap()
                .display_gloss(),
            Some("still editable")
        );
    }

    #[test]
    fn new_issues_matches_on_subject_so_a_second_fault_under_one_code_is_new() {
        let a = ValidationReport::new();
        let mut b = ValidationReport::new();
        b.push(Issue::new(Severity::Error, "dup", "one").about("w_0001"));
        b.push(Issue::new(Severity::Error, "dup", "two").about("w_0002"));
        assert_eq!(new_errors(&a, &b).len(), 2, "different subjects, both new");

        // And the same issue present before is not "introduced".
        assert!(new_errors(&b, &b).is_empty());
    }

    // ----------------------------------------------------------------- rule order

    fn rule_set() -> stem_soundchange::RuleSet {
        // Four rules distinguishable only by id: reordering reads `rules` as an
        // opaque `Vec` and must not care what a rule *does*, so a test that gave
        // them real feature patterns would be asserting something else.
        let rule = |id: &str| stem_soundchange::SoundChangeRule {
            id: stem_core::RuleId::new(id),
            name: id.to_owned(),
            description: String::new(),
            chronology_years: 0,
            target: stem_soundchange::SegmentPattern {
                features: stem_phonology::FeatureBundle::EMPTY,
                stress: None,
            },
            environment: stem_soundchange::Environment::default(),
            change: stem_soundchange::Change::Delete,
        };
        stem_soundchange::RuleSet {
            id: "r".to_owned(),
            name: "R".to_owned(),
            description: String::new(),
            rules: vec![rule("a"), rule("b"), rule("c"), rule("d")],
        }
    }

    fn ids(set: &stem_soundchange::RuleSet) -> Vec<&str> {
        set.rules.iter().map(|r| r.id.as_str()).collect()
    }

    /// A rotation, not a swap: moving 3 to 0 pushes the rest down one, which is what
    /// dragging a row looks like.
    #[test]
    fn moving_a_rule_rotates_rather_than_swapping() {
        let moved = move_rule(&rule_set(), 3, 0).expect("in range");
        assert_eq!(ids(&moved), ["d", "a", "b", "c"]);

        let back = move_rule(&moved, 0, 3).expect("in range");
        assert_eq!(ids(&back), ["a", "b", "c", "d"], "and it is reversible");
    }

    #[test]
    fn moving_a_rule_to_its_own_index_changes_nothing() {
        let set = rule_set();
        assert_eq!(move_rule(&set, 2, 2).expect("in range"), set);
    }

    #[test]
    fn an_out_of_range_rule_index_is_refused_and_names_the_bound() {
        let err = move_rule(&rule_set(), 0, 9).expect_err("out of range");
        assert!(err.to_string().contains("0..3"), "{err}");
    }
}
