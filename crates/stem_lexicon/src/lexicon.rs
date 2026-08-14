//! The collection, its validation, and the cross-check against an inventory.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use stem_core::{
    CognateSetId, Issue, Result, Severity, StemmaError, Validate, ValidationReport, WordId,
};
use stem_phonology::{PhonemeInventory, SegmentKind};

use crate::concept::{ConceptKey, nearest_concept_key};
use crate::word::WordEntry;

/// One language's words, in a stable order.
///
/// `#[serde(transparent)]` over a `Vec`, exactly as `PhonemeInventory` is, and for
/// the same reason: **the generated or authored order is the serialised order and
/// the export order**, and a map here would put iteration order into a document
/// that §9.4 requires to be byte-stable. `PROGRESS.md` already records a `HashMap`
/// order leaking into validation output once.
///
/// The transparency also encodes a decision. The obvious thing to put beside
/// `entries` is a cognate-set registry — and it must not live here, because a
/// cognate set is *family*-scoped while a `Lexicon` is *language*-scoped. Forking
/// would give every daughter its own copy, free to disagree about what
/// `cog_proto_asterian_0007` means, and every field it would hold is already
/// derivable from the proto entry that points at it. The registry's home is M4's
/// family object, where there is exactly one of it; at M5 it is a **derived** index
/// over the union of daughter lexicons, built in memory and never stored.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Lexicon {
    entries: Vec<WordEntry>,
}

impl Lexicon {
    /// An empty lexicon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a lexicon from a sequence of entries, preserving their order.
    pub fn from_entries(entries: impl IntoIterator<Item = WordEntry>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    /// Appends an entry.
    pub fn push(&mut self, entry: WordEntry) -> &mut Self {
        self.entries.push(entry);
        self
    }

    /// Every entry, in stored order.
    /// Mutable iteration over the entries, in stored order (M16).
    ///
    /// Editing needs it, and there is no safe way to hand out a `&mut WordEntry`
    /// without it. Order is preserved by construction — this cannot re-sort, and
    /// nothing here mints: `scoped_cognate_set` is still the only site, and the
    /// source scan over this file still proves it.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, WordEntry> {
        self.entries.iter_mut()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, WordEntry> {
        self.entries.iter()
    }

    /// How many entries the lexicon holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the lexicon holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Looks an entry up by ID.
    pub fn get(&self, id: &WordId) -> Option<&WordEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// Looks an entry up by ID, erroring if absent.
    ///
    /// The never-swallow accessor, matching `PhonemeInventory::require`.
    pub fn require(&self, id: &WordId) -> Result<&WordEntry> {
        self.get(id)
            .ok_or_else(|| StemmaError::not_found("word", id))
    }

    /// The entry reflecting `set` in this language, if it survives here.
    ///
    /// **The one lookup M5's cognate table is built on**, shipped now so the M2
    /// types are shaped to a query that already exists on paper (§10.3): for each
    /// row of the proto-lexicon, for each language, `by_cognate_set(row)` is the
    /// cell, and `None` renders as a genuine loss rather than shifting the table.
    ///
    /// Returns the **first** match in stored order. M2's builder never produces
    /// two (a test asserts it) and `lexicon.duplicate_cognate_set` warns if a file
    /// does, but the method is well-defined either way because a `Vec` has an
    /// order. M4's doublets are legitimate, which is why that check is a Warning;
    /// when they arrive this method keeps meaning "the primary reflex" and gains a
    /// sibling returning all of them. Additive.
    ///
    /// Linear. A hundred entries across a handful of languages is not worth an
    /// index, and an index would be a map — see the type docs on why no map may
    /// exist here. M5 may build one in memory; it must never store one.
    pub fn by_cognate_set(&self, set: &CognateSetId) -> Option<&WordEntry> {
        self.entries.iter().find(|e| &e.cognate_set == set)
    }

    /// Every entry expressing this meaning, in stored order.
    ///
    /// Returns a `Vec` rather than an `Option` even though M2 produces at most one
    /// per concept, because synonymy is real, M7 will produce more, and a caller
    /// that has to choose should be *told* it is choosing.
    pub fn by_concept(&self, key: &ConceptKey) -> Vec<&WordEntry> {
        self.entries
            .iter()
            .filter(|e| e.concept.as_ref() == Some(key))
            .collect()
    }

    /// Every entry whose *displayed* gloss matches `meaning`, case-insensitively,
    /// in stored order. The token→word resolver shared by `stemma cognates` and
    /// `stemma trace-word`, so the two can never disagree about what a meaning
    /// names (M5).
    ///
    /// Matches [`WordEntry::display_gloss`] — the label a reader sees — **not**
    /// `concept`: the meaning a user types is the *surface* gloss, so `king` must
    /// find `*rekan` (concept MAN, gloss override "king"), §10.3's own
    /// `king → *rekan` row. A concept-key match (`concept_by_gloss("king") →
    /// KING`, then `by_concept(KING) → []`) would render an empty row — the bug
    /// the fixture is built to expose. And because `display_gloss` falls back to
    /// the concept's own gloss, a word with no override (`MOTHER *mikala`) still
    /// resolves via "mother". `docs/adr/0007`: meaning gets you *into* the row;
    /// ancestry (`cognate_set`) fills it.
    ///
    /// A `Vec`, not `Option`, for the reason [`Self::by_concept`] is: a displayed
    /// gloss is not unique within a lexicon (synonymy, or an override colliding
    /// with another entry's concept gloss), so a caller that must pick is *told*
    /// it is picking. Empty = the meaning is absent here. Linear, stored order,
    /// no map (§9.4).
    pub fn by_meaning(&self, meaning: &str) -> Vec<&WordEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.display_gloss()
                    .is_some_and(|g| g.eq_ignore_ascii_case(meaning))
            })
            .collect()
    }

    /// A one-line summary for CLI output, in `LanguageGenome::summary`'s style.
    ///
    /// Deliberately says nothing about homophones: detecting those needs the
    /// inventory (two entries are homophones when they *render* the same, not when
    /// their segment ids match), and this method has none. Reporting a number here
    /// that disagreed with the printed dictionary would be the validator and the
    /// engine disagreeing, in a `String`.
    pub fn summary(&self) -> String {
        let concepts: BTreeSet<&str> = self
            .entries
            .iter()
            .filter_map(|e| e.concept.as_ref().map(ConceptKey::as_str))
            .collect();
        format!(
            "{} words over {} concepts",
            self.entries.len(),
            concepts.len()
        )
    }
}

impl<'a> IntoIterator for &'a Lexicon {
    type Item = &'a WordEntry;
    type IntoIter = std::slice::Iter<'a, WordEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl FromIterator<WordEntry> for Lexicon {
    fn from_iter<T: IntoIterator<Item = WordEntry>>(iter: T) -> Self {
        Self::from_entries(iter)
    }
}

/// Whether an identifier is safe to hand to another tool.
///
/// CSV stays lossless whatever the id is — every field is quoted — but a CLDF
/// consumer would reject anything outside this charset, so it is worth a warning.
pub fn is_portable_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Validate for Lexicon {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.entries.is_empty() {
            report.note("empty", "this language has no lexicon yet");
            return report;
        }

        let mut seen_ids: BTreeMap<&str, usize> = BTreeMap::new();
        let mut seen_sets: BTreeMap<&str, usize> = BTreeMap::new();
        let mut seen_concepts: BTreeMap<&str, usize> = BTreeMap::new();

        for entry in &self.entries {
            if entry.id.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "empty_word_id",
                        "an entry has an empty id, so nothing can address it",
                    )
                    .about(entry.display_gloss().unwrap_or("<no gloss>")),
                );
            }
            if entry.cognate_set.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "empty_cognate_set",
                        "an entry has an empty cognate set, severing it from its family (§8.6)",
                    )
                    .about(&entry.id),
                );
            }
            if entry.phonemic_form.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "empty_form",
                        "an entry has no segments, so there is nothing to render or transform",
                    )
                    .about(&entry.id),
                );
            }

            // The genuine structural break: a dictionary headword with no meaning.
            if entry.display_gloss().is_none() {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "no_gloss",
                        "this entry has no gloss and names no concept the built-in list holds, \
                         so it would print as a headword with no meaning",
                    )
                    .about(&entry.id),
                );
            }

            // `unknown_concept` used to live here. It moved to
            // `check_against_concepts` at M12, because a genome may now declare its
            // own meanings and this method takes no context — so from here it is
            // genuinely unable to tell an invented key from a project one, and
            // warning about both would be the validator contradicting the generator
            // on every word `new-lexicon` had just coined. Same reasoning, and the
            // same remedy, as `check_against_inventory`.

            for (id, label) in [
                (entry.id.as_str(), "word id"),
                (entry.cognate_set.as_str(), "cognate set id"),
            ] {
                if !id.is_empty() && !is_portable_id(id) {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "id_not_portable",
                            format!(
                                "this {label} contains characters outside [A-Za-z0-9_-]; Stemma \
                                 is unaffected but a CLDF consumer would reject it"
                            ),
                        )
                        .about(id),
                    );
                }
            }

            *seen_ids.entry(entry.id.as_str()).or_default() += 1;
            *seen_sets.entry(entry.cognate_set.as_str()).or_default() += 1;
            if let Some(key) = &entry.concept {
                *seen_concepts.entry(key.as_str()).or_default() += 1;
            }
        }

        // `BTreeMap`, so duplicate reports come out in a fixed order however the
        // entries were arranged. Same discipline as `sorted_duplicates`.
        for (id, count) in seen_ids.iter().filter(|&(_, &n)| n > 1) {
            report.push(
                Issue::new(
                    Severity::Error,
                    "duplicate_word_id",
                    format!("{count} entries share this id; `require` would return the wrong word"),
                )
                .about(id),
            );
        }

        // Warning, not Error: M4's doublets — one proto-form, two daughter reflexes
        // — are legitimate, and `by_cognate_set` is well-defined over a `Vec`.
        for (set, count) in seen_sets.iter().filter(|&(_, &n)| n > 1) {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "duplicate_cognate_set",
                    format!(
                        "{count} entries share this cognate set; that is a doublet, which is \
                         real, but `by_cognate_set` will return only the first"
                    ),
                )
                .about(set),
            );
        }

        // Synonymy is real.
        for (concept, count) in seen_concepts.iter().filter(|&(_, &n)| n > 1) {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "duplicate_concept",
                    format!("{count} entries express this concept; they are synonyms"),
                )
                .about(concept),
            );
        }

        report
    }
}

/// The cross-check that needs both halves of the language.
///
/// Called from `LanguageGenome::validate` beside `phonotactics::check_against_inventory`,
/// because `Validate::validate(&self)` takes no context.
pub fn check_against_inventory(
    lexicon: &Lexicon,
    inventory: &PhonemeInventory,
) -> ValidationReport {
    let mut report = ValidationReport::new();

    // Rendered forms, so homophones are counted the way a reader sees them.
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();

    for entry in lexicon.iter() {
        let mut renderable = true;

        for id in entry.segments() {
            if inventory.get(id).is_none() {
                renderable = false;
                report.push(
                    Issue::new(
                        Severity::Error,
                        "unknown_phoneme",
                        format!(
                            "this entry uses segment `{id}`, which is not in this language's \
                             inventory, so its form cannot be rendered or transformed"
                        ),
                    )
                    .about(&entry.id),
                );
            }
        }

        // Nothing reads `pattern` for semantics, so a disagreement is not broken —
        // and M3's vowel loss will legitimately produce one until a resyllabifier
        // exists. Warning, with a message that names both possibilities.
        for syllable in &entry.phonemic_form.syllables {
            let declared: Option<Vec<SegmentKind>> = syllable
                .pattern
                .chars()
                .map(|c| match c {
                    'C' => Some(SegmentKind::Consonant),
                    'V' => Some(SegmentKind::Vowel),
                    _ => None,
                })
                .collect();
            let actual: Option<Vec<SegmentKind>> = syllable
                .segments
                .iter()
                .map(|id| inventory.get(id).map(|p| p.kind))
                .collect();

            if let (Some(declared), Some(actual)) = (declared, actual)
                && declared != actual
            {
                // The discriminator is deliberately the trace rather than
                // `WordSource`: `source` says who typed the word, `trace` says
                // whether there is a *recorded causal explanation* for the
                // mismatch. A word with a derivation has that explanation one
                // `stemma trace` away, so a Warning that would fire on every M3
                // output is noise; Note is the honest severity. A hand-edited
                // mismatch with no derivation stays a Warning.
                let severity = if entry.trace.is_some() {
                    Severity::Note
                } else {
                    Severity::Warning
                };
                report.push(
                    Issue::new(
                        severity,
                        "syllable_shape_mismatch",
                        format!(
                            "syllable `{}` does not describe its own segments; either the \
                             pattern was hand-edited, or a sound change altered the syllable \
                             and no resyllabifier has run (nothing reads the pattern, so \
                             nothing is broken either way)",
                            syllable.pattern
                        ),
                    )
                    .about(&entry.id),
                );
            }
        }

        if renderable && let Ok(form) = entry.written(inventory) {
            *forms.entry(form).or_default() += 1;
        }
    }

    let homophones: usize = forms.values().filter(|&&n| n > 1).map(|n| n - 1).sum();
    if homophones > 0 {
        let shared: Vec<&str> = forms
            .iter()
            .filter(|&(_, &n)| n > 1)
            .map(|(f, _)| f.as_str())
            .take(5)
            .collect();
        report.note(
            "homophones",
            format!(
                "{homophones} entries share a written form with another entry (e.g. {}); \
                 homophony is real and is reported, not prevented (§17)",
                shared.join(", ")
            ),
        );
    }

    report
}

/// The concept checks that need the project's own declared meanings (M12).
///
/// A free function taking context, for the reason `check_against_inventory` is one:
/// `Validate::validate(&self)` takes no arguments, and from inside a bare [`Lexicon`]
/// an invented concept key and a project-declared one are indistinguishable.
///
/// Three checks, all reporting rather than policing (§17):
///
/// - `unknown_concept` — the key is on neither list. A **Warning** with a spelling
///   suggestion: the entry still has an id, a form, a gloss and a cognate set, so it
///   is unusual rather than broken.
/// - `shadows_builtin` — a project concept reuses a compiled key. A **Warning**: two
///   meanings under one key make every join ambiguous, and the compiled one wins —
///   `concept::meanings` drops the declaration, so it coins no word. Expect this
///   after the built-in list grows (M13 collided with three of
///   `fixtures/seafarers.ron`'s own declarations), which is why the message names
///   the compiled list rather than blaming the author.
/// - `stale_project_gloss` — an entry's stored gloss disagrees with the project
///   concept it names. The paired guard for the gloss `build_proto_lexicon` writes
///   onto project-concept entries, exactly as `semantics.stale_sense_gloss` guards
///   the echo on a `SenseRef`.
pub fn check_against_concepts(
    lexicon: &Lexicon,
    project: &[crate::concept::ProjectConcept],
) -> ValidationReport {
    let mut report = ValidationReport::new();

    for declared in project {
        if crate::concept::concept(&declared.key).is_some() {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "shadows_builtin",
                    format!(
                        "`{}` is already on the built-in concept list; the compiled meaning \
                         wins, so this declaration coins no word — drop it, or rename it if \
                         your language means something else by it",
                        declared.key
                    ),
                )
                .about(&declared.key),
            );
        }
    }

    for entry in lexicon.iter() {
        let Some(key) = &entry.concept else { continue };
        if crate::concept::concept(key).is_some() {
            continue;
        }
        match project.iter().find(|p| &p.key == key) {
            Some(declared) => {
                // The entry carries its own copy of the gloss (it must — nothing
                // compiled can supply one). Catch a hand edit that desynchronises it.
                if let Some(shown) = entry.display_gloss()
                    && shown != declared.gloss
                {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "stale_project_gloss",
                            format!(
                                "word `{}` displays \"{shown}\", but the concept `{key}` it \
                                 names says \"{}\"; the word's copy is stale",
                                entry.id, declared.gloss
                            ),
                        )
                        .about(&entry.id),
                    );
                }
            }
            None => {
                let message = match nearest_concept_key(key) {
                    Some(suggestion) => format!(
                        "`{key}` is on neither the built-in list nor this project's own \
                         concepts; did you mean `{suggestion}`?"
                    ),
                    None => format!(
                        "`{key}` is on neither the built-in list nor this project's own \
                         concepts; it will export with its own gloss and no Concepticon anchor"
                    ),
                };
                report.push(
                    Issue::new(Severity::Warning, "unknown_concept", message).about(&entry.id),
                );
            }
        }
    }

    report
}
