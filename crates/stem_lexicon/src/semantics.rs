//! Meaning as a modelled, drifting, traceable thing (`DESIGN.md` §7.5, M9).
//!
//! # The structural claim
//!
//! **Meaning is modelled exactly as form is.** The correspondence is field for
//! field, and it is the whole design:
//!
//! | form (M3) | meaning (M9) |
//! |---|---|
//! | [`WordEntry::phonemic_form`] | [`WordEntry::senses`] |
//! | [`WordEntry::trace`] ([`Derivation`]) | [`WordEntry::sense_history`] ([`SenseHistory`]) |
//! | `Derivation::input`, never rewritten | [`SenseHistory::input`], never rewritten |
//! | `RuleApplication { rule, index }` | [`SenseShift`] `{ event, index }` |
//! | `replay()` / `final_form()` | [`SenseHistory::replay`] / [`SenseHistory::final_senses`] |
//! | `genome.applied_rules` (a log) | `genome.applied_drifts` (a log) |
//!
//! Nothing here is a second mechanism for a job the project already has one for.
//! [`SenseHistory`] is `stem_lexicon`'s for [`Derivation`]'s reason (`trace.rs`):
//! a §3.3 record stored *on* a [`WordEntry`] cannot live in a crate above
//! `stem_lexicon` without a cycle.
//!
//! # What M9 exists to show
//!
//! §10.2's worked example: `*takala` "star" becomes **"omen"** in Coastal while
//! staying **"star"** in Highland — and the two reflexes remain **one cognate
//! set**, so they keep sharing a row in the comparative table. That is why
//! `concept` (what a word was coined for), `cognate_set` (what it descends from)
//! and `senses` (what it means *now, here*) are three separate identities and have
//! been since M2 (`docs/adr/0007`). **Drift writes only `senses`.**
//!
//! # v0 applies authored drift; it never invents drift
//!
//! A [`DriftSet`] is a file of authored events, exactly as a `RuleSet` is a file
//! of authored rules (§20.1, and the "formal engine is the source of truth"
//! constraint). There is no probabilistic drift, no simulation, and no LLM.
//! [`apply_drift`] is pure, total and RNG-free.

use serde::{Deserialize, Serialize};
use stem_core::{
    EventId, Issue, Result, SemanticNodeId, Severity, Validate, ValidationReport, WordId,
};

use crate::concept::ConceptKey;
use crate::lexicon::Lexicon;
use crate::word::WordEntry;

// ------------------------------------------------------------- the sense model

/// One sense this language distinguishes: a meaning a word can hold (§7.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticNode {
    /// Stable identity within this language, e.g. `sn_omen`.
    pub id: SemanticNodeId,

    /// The English label a reader sees.
    ///
    /// **The source of truth for this sense's gloss.** Every [`SenseRef::gloss`]
    /// echo is copied from here by [`apply_drift`] and nowhere else, and
    /// `semantics.stale_sense_gloss` catches a hand edit that desynchronises them.
    pub gloss: String,

    /// The built-in concept this sense *is*, where [`crate::CONCEPTS`] already
    /// names it.
    ///
    /// `sn_star` anchors `STAR`; `sn_omen` anchors nothing, and inventing a
    /// Concepticon id it does not have would be the false provenance
    /// [`crate::Concept::concepticon_id`] forbids. **Never a resolution fallback**
    /// — it is the comparison/CLDF anchor only, so an unanchored sense is fully
    /// first-class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept: Option<ConceptKey>,

    /// Authorial prose. Not interpreted by the engine.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// One word's link to one sense, with the sense's gloss **echoed**.
///
/// # Why the gloss is echoed
///
/// [`WordEntry::display_gloss`] must stay **context-free**: it takes no arguments,
/// and `stem_soundchange::render_derivation` calls it. A context-taking
/// `display_gloss(&SemanticSpace)` would make the sound-change engine name a
/// semantic type — spending exactly the separation `docs/adr/0010` bought by
/// keeping morphology out of it. `MorphemeRef` echoes a gloss for the same reason
/// ("echoed for the same reason — the ref is self-contained"), and this improves on
/// that precedent by pairing the echo with a staleness Warning.
///
/// [`apply_drift`] is the **only** write site in the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SenseRef {
    /// Which sense.
    pub node: SemanticNodeId,
    /// Its gloss, echoed from [`SemanticNode::gloss`] so the ref is self-contained.
    pub gloss: String,
}

/// One language's senses, in authored order (`DESIGN.md` §8.1's `SemanticSpace`).
///
/// A `Vec`, never a map — authored order is part of the determinism contract
/// (§9.4) and it is the order the space renders in.
///
/// **Not a family registry.** [`Lexicon`]'s docs forbid a cognate registry inside a
/// language-scoped type because nothing coordinates two independently authored
/// files. This is not that: it is *this* language's own sense inventory, copied
/// verbatim by `fork` and `evolve`, so a daughter file is self-contained and
/// `stemma trace` renders "star → omen" with no drift file on disk — the `seed`
/// contract's "reproducible from the file alone", applied to meaning. Cross-file
/// agreement is by **string id**, and disagreement is *reported*
/// (`family.semantic_node_conflict`), never silently merged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticSpace {
    /// The senses, in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<SemanticNode>,
}

impl SemanticSpace {
    /// An empty space.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when nothing is modelled — every pre-M9 file. `LanguageGenome` uses it
    /// for `skip_serializing_if`, so such a file round-trips byte-identically.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// How many senses are declared.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// The node with this id, or `None`. Linear over an authored `Vec`; a map is
    /// forbidden on any path that reaches output (§9.4), and these lists are tiny.
    pub fn node(&self, id: &SemanticNodeId) -> Option<&SemanticNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// The nearest declared id to a typo, for the `unknown_node` diagnostic.
    pub fn nearest_node_id(&self, id: &SemanticNodeId) -> Option<&str> {
        stem_core::suggest::nearest(id.as_str(), self.nodes.iter().map(|n| n.id.as_str()))
    }

    /// This space merged with newly `declared` nodes: **inherited first, then
    /// declared, first-seen wins, order preserved.**
    ///
    /// The semantic twin of a rule minting a phoneme the language lacked. A
    /// redeclaration carrying a *different* gloss keeps the existing node and
    /// reports `node_redeclared` — first-declared-wins with no diagnostic is
    /// Lexurgy issue #9, and the fix is to say so (the same reasoning that puts
    /// `ambiguous_with` in a `SymbolResolution`).
    pub fn merged_with(&self, declared: &[SemanticNode]) -> (SemanticSpace, Vec<Issue>) {
        let mut merged = self.clone();
        let mut issues = Vec::new();
        for node in declared {
            match merged.node(&node.id) {
                Some(existing) => {
                    if existing.gloss != node.gloss {
                        issues.push(
                            Issue::new(
                                Severity::Warning,
                                "node_redeclared",
                                format!(
                                    "this sense is already declared as \"{}\" and is redeclared \
                                     as \"{}\"; the existing declaration wins, so the drift will \
                                     use \"{}\"",
                                    existing.gloss, node.gloss, existing.gloss
                                ),
                            )
                            .about(&node.id),
                        );
                    }
                }
                None => merged.nodes.push(node.clone()),
            }
        }
        (merged, issues)
    }
}

// ------------------------------------------------------------------ the record

/// One word's recorded semantic history (§3.3 for meaning; §10.2's "Semantic
/// shift" line).
///
/// [`Derivation`]'s twin, and deliberately so: only `input` plus per-event deltas
/// are stored, and [`Self::replay`] reconstructs every intermediate sense set. A
/// stored snapshot beside the deltas would be a second source of truth that
/// desynchronises the first time anything touches either.
///
/// [`Derivation`]: crate::trace::Derivation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SenseHistory {
    /// The sense set entering the **first** drift event this word ever met.
    ///
    /// A later stratum extends `steps` and leaves this alone — `Derivation::input`'s
    /// contract verbatim — so a history always begins at the proto sense however
    /// many strata were applied, across `fork` and `evolve` alike.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<SemanticNodeId>,

    /// One entry per drift event that **named** this word, in application order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<SenseShift>,
}

/// One drift event's effect on one word.
///
/// Emitted whenever the event **named** this word, even when both deltas come out
/// empty — "the event was named here and changed nothing" is precisely the fact the
/// user needs, the same reasoning that emits a `RuleApplication` carrying only
/// `blocked`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SenseShift {
    /// Which event.
    ///
    /// A name, never a denormalised label: the mechanism, register, date and prose
    /// are read from `genome.applied_drifts[index]`, so a renamed event cannot
    /// leave a stale label behind in a stored history.
    pub event: EventId,

    /// This event's index in the genome's `applied_drifts`, **globally** across
    /// strata — `RuleApplication::index`'s contract.
    pub index: u32,

    /// What was **actually** removed — the effective delta, not the declared one.
    ///
    /// An event naming a sense this word never held removes nothing, and the record
    /// must say what happened rather than what was asked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<SemanticNodeId>,

    /// What was actually added, same reasoning.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<SemanticNodeId>,
}

/// Applies one step's deltas to `current`, returning what was **effectively**
/// removed and added.
///
/// **This is the single definition of what a drift step does.** Both
/// [`apply_drift`] (which performs a step) and [`SenseHistory::replay`] (which
/// reconstructs one) go through here, so the state the engine stores and the state
/// the record replays to cannot diverge — and `semantics.sense_history_desync` is an
/// Error precisely for that divergence, so a second transcription of this fold would
/// let the engine emit a genome that fails its own validation.
///
/// The two subtleties both fall out of using one implementation rather than needing
/// to be restated as caveats: a sense named in **both** `remove` and `add` is
/// dropped and then re-added (a re-assertion survives), and a repeated id inside
/// `add` lands once (`current` grows as it goes).
fn advance(
    current: &mut Vec<SemanticNodeId>,
    remove: &[SemanticNodeId],
    add: &[SemanticNodeId],
) -> (Vec<SemanticNodeId>, Vec<SemanticNodeId>) {
    let removed: Vec<SemanticNodeId> = current
        .iter()
        .filter(|id| remove.contains(id))
        .cloned()
        .collect();
    current.retain(|id| !remove.contains(id));

    let mut added = Vec::new();
    for id in add {
        if !current.contains(id) {
            current.push(id.clone());
            added.push(id.clone());
        }
    }
    (removed, added)
}

impl SenseHistory {
    /// Every intermediate sense set, one per step, folding `steps` over `input`.
    ///
    /// Consults no space, no event and no engine — it applies stored ids at stored
    /// positions, exactly as `Derivation::replay` applies stored segments. So
    /// §16.3's "the record reconstructs the state" is a real statement about the
    /// *file*: the history is sufficient to rebuild the senses, i.e. not lossy.
    pub fn replay(&self) -> Vec<Vec<SemanticNodeId>> {
        let mut current = self.input.clone();
        let mut states = Vec::with_capacity(self.steps.len());
        for step in &self.steps {
            advance(&mut current, &step.removed, &step.added);
            states.push(current.clone());
        }
        states
    }

    /// The last intermediate, or `input` when `steps` is empty.
    pub fn final_senses(&self) -> Vec<SemanticNodeId> {
        self.replay().pop().unwrap_or_else(|| self.input.clone())
    }

    /// How many shifts this word has recorded — the plausibility basis.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// True when no event has touched this word.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

// ------------------------------------------------------------- drift, as data

/// §7.5's ten named pathways.
///
/// Closed for `docs/adr/0004`'s typo argument (a misspelled mechanism must be a
/// load error, not a silently registered category), and `#[non_exhaustive]` on the
/// [`crate::PartOfSpeech`] precedent so appending one is not a breaking change.
///
/// **Descriptive, never a switch.** No code branches on it to compute a value; it
/// reaches the renderer and the plausibility basis and nothing else — the
/// [`crate::WordSource`] discipline. Judging whether a given mechanism makes a
/// given shift *plausible* needs a typology of attested pathways this project does
/// not have, and inventing one would be the fabrication §17 forbids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DriftMechanism {
    /// Resemblance: `star` → `divine sign`.
    Metaphor,
    /// Contiguity: `the sign` → `the thing signified`.
    Metonymy,
    /// The sense covers less: `deer` from `animal`.
    Narrowing,
    /// The sense covers more: `bird` from `young bird`.
    Broadening,
    /// The sense worsens: `silly` from `blessed`.
    Pejoration,
    /// The sense improves: `nice` from `ignorant`.
    Amelioration,
    /// A word is avoided and another takes its place.
    TabooReplacement,
    /// A sense is raised into religious use.
    ReligiousElevation,
    /// A sense narrows into a craft or discipline.
    TechnicalSpecialization,
    /// A sense inverts in in-group speech: `wicked` meaning excellent.
    SlangInversion,
}

impl DriftMechanism {
    /// The lowercase name, as it appears in RON and in renders.
    pub fn name(self) -> &'static str {
        match self {
            Self::Metaphor => "metaphor",
            Self::Metonymy => "metonymy",
            Self::Narrowing => "narrowing",
            Self::Broadening => "broadening",
            Self::Pejoration => "pejoration",
            Self::Amelioration => "amelioration",
            Self::TabooReplacement => "taboo replacement",
            Self::ReligiousElevation => "religious elevation",
            Self::TechnicalSpecialization => "technical specialization",
            Self::SlangInversion => "slang inversion",
        }
    }
}

impl std::fmt::Display for DriftMechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One authored semantic shift — §8.5's `HistoricalEventKind::SemanticShift`, as
/// data.
///
/// # Why it targets one word
///
/// Semantic change is **lexically idiosyncratic**: it has no Neogrammarian
/// regularity, so there is no semantic analogue of "voiceless stops voice between
/// vowels". Matching by sense instead would drift every word holding `sn_star` at
/// once, asserting a regularity that semantics does not have. A `WordId` target is
/// unambiguous (`duplicate_word_id` is already an Error) and survives forking, since
/// word ids are copied verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriftEvent {
    /// Stable identity, `ev_0001`. [`EventId`]'s first producer in the workspace.
    pub id: EventId,

    /// A short name, e.g. "The star stands for the will behind it".
    pub name: String,

    /// Why this happened — the authorial reason, rendered in the trace.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// The word this event acts on. Absent from the lexicon →
    /// `drift.target_not_found` (Warning), never an Error.
    pub word: WordId,

    /// Which of §7.5's pathways this is.
    pub mechanism: DriftMechanism,

    /// Absolute years from the lineage root — the same field, same meaning, as
    /// `SoundChangeRule::chronology_years`, so §10.4's unified timeline is one
    /// merge of two ordered logs away and needs nothing stored.
    #[serde(default)]
    pub chronology_years: i32,

    /// §10.2's "in priestly register".
    ///
    /// A free-text **label**: register is a fact about one culture, not a
    /// typological universal, and a closed enum would render "ritual" where the
    /// worked example says "priestly". Nothing computes over it. (`WordEntry`'s
    /// own §8.3 `register` field stays deferred — still no producer.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub register: Option<String>,

    /// Senses this event displaces. Empty is legal — pure broadening adds without
    /// removing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<SemanticNodeId>,

    /// Senses this event confers. Empty is legal — a taboo loss removes without
    /// adding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<SemanticNodeId>,
}

/// One stratum's authored semantic history — a file, exactly as a `RuleSet` is.
///
/// A **set** (ids unique; `duplicate_id` is an Error). The genome's
/// `applied_drifts` is a **log** and may repeat ids — the same distinction the
/// project already draws between a `RuleSet` and `applied_rules`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriftSet {
    /// Stable identity for the file.
    pub id: String,
    /// Display name.
    pub name: String,
    /// What this stratum of semantic history was.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// The senses this stratum **introduces**, merged into the language's space
    /// before any event runs.
    ///
    /// The semantic twin of a rule minting a phoneme the language lacked: it keeps
    /// the file self-contained, so one drift set applies to any daughter without
    /// the daughter having to predeclare the senses it is about to acquire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<SemanticNode>,

    /// The events, **in order**. Order is chronology; never a set, never re-sorted.
    pub events: Vec<DriftEvent>,
}

impl Validate for DriftSet {
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.events.is_empty() {
            report.note("empty", "this drift set declares no events");
        }

        // A set is a set: two events sharing an id would make a `SenseShift`
        // ambiguous about which one it records.
        let mut seen: Vec<&str> = Vec::new();
        for event in &self.events {
            if seen.contains(&event.id.as_str()) {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "duplicate_id",
                        "two events in one set share this id; a history could not name which \
                         applied",
                    )
                    .about(&event.id),
                );
            }
            seen.push(event.id.as_str());

            if event.remove.is_empty() && event.add.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Note,
                        "empty_delta",
                        "this event neither removes nor adds a sense, so it can only ever \
                         record that it was named",
                    )
                    .about(&event.id),
                );
            }
        }

        // Same reasoning as the rule set's chronology check: the timeline label
        // must not silently contradict application order.
        for window in self.events.windows(2) {
            if window[1].chronology_years < window[0].chronology_years {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "chronology_not_monotonic",
                        format!(
                            "event `{}` is dated {} years but follows `{}` at {}; application \
                             order is list order, and a timeline would contradict the history",
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
}

// ----------------------------------------------------------------- the applier

/// What applying a drift set produced: the new lexicon, the possibly-grown space,
/// and what happened.
///
/// **The three are one value and must be applied together** — taking the lexicon
/// while keeping the old space yields `semantics.unknown_sense_node` on every
/// sense the set introduced. `LanguageGenome::with_drift` is the sanctioned caller.
#[derive(Debug, Clone, PartialEq)]
pub struct DriftOutcome {
    /// The drifted entries, in input order. Only `senses` and `sense_history` move.
    pub lexicon: Lexicon,
    /// The input space plus every node the set introduced, in authored order.
    pub space: SemanticSpace,
    /// Innovations, misses and no-ops. Bare codes; the caller absorbs them.
    pub report: ValidationReport,
}

/// Applies `set`'s events, in authored order, to `lexicon`.
///
/// Mirrors `apply_rules(rules, first_index, inventory, prosody, lexicon)`: an
/// `offset` so [`SenseShift::index`] stays globally meaningful across strata, and a
/// possibly-grown space returned alongside (the grown-inventory analogue).
///
/// **No RNG, no map, no clock, no float, no sort.** A pure function of its four
/// arguments — the same determinism claim `apply_rules` makes.
///
/// Per event, over entries in **stored order**:
/// 1. the effective `removed` is `event.remove` ∩ held, in **held** order;
/// 2. the effective `added` is `event.add` minus held, in **`event.add`** order;
/// 3. survivors keep their relative order and `added` is **appended** — a sense set
///    has no intrinsic positions, so salience order after a shift is the event's
///    declared order. Stated, deterministic, total;
/// 4. each added ref's gloss is copied from the node — **the only [`SenseRef`]
///    write site in the workspace**;
/// 5. a [`SenseShift`] is pushed whether or not the deltas were empty.
///
/// Untouched on every entry: `id`, `concept`, `cognate_set`, `phonemic_form`,
/// `glosses`, `part_of_speech`, `source`, `trace`, `morphemes`. `WordSource` is
/// never consulted — an M8 `Derived` paradigm cell drifts identically to a root
/// (`docs/adr/0010`).
pub fn apply_drift(
    set: &DriftSet,
    offset: u32,
    space: &SemanticSpace,
    lexicon: &Lexicon,
) -> Result<DriftOutcome> {
    let mut report = ValidationReport::new();

    // The set's own senses join the space once, before any event runs.
    let (space, redeclarations) = space.merged_with(&set.nodes);
    for issue in redeclarations {
        report.push(issue);
    }

    let mut entries: Vec<WordEntry> = lexicon.iter().cloned().collect();

    for (i, event) in set.events.iter().enumerate() {
        let index = offset + i as u32;

        let Some(entry) = entries.iter_mut().find(|e| e.id == event.word) else {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "target_not_found",
                    format!(
                        "event `{}` (\"{}\") names word `{}`, which this lexicon does not hold; \
                         nothing drifted",
                        event.id, event.name, event.word
                    ),
                )
                .about(&event.id),
            );
            continue;
        };

        // The sense set as it stood before this event — the history's `input` when
        // this is the first event ever to name the word.
        let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();

        // The effective deltas — what actually happened, not what was asked for —
        // computed by **the same `advance` the record's `replay` uses**. Sharing one
        // implementation is what makes "the state the engine stores" and "the state
        // the record replays to" agree by construction rather than by comment.
        let mut current = held.clone();
        let (removed, added) = advance(&mut current, &event.remove, &event.add);

        // Survivors keep their order; the acquired senses are appended.
        entry.senses.retain(|s| !removed.contains(&s.node));
        for id in &added {
            let gloss = match space.node(id) {
                Some(node) => node.gloss.clone(),
                // Unreachable through the gate (`drift.unknown_node` is an Error),
                // but a direct caller could reach it. The id is a truthful
                // last-resort label — never a fabricated gloss.
                None => id.as_str().to_owned(),
            };
            entry.senses.push(SenseRef {
                node: id.clone(),
                gloss,
            });
        }

        let history = entry.sense_history.get_or_insert_with(|| SenseHistory {
            input: held,
            steps: Vec::new(),
        });
        history.steps.push(SenseShift {
            event: event.id.clone(),
            index,
            removed: removed.clone(),
            added: added.clone(),
        });

        // A declared removal that matched nothing. Reported even when the event
        // *did* something else, because `no_effect` below only fires when the whole
        // event was inert — so half an event silently doing nothing would otherwise
        // pass unremarked, and "why didn't my change apply here?" is the question
        // this project treats as the one worth always answering. A Note: naming a
        // sense a word does not hold is odd, not broken.
        for id in &event.remove {
            if !removed.contains(id) {
                report.push(
                    Issue::new(
                        Severity::Note,
                        "removal_matched_nothing",
                        format!(
                            "event `{}` removes sense `{id}` from word `{}`, which did not hold \
                             it; that half of the event changed nothing",
                            event.id, event.word
                        ),
                    )
                    .about(&event.id),
                );
            }
        }

        if removed.is_empty() && added.is_empty() {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "no_effect",
                    format!(
                        "event `{}` (\"{}\") named word `{}` but changed nothing — it removed no \
                         sense the word held and added none it lacked",
                        event.id, event.name, event.word
                    ),
                )
                .about(&event.id),
            );
        }
        if entry.senses.is_empty() {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "left_no_sense",
                    format!(
                        "event `{}` left word `{}` with no sense at all; it will fall back to \
                         its concept's gloss, which is probably not what was meant",
                        event.id, event.word
                    ),
                )
                .about(&event.id),
            );
        }
    }

    Ok(DriftOutcome {
        lexicon: Lexicon::from_entries(entries),
        space,
        report,
    })
}

/// The checks that need the drift set *and* the language it will run over — the
/// `check_against_language` shape, and a free function for the same reason
/// (`Validate::validate(&self)` takes no context).
///
/// Two Errors of structure; everything else reports (§17). `depth_years` is the
/// target language's `lineage_depth_years`, for the chronology Note.
pub fn check_drift_against_language(
    set: &DriftSet,
    space: &SemanticSpace,
    lexicon: &Lexicon,
    depth_years: i32,
) -> ValidationReport {
    let mut report = ValidationReport::new();

    // The set's own nodes are in scope for its own events.
    let (space, _) = space.merged_with(&set.nodes);

    for event in &set.events {
        for id in event.remove.iter().chain(event.add.iter()) {
            if space.node(id).is_none() {
                let suggestion = match space.nearest_node_id(id) {
                    Some(near) => format!("; did you mean `{near}`?"),
                    None => String::new(),
                };
                report.push(
                    Issue::new(
                        Severity::Error,
                        "unknown_node",
                        format!(
                            "event `{}` names sense `{id}`, which neither this language nor the \
                             drift set declares, so no gloss could be written for it{suggestion}",
                            event.id
                        ),
                    )
                    .about(&event.id),
                );
            }
        }

        if !lexicon.iter().any(|e| e.id == event.word) {
            report.push(
                Issue::new(
                    Severity::Warning,
                    // A DISTINCT code from the applier's `target_not_found`: both
                    // fire for one condition, and `with_drift` runs the check and
                    // then the applier, so one shared code would print the same
                    // fact twice and read as two problems. The sound-change
                    // precedent keeps its pre-flight and run codes disjoint too.
                    "target_not_in_lexicon",
                    format!(
                        "event `{}` names word `{}`, which this lexicon does not hold",
                        event.id, event.word
                    ),
                )
                .about(&event.id),
            );
        }

        if event.chronology_years > depth_years {
            report.push(
                Issue::new(
                    Severity::Note,
                    "event_after_language_depth",
                    format!(
                        "event `{}` is dated {} years but this language stands at {} years",
                        event.id, event.chronology_years, depth_years
                    ),
                )
                .about(&event.id),
            );
        }
    }

    report
}

/// The integrity checks over a language's own senses — absorbed under `"semantics"`
/// by `LanguageGenome::validate`, beside `check_against_inventory`.
///
/// **Every check is gated on non-empty senses / history / space**, so a pre-M9
/// language reports exactly nothing new and the reference fixture's pinned code
/// list is undisturbed.
pub fn check_against_semantics(
    lexicon: &Lexicon,
    space: &SemanticSpace,
    applied_drifts: &[DriftEvent],
) -> ValidationReport {
    let mut report = ValidationReport::new();

    // Duplicate node ids make every resolution ambiguous — the `duplicate_word_id`
    // precedent, and an Error for the same reason.
    let mut seen: Vec<&str> = Vec::new();
    for node in &space.nodes {
        if seen.contains(&node.id.as_str()) {
            report.push(
                Issue::new(
                    Severity::Error,
                    "duplicate_node_id",
                    "two senses share this id; which one a word means would be ambiguous",
                )
                .about(&node.id),
            );
        }
        seen.push(node.id.as_str());
    }

    for entry in lexicon.iter() {
        for reference in &entry.senses {
            match space.node(&reference.node) {
                None => {
                    let suggestion = match space.nearest_node_id(&reference.node) {
                        Some(near) => format!("; did you mean `{near}`?"),
                        None => String::new(),
                    };
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "unknown_sense_node",
                            format!(
                                "word `{}` holds sense `{}`, which this language does not \
                                 declare{suggestion}",
                                entry.id, reference.node
                            ),
                        )
                        .about(&entry.id),
                    );
                }
                // The echo's paired guard: the ref carries a copy of the gloss, so
                // something must notice when the copy goes stale.
                Some(node) if node.gloss != reference.gloss => {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "stale_sense_gloss",
                            format!(
                                "word `{}` labels sense `{}` \"{}\", but the sense itself says \
                                 \"{}\"; the word's copy is stale",
                                entry.id, reference.node, reference.gloss, node.gloss
                            ),
                        )
                        .about(&entry.id),
                    );
                }
                Some(_) => {}
            }
        }

        let Some(history) = &entry.sense_history else {
            continue;
        };

        // The record must reproduce the state, or the trace lies. Compared as
        // SETS: salience order is authorial and carries no claim.
        let replayed = history.final_senses();
        let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();
        // Containment **both ways**, not just replayed-in-held. A one-way check
        // plus equal lengths passes on `replayed = [A, A]` against
        // `held = [A, B]` — every replayed id is present, the counts agree, and `B`
        // is a sense the word holds with no recorded provenance whatsoever, which
        // is precisely what this Error exists to catch.
        let same = replayed.len() == held.len()
            && replayed.iter().all(|id| held.contains(id))
            && held.iter().all(|id| replayed.contains(id));
        if !same {
            report.push(
                Issue::new(
                    Severity::Error,
                    "sense_history_desync",
                    format!(
                        "word `{}` records a history that replays to a different sense set than \
                         it holds; the recorded meaning history cannot be trusted",
                        entry.id
                    ),
                )
                .about(&entry.id),
            );
        }

        // Each step must act on what the previous one produced, or replay is
        // meaningless.
        //
        // The intermediates come from `replay` itself rather than being re-folded
        // here: `replay` is documented as the single definition of what a step
        // does, and a second hand-rolled copy of that fold would be free to drift
        // from it — which would surface only as a spurious or missing
        // `discontinuous_history`, i.e. this check silently lying about the very
        // thing it exists to catch.
        let states = history.replay();
        for (i, step) in history.steps.iter().enumerate() {
            let before = if i == 0 {
                &history.input
            } else {
                &states[i - 1]
            };
            if let Some(missing) = step.removed.iter().find(|id| !before.contains(id)) {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "discontinuous_history",
                        format!(
                            "word `{}` records event `{}` removing sense `{missing}`, which the \
                             word did not hold at that point; the history does not chain",
                            entry.id, step.event
                        ),
                    )
                    .about(&entry.id),
                );
            }
        }
    }

    // A declared sense nothing uses is probably a typo — but an *intermediate*
    // sense in a chain is normal (Coastal's `sn_divine_sign` is held by nothing at
    // the end), so an event naming it counts as use. Note, never louder.
    for node in &space.nodes {
        let held = lexicon
            .iter()
            .any(|e| e.senses.iter().any(|s| s.node == node.id));
        let named = applied_drifts
            .iter()
            .any(|ev| ev.remove.contains(&node.id) || ev.add.contains(&node.id));
        if !held && !named {
            report.push(
                Issue::new(
                    Severity::Note,
                    "node_unused",
                    format!(
                        "sense `{}` (\"{}\") is declared but no word holds it and no recorded \
                         event names it",
                        node.id, node.gloss
                    ),
                )
                .about(&node.id),
            );
        }
    }

    report
}

// ------------------------------------------------------------- the measure (§17)

/// The recorded-chain length at or above which a word's semantic history is
/// reported as remarkable.
///
/// **Read by both the profile band and the validation Note**, so the two cannot
/// disagree (`docs/adr/0009`). §7.5's own example chains are two steps
/// (`hand → control → authority`), and the M9 demo's Coastal chain is two — which,
/// by M8's rule that the tool's own showcase must sit *below* the extreme bar, must
/// not trip the Note. Three or more recorded shifts on one word is where
/// "remarkable" begins.
///
/// A deliberately loose tripwire, not a cited typological constant: the field has
/// no attested "senses per word per millennium" figure, and claiming one would be
/// the fabrication §17 forbids.
pub const LONG_SENSE_CHAIN: usize = 3;

/// One word's recorded semantic chain — the plausibility basis, shown beside the
/// band for honesty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenseChain {
    /// Which word.
    pub word: WordId,
    /// Its displayed gloss, for reading the profile without the lexicon to hand.
    pub gloss: String,
    /// How many shifts it records.
    pub shifts: usize,
}

/// Every word carrying a sense history, with its shift count, in **lexicon order**.
///
/// Reads only `WordEntry.sense_history` — no space, no event, no engine.
/// [`crate::morphological_irregularity`]'s twin: a `Vec`, never a map; `usize`,
/// never a float.
pub fn sense_chains(lexicon: &Lexicon) -> Vec<SenseChain> {
    lexicon
        .iter()
        .filter_map(|entry| {
            entry.sense_history.as_ref().map(|history| SenseChain {
                word: entry.id.clone(),
                gloss: entry.display_gloss().unwrap_or("?").to_owned(),
                shifts: history.len(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_core::PhonemeId;
    use stem_phonology::{Root, Syllable};

    fn node(id: &str, gloss: &str) -> SemanticNode {
        SemanticNode {
            id: SemanticNodeId::new(id),
            gloss: gloss.to_owned(),
            concept: None,
            note: String::new(),
        }
    }

    fn space() -> SemanticSpace {
        SemanticSpace {
            nodes: vec![
                node("sn_star", "star"),
                node("sn_divine_sign", "divine sign"),
                node("sn_omen", "omen"),
                node("sn_royal_sign", "royal sign"),
            ],
        }
    }

    /// One word, `w_0001`, holding the inherited sense `sn_star`.
    fn lexicon() -> Lexicon {
        Lexicon::from_entries([WordEntry {
            id: WordId::new("w_0001"),
            concept: Some(ConceptKey::new("STAR")),
            phonemic_form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![PhonemeId::new("ph_t"), PhonemeId::new("ph_a")],
                    stress: None,
                }],
            },
            glosses: Vec::new(),
            part_of_speech: crate::PartOfSpeech::Noun,
            // Via the sanctioned mint site: the scan in `lib.rs` deliberately does
            // not exempt `#[cfg(test)]`, and this module must never name
            // `CognateSetId::new` — drift must not be able to touch ancestry.
            cognate_set: crate::scoped_cognate_set(&stem_core::LanguageId::new("x"), 1),
            source: crate::WordSource::Authored,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: vec![SenseRef {
                node: SemanticNodeId::new("sn_star"),
                gloss: "star".to_owned(),
            }],
            sense_history: None,
        }])
    }

    fn event(id: &str, remove: &[&str], add: &[&str]) -> DriftEvent {
        DriftEvent {
            id: EventId::new(id),
            name: format!("event {id}"),
            description: String::new(),
            word: WordId::new("w_0001"),
            mechanism: DriftMechanism::Metaphor,
            chronology_years: 100,
            register: None,
            remove: remove.iter().map(|s| SemanticNodeId::new(*s)).collect(),
            add: add.iter().map(|s| SemanticNodeId::new(*s)).collect(),
        }
    }

    /// §10.2's chain: star → divine sign → omen, royal sign.
    fn coastal_set() -> DriftSet {
        DriftSet {
            id: "drift_coastal".to_owned(),
            name: "Coastal".to_owned(),
            description: String::new(),
            nodes: Vec::new(),
            events: vec![
                event("ev_0001", &["sn_star"], &["sn_divine_sign"]),
                event(
                    "ev_0002",
                    &["sn_divine_sign"],
                    &["sn_omen", "sn_royal_sign"],
                ),
            ],
        }
    }

    fn glosses_of(lexicon: &Lexicon) -> Vec<String> {
        lexicon
            .iter()
            .next()
            .unwrap()
            .senses
            .iter()
            .map(|s| s.gloss.clone())
            .collect()
    }

    // --- the acceptance mechanism, at unit level ---

    #[test]
    fn a_two_step_drift_replaces_the_sense_and_records_both_steps() {
        let out = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        assert!(out.report.is_ok(), "{}", out.report);

        assert_eq!(glosses_of(&out.lexicon), ["omen", "royal sign"]);

        let entry = out.lexicon.iter().next().unwrap();
        assert_eq!(
            entry.display_gloss(),
            Some("omen"),
            "the drifted sense wins"
        );

        let history = entry.sense_history.as_ref().expect("history recorded");
        assert_eq!(
            history.input,
            vec![SemanticNodeId::new("sn_star")],
            "the history begins at the inherited sense"
        );
        assert_eq!(history.steps.len(), 2);
        assert_eq!(history.steps[0].index, 0);
        assert_eq!(history.steps[1].index, 1);
    }

    /// §3.3 for meaning: the record alone reconstructs the state.
    #[test]
    fn the_recorded_history_replays_to_the_stored_senses() {
        let out = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let entry = out.lexicon.iter().next().unwrap();
        let replayed = entry.sense_history.as_ref().unwrap().final_senses();
        let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();
        assert_eq!(replayed, held);
    }

    #[test]
    fn drift_touches_only_senses_and_sense_history() {
        let before = lexicon();
        let out = apply_drift(&coastal_set(), 0, &space(), &before).expect("applies");
        let a = before.iter().next().unwrap();
        let b = out.lexicon.iter().next().unwrap();

        assert_eq!(a.id, b.id);
        assert_eq!(a.concept, b.concept, "the etymological anchor is untouched");
        assert_eq!(
            a.cognate_set, b.cognate_set,
            "THE acceptance: ancestry survives the drift"
        );
        assert_eq!(a.phonemic_form, b.phonemic_form);
        assert_eq!(
            a.glosses, b.glosses,
            "an authored override is never clobbered"
        );
        assert_eq!(a.part_of_speech, b.part_of_speech);
        assert_eq!(a.source, b.source);
        assert_eq!(a.trace, b.trace);
        assert_eq!(a.morphemes, b.morphemes);
        assert_ne!(a.senses, b.senses, "only meaning moved");
    }

    #[test]
    fn applying_a_drift_set_twice_produces_identical_output() {
        let a = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let b = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        assert_eq!(a.lexicon, b.lexicon, "no RNG: two runs agree");
        assert_eq!(a.space, b.space);
    }

    /// A second stratum extends `steps` and leaves `input` alone — `Derivation`'s
    /// contract, for meaning.
    #[test]
    fn a_second_drift_set_extends_the_history_and_never_rewrites_its_input() {
        let first = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let second_set = DriftSet {
            id: "later".to_owned(),
            events: vec![event("ev_0003", &["sn_omen"], &[])],
            ..coastal_set()
        };
        let second = apply_drift(&second_set, 2, &first.space, &first.lexicon).expect("applies");

        let history = second
            .lexicon
            .iter()
            .next()
            .unwrap()
            .sense_history
            .as_ref()
            .unwrap();
        assert_eq!(
            history.input,
            vec![SemanticNodeId::new("sn_star")],
            "input still names the ORIGINAL sense after a second stratum"
        );
        assert_eq!(history.steps.len(), 3);
        assert_eq!(
            history.steps[2].index, 2,
            "indices continue from the offset"
        );
    }

    // --- effective deltas, not declared ones ---

    #[test]
    fn removing_a_sense_the_word_never_held_records_an_empty_removal_and_warns() {
        let set = DriftSet {
            events: vec![event("ev_x", &["sn_omen"], &[])],
            ..coastal_set()
        };
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        let history = out
            .lexicon
            .iter()
            .next()
            .unwrap()
            .sense_history
            .as_ref()
            .unwrap();
        assert!(
            history.steps[0].removed.is_empty(),
            "the record says what happened, not what was asked"
        );
        assert!(out.report.warnings().any(|i| i.code == "no_effect"));
    }

    /// **The applier and `replay` must never disagree**, or the engine emits a
    /// genome that fails its own `sense_history_desync` Error.
    ///
    /// Regression: `add: ["sn_omen", "sn_omen"]` used to store the sense twice
    /// while `replay` (which pushes only `if !current.contains`) reconstructed it
    /// once — length 2 versus 1, so a fixture the engine itself produced validated
    /// as broken.
    #[test]
    fn a_repeated_id_inside_one_add_is_stored_once_so_the_record_still_replays() {
        let set = DriftSet {
            events: vec![event("ev_x", &["sn_star"], &["sn_omen", "sn_omen"])],
            ..coastal_set()
        };
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        let entry = out.lexicon.iter().next().unwrap();

        assert_eq!(glosses_of(&out.lexicon), ["omen"], "stored once, not twice");
        let history = entry.sense_history.as_ref().unwrap();
        let replayed = history.final_senses();
        let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();
        assert_eq!(
            replayed, held,
            "the record reconstructs exactly what is stored"
        );

        // And the genome it produced passes its own integrity check.
        let report = check_against_semantics(&out.lexicon, &out.space, &set.events);
        assert!(
            !report.errors().any(|i| i.code == "sense_history_desync"),
            "the engine must not emit a genome that fails its own validation: {report}"
        );
    }

    /// The other half of the same invariant: a sense named in **both** `remove` and
    /// `add` is a re-assertion. `replay` drops it then re-adds it, so the applier
    /// must too — filtering `add` against the pre-event senses would delete it and
    /// desync.
    #[test]
    fn a_sense_both_removed_and_added_survives_because_replay_restores_it() {
        let set = DriftSet {
            events: vec![event("ev_x", &["sn_star"], &["sn_star", "sn_omen"])],
            ..coastal_set()
        };
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        let entry = out.lexicon.iter().next().unwrap();

        assert_eq!(
            glosses_of(&out.lexicon),
            ["star", "omen"],
            "re-asserted senses are kept, in the event's declared order"
        );
        let replayed = entry.sense_history.as_ref().unwrap().final_senses();
        let held: Vec<SemanticNodeId> = entry.senses.iter().map(|s| s.node.clone()).collect();
        assert_eq!(replayed, held, "applier and replay agree");
    }

    /// A sense the word holds with **no recorded provenance** must be caught. A
    /// one-way containment check plus equal lengths lets it through: `[A, A]`
    /// replayed against `[A, B]` has matching counts and every replayed id present,
    /// while `B` came from nowhere.
    #[test]
    fn a_held_sense_with_no_provenance_is_caught_even_when_the_counts_agree() {
        let mut entries: Vec<WordEntry> = lexicon().iter().cloned().collect();
        entries[0].senses = vec![
            SenseRef {
                node: SemanticNodeId::new("sn_star"),
                gloss: "star".to_owned(),
            },
            // Holds `sn_omen`, but the history below never adds it.
            SenseRef {
                node: SemanticNodeId::new("sn_omen"),
                gloss: "omen".to_owned(),
            },
        ];
        entries[0].sense_history = Some(SenseHistory {
            input: vec![
                SemanticNodeId::new("sn_star"),
                SemanticNodeId::new("sn_star"),
            ],
            steps: Vec::new(),
        });
        let report = check_against_semantics(&Lexicon::from_entries(entries), &space(), &[]);
        assert!(
            report.errors().any(|i| i.code == "sense_history_desync"),
            "a sense with no provenance must not hide behind a matching count: {report}"
        );
    }

    /// Half an event doing nothing must still be said. `no_effect` only fires when
    /// the *whole* event was inert, so a removal that matched nothing while the add
    /// succeeded would otherwise pass unremarked.
    #[test]
    fn a_removal_that_matched_nothing_is_reported_even_when_the_add_succeeded() {
        let set = DriftSet {
            // The word holds sn_star, not sn_royal_sign.
            events: vec![event("ev_x", &["sn_royal_sign"], &["sn_omen"])],
            ..coastal_set()
        };
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        assert!(
            out.report
                .issues
                .iter()
                .any(|i| i.code == "removal_matched_nothing"),
            "the inert half of the event must be reported: {}",
            out.report
        );
        assert!(
            !out.report.issues.iter().any(|i| i.code == "no_effect"),
            "the event as a whole DID something, so `no_effect` must stay silent"
        );
    }

    #[test]
    fn a_drift_that_leaves_no_sense_warns_rather_than_erroring() {
        let set = DriftSet {
            events: vec![event("ev_x", &["sn_star"], &[])],
            ..coastal_set()
        };
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        assert!(out.report.is_ok(), "report, do not police: {}", out.report);
        assert!(out.report.warnings().any(|i| i.code == "left_no_sense"));
    }

    #[test]
    fn an_event_naming_an_absent_word_warns_and_drifts_nothing() {
        let mut set = coastal_set();
        set.events = vec![DriftEvent {
            word: WordId::new("w_9999"),
            ..event("ev_x", &["sn_star"], &["sn_omen"])
        }];
        let out = apply_drift(&set, 0, &space(), &lexicon()).expect("applies");
        assert!(
            out.report.warnings().any(|i| i.code == "target_not_found"),
            "{}",
            out.report
        );
        assert_eq!(glosses_of(&out.lexicon), ["star"], "nothing moved");
    }

    // --- the set introduces its own senses ---

    #[test]
    fn a_drift_set_may_introduce_the_senses_it_confers() {
        let set = DriftSet {
            nodes: vec![node("sn_new", "a brand-new sense")],
            events: vec![event("ev_x", &[], &["sn_new"])],
            ..coastal_set()
        };
        // A space that does NOT declare sn_new.
        let bare = SemanticSpace {
            nodes: vec![node("sn_star", "star")],
        };
        let out = apply_drift(&set, 0, &bare, &lexicon()).expect("applies");
        assert!(
            out.space.node(&SemanticNodeId::new("sn_new")).is_some(),
            "the set's node joined the language's space"
        );
        assert_eq!(glosses_of(&out.lexicon), ["star", "a brand-new sense"]);
    }

    #[test]
    fn a_redeclared_sense_keeps_the_existing_gloss_and_says_so() {
        let (merged, issues) = space().merged_with(&[node("sn_star", "a different label")]);
        assert_eq!(
            merged.node(&SemanticNodeId::new("sn_star")).unwrap().gloss,
            "star",
            "first-declared wins"
        );
        assert!(issues.iter().any(|i| i.code == "node_redeclared"));
    }

    // --- the checks ---

    #[test]
    fn an_event_naming_an_undeclared_sense_is_an_error_with_a_suggestion() {
        let set = DriftSet {
            events: vec![event("ev_x", &[], &["sn_ommen"])],
            ..coastal_set()
        };
        let report = check_drift_against_language(&set, &space(), &lexicon(), 500);
        let issue = report
            .errors()
            .find(|i| i.code == "unknown_node")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(issue.message.contains("sn_omen"), "{}", issue.message);
    }

    #[test]
    fn a_stale_echoed_gloss_is_caught() {
        let mut entries: Vec<WordEntry> = lexicon().iter().cloned().collect();
        entries[0].senses[0].gloss = "starre".to_owned();
        let report = check_against_semantics(&Lexicon::from_entries(entries), &space(), &[]);
        assert!(
            report.warnings().any(|i| i.code == "stale_sense_gloss"),
            "{report}"
        );
    }

    #[test]
    fn a_desynchronised_history_is_an_error() {
        let mut entries: Vec<WordEntry> = lexicon().iter().cloned().collect();
        entries[0].sense_history = Some(SenseHistory {
            input: vec![SemanticNodeId::new("sn_omen")],
            steps: Vec::new(),
        });
        let report = check_against_semantics(&Lexicon::from_entries(entries), &space(), &[]);
        assert!(
            report.errors().any(|i| i.code == "sense_history_desync"),
            "a record that cannot reproduce the state makes the trace lie: {report}"
        );
    }

    /// An intermediate sense is held by nothing at the end of a chain, and that is
    /// normal — the event naming it counts as use.
    #[test]
    fn an_intermediate_sense_in_a_chain_is_not_reported_unused() {
        let out = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let report = check_against_semantics(&out.lexicon, &out.space, &coastal_set().events);
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == "node_unused" && i.subject.as_deref() == Some("sn_divine_sign")),
            "a sense passed through is used: {report}"
        );
    }

    #[test]
    fn a_pre_m9_lexicon_reports_nothing_new() {
        let mut entries: Vec<WordEntry> = lexicon().iter().cloned().collect();
        entries[0].senses.clear();
        let report =
            check_against_semantics(&Lexicon::from_entries(entries), &SemanticSpace::new(), &[]);
        assert!(
            report.issues.is_empty(),
            "every check is gated on non-empty semantics: {report}"
        );
    }

    // --- the measure ---

    #[test]
    fn sense_chains_counts_recorded_shifts_in_lexicon_order() {
        let out = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let chains = sense_chains(&out.lexicon);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].shifts, 2);
        assert_eq!(chains[0].gloss, "omen");
        assert!(
            chains[0].shifts < LONG_SENSE_CHAIN,
            "the demo's own chain must sit below the extreme bar"
        );
    }

    #[test]
    fn a_lexicon_with_no_history_has_no_chains() {
        assert!(sense_chains(&lexicon()).is_empty());
    }

    // --- validation of the set itself ---

    #[test]
    fn two_events_sharing_an_id_is_an_error() {
        let set = DriftSet {
            events: vec![event("ev_dup", &[], &[]), event("ev_dup", &[], &[])],
            ..coastal_set()
        };
        let report = set.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_id"),
            "{report}"
        );
    }

    #[test]
    fn a_backwards_chronology_warns() {
        let mut set = coastal_set();
        set.events[0].chronology_years = 500;
        set.events[1].chronology_years = 100;
        let report = set.validate();
        assert!(
            report
                .warnings()
                .any(|i| i.code == "chronology_not_monotonic"),
            "{report}"
        );
    }

    // --- serde ---

    #[test]
    fn a_drift_set_round_trips_through_ron() {
        let set = coastal_set();
        let text = ron::ser::to_string(&set).expect("serialise");
        let back: DriftSet = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, set);
    }

    #[test]
    fn a_sense_history_round_trips_through_ron() {
        let out = apply_drift(&coastal_set(), 0, &space(), &lexicon()).expect("applies");
        let history = out
            .lexicon
            .iter()
            .next()
            .unwrap()
            .sense_history
            .clone()
            .unwrap();
        let text = ron::ser::to_string(&history).expect("serialise");
        let back: SenseHistory = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, history);
    }

    #[test]
    fn a_misspelled_drift_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(id: "d", name: "D", evens: [])"#;
        assert!(ron::from_str::<DriftSet>(text).is_err());
    }

    #[test]
    fn a_mechanism_round_trips_in_snake_case() {
        let json = serde_json::to_string(&DriftMechanism::TabooReplacement).expect("serialise");
        assert_eq!(json, "\"taboo_replacement\"");
        assert_eq!(DriftMechanism::Metonymy.name(), "metonymy");
    }
}
