//! Syntactic parameters as data (ROADMAP M17, `DESIGN.md` §7.4).
//!
//! # What this crate is, and what it is deliberately not
//!
//! It is a **description of a language's grammar**, validated and rendered. It is
//! **not a syntax engine**: nothing here parses, generates or transforms a sentence.
//! §20.1 names scope explosion as the top risk to this project and a syntax engine is
//! the largest available example, so Phase 5 is split — parameters first (M17),
//! constructions and generation second (M18), syntactic *change* third (M19). Each is
//! independently runnable, and the first one is a profile you can print.
//!
//! That ordering repeats §20.4's, which put the sound-change structs at M3 and the
//! readable rule syntax at M10 so the notation was designed against semantics that
//! already worked. Here the parameters come first so M18's constructions are written
//! against a grammar that already validates.
//!
//! # Harmony is reported, never enforced
//!
//! The typological content of this milestone is that some combinations of parameters
//! are common and some are rare. Verb–object order and adposition–noun order
//! correlate strongly; so do object–verb order and genitive–noun order. These are
//! Greenberg's word-order universals (1963) and the implicational tendencies that
//! Dryer's later cross-linguistic work refined — and they are **tendencies**, not
//! laws. Real languages violate them.
//!
//! So a disharmonic language earns a Warning that names the tendency and says which
//! way it runs, and it still validates. `CLAUDE.md`'s constraint is exact about this:
//! "Unusual designs get warnings; only structurally broken ones get errors. The tool
//! guides the creator, it does not police them. Resist the urge to reject a weird
//! language." A conlanger who wants a VO language with postpositions is describing
//! something rare and is entitled to it.
//!
//! **No frequencies are quoted.** It would be easy to write "only 4% of languages"
//! into a warning message and impossible to verify it from inside this program; that
//! is the same fabricated-provenance rule that keeps invented Concepticon ids out of
//! `stem_lexicon::concept`. The messages name the correlation and its direction, and
//! stop there.
//!
//! # Headedness is derived, never stored
//!
//! §7.4 lists "head directionality" beside the specific orders, and it is tempting to
//! store it. It must not be: it is a *summary* of the specific orders, so storing it
//! makes a second source of truth that disagrees with the first the moment somebody
//! edits one field. It is [`SyntaxProfile::headedness`], computed — and computing it
//! is what makes the harmony report meaningful rather than a consistency check on
//! redundant data. Same rule as the M4 lineage graph, which derives its edges from
//! `parent` and stores none (`docs/adr/0008`).

use serde::{Deserialize, Serialize};
use stem_core::{Issue, Severity, Validate, ValidationReport};

/// The dominant order of subject, object and verb in a declarative main clause.
///
/// "Dominant", not "only": every one of these languages has other orders available
/// under focus, and the profile records the unmarked one. `#[non_exhaustive]` so a
/// future variant is not a breaking change; nothing on disk refers to one by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WordOrder {
    /// Not stated. The default, so a pre-M17 file says nothing rather than
    /// silently claiming to be SVO — the `Phonotactics::default()` rule, which is
    /// empty for exactly this reason.
    #[default]
    Unspecified,
    /// Subject–object–verb: the commonest order among the world's languages.
    Sov,
    /// Subject–verb–object.
    Svo,
    /// Verb–subject–object.
    Vso,
    /// Verb–object–subject.
    Vos,
    /// Object–verb–subject.
    Ovs,
    /// Object–subject–verb.
    Osv,
    /// No dominant order — constituent order is governed by information structure
    /// rather than grammatical role. A real answer, not a missing one.
    Free,
}

impl WordOrder {
    /// The label as it appears in RON and in the sketch.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Sov => "SOV",
            Self::Svo => "SVO",
            Self::Vso => "VSO",
            Self::Vos => "VOS",
            Self::Ovs => "OVS",
            Self::Osv => "OSV",
            Self::Free => "free",
        }
    }

    /// Whether the object precedes the verb (`OV`) or follows it (`VO`).
    ///
    /// The **only** thing the harmony checks read from this field, because it is the
    /// only part the word-order correlations are about: Greenberg's universals turn
    /// on the verb–object relation, not on where the subject sits. Returning
    /// `Option` rather than defaulting is the point — `Free` and `Unspecified` have
    /// no answer, and inventing one would make every harmony warning below fire on
    /// languages that never made the claim.
    pub fn object_before_verb(self) -> Option<bool> {
        match self {
            Self::Sov | Self::Osv | Self::Ovs => Some(true),
            Self::Svo | Self::Vso | Self::Vos => Some(false),
            Self::Unspecified | Self::Free => None,
        }
    }
}

/// Where an adposition sits relative to its noun phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AdpositionOrder {
    /// Not stated.
    #[default]
    Unspecified,
    /// Before the noun phrase: *in the house*.
    Prepositions,
    /// After it: *the house in*.
    Postpositions,
    /// This language has no adpositions — case marking or serial verbs do the work.
    None,
}

/// Which comes first, the possessor or the thing possessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GenitiveOrder {
    /// Not stated.
    #[default]
    Unspecified,
    /// Possessor first: *the king's road*.
    GenitiveNoun,
    /// Possessed first: *the road of the king*.
    NounGenitive,
}

/// Which side of the noun an attributive adjective goes.
///
/// Kept separate from [`GenitiveOrder`] because the two do **not** correlate nearly
/// as strongly — adjective order is the classic example of a noun-phrase parameter
/// that goes its own way, and collapsing them into one "modifier order" field would
/// build a claim into the data model that the data does not support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AdjectiveOrder {
    /// Not stated.
    #[default]
    Unspecified,
    /// Before the noun: *the black stone*.
    AdjectiveNoun,
    /// After it: *the stone black*.
    NounAdjective,
}

/// How this language groups the single argument of an intransitive verb with the
/// arguments of a transitive one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Alignment {
    /// Not stated.
    #[default]
    Unspecified,
    /// The intransitive subject patterns with the transitive subject.
    NominativeAccusative,
    /// The intransitive subject patterns with the transitive object.
    ErgativeAbsolutive,
    /// All three are marked differently.
    Tripartite,
    /// The intransitive subject patterns with one or the other according to
    /// semantics — agency, volition, or aspect.
    ActiveStative,
    /// No case marking distinguishes them; order or agreement carries the load.
    Neutral,
}

/// Where a relative clause sits, and how it is joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RelativeClause {
    /// Not stated.
    #[default]
    Unspecified,
    /// Before the noun it modifies.
    Prenominal,
    /// After it.
    Postnominal,
    /// The head noun sits inside the relative clause itself.
    InternallyHeaded,
    /// A separate clause with a resumptive element in the main clause.
    Correlative,
}

/// How a clause is negated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Negation {
    /// Not stated.
    #[default]
    Unspecified,
    /// A free particle.
    Particle,
    /// An affix on the verb.
    Affix,
    /// A dedicated negative auxiliary verb.
    Auxiliary,
    /// Two elements bracketing the verb, French *ne … pas*.
    Discontinuous,
}

/// How a polar question is formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuestionFormation {
    /// Not stated.
    #[default]
    Unspecified,
    /// A question particle.
    Particle,
    /// Intonation alone.
    Intonation,
    /// A change of constituent order.
    WordOrder,
    /// Verbal morphology.
    VerbMorphology,
}

/// Whether a pronominal subject may be omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProDrop {
    /// Not stated.
    #[default]
    Unspecified,
    /// Subject pronouns are routinely dropped.
    Yes,
    /// They are obligatory.
    No,
}

/// Whether the grammar obliges a speaker to mark how they know what they assert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Evidentiality {
    /// Not stated.
    #[default]
    Unspecified,
    /// No grammaticalised evidentials.
    None,
    /// A two-way contrast, typically firsthand versus not.
    TwoWay,
    /// Three or more marked sources — witnessed, inferred, reported.
    Elaborated,
}

/// Whether a dependent clause marks that its subject is (or is not) the same as the
/// main clause's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SwitchReference {
    /// Not stated.
    #[default]
    Unspecified,
    /// Not grammaticalised.
    None,
    /// Marked on the verb of the dependent clause.
    Marked,
}

/// Which side of a clause the head sits on, taken over all the parameters that have
/// a side.
///
/// Derived, never stored — see the module docs. `Mixed` is a perfectly good answer
/// and a great many real languages are it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Headedness {
    /// Nothing stated has a side.
    Unknown,
    /// Every parameter that has a side puts the head first.
    HeadInitial,
    /// Every one puts the head last.
    HeadFinal,
    /// Some of each. Not a fault — English is mixed, and so is most of Europe.
    Mixed,
}

impl Headedness {
    /// The label, for the sketch.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::HeadInitial => "head-initial",
            Self::HeadFinal => "head-final",
            Self::Mixed => "mixed",
        }
    }
}

/// §7.4's parameters, as a stored description of one language's grammar.
///
/// Every field defaults to its `Unspecified` variant, so a pre-M17 file loads
/// unchanged and — more importantly — a language that has not been given a grammar
/// **says so** rather than silently claiming to be SVO. That is
/// `Phonotactics::default()`'s rule: a compiled-in default would make the answer
/// depend on the binary rather than on the file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SyntaxProfile {
    /// Authorial prose about the grammar as a whole. Not interpreted.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Dominant order of subject, object and verb.
    pub word_order: WordOrder,
    /// Where adpositions sit.
    pub adpositions: AdpositionOrder,
    /// Possessor and possessed.
    pub genitive: GenitiveOrder,
    /// Attributive adjectives.
    pub adjective: AdjectiveOrder,
    /// Case alignment.
    pub alignment: Alignment,
    /// Relative-clause strategy.
    pub relative_clause: RelativeClause,
    /// How a clause is negated.
    pub negation: Negation,
    /// How a polar question is formed.
    pub question: QuestionFormation,
    /// Whether subject pronouns may be dropped.
    pub pro_drop: ProDrop,
    /// Whether evidentiality is grammaticalised.
    pub evidentiality: Evidentiality,
    /// Whether switch-reference is marked.
    pub switch_reference: SwitchReference,
}

impl SyntaxProfile {
    /// True when nothing is stated — every pre-M17 file. `LanguageGenome` uses it
    /// for `skip_serializing_if`, so such a file round-trips byte-identically.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Which side the head sits on, over every parameter that has a side.
    ///
    /// The four that do: verb–object order, adposition placement, genitive order and
    /// relative-clause position. Adjective order is **excluded on purpose** — it is
    /// the classic parameter that does not track the others, and counting it would
    /// report half the world's languages as mixed for a reason that is not about
    /// headedness at all.
    pub fn headedness(&self) -> Headedness {
        let mut initial = 0usize;
        let mut final_ = 0usize;
        let mut count = |head_first: Option<bool>| match head_first {
            Some(true) => initial += 1,
            Some(false) => final_ += 1,
            None => {}
        };

        // In each pair the *head* is the verb, the adposition, the possessed noun,
        // and the noun a relative clause modifies.
        count(self.word_order.object_before_verb().map(|ov| !ov));
        count(match self.adpositions {
            AdpositionOrder::Prepositions => Some(true),
            AdpositionOrder::Postpositions => Some(false),
            _ => None,
        });
        count(match self.genitive {
            GenitiveOrder::NounGenitive => Some(true),
            GenitiveOrder::GenitiveNoun => Some(false),
            _ => None,
        });
        count(match self.relative_clause {
            RelativeClause::Postnominal => Some(true),
            RelativeClause::Prenominal => Some(false),
            _ => None,
        });

        match (initial, final_) {
            (0, 0) => Headedness::Unknown,
            (_, 0) => Headedness::HeadInitial,
            (0, _) => Headedness::HeadFinal,
            _ => Headedness::Mixed,
        }
    }
}

impl Validate for SyntaxProfile {
    /// Reports typological harmony. **Never an Error** — this whole impl produces
    /// Warnings and Notes, because every claim it makes is about what is *common*,
    /// and a rare language is a design rather than a fault.
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();
        if self.is_empty() {
            report.note(
                "unspecified",
                "this language has no syntax profile yet; `stemma grammar` will have \
                 nothing to describe",
            );
            return report;
        }

        // --- The two strong word-order correlations. ---
        //
        // Both are Greenberg's (1963), refined by Dryer's later cross-linguistic
        // work into statistical tendencies rather than absolutes. The messages say
        // which way the tendency runs and stop: no frequency is quoted, because none
        // could be verified from inside this program.
        if let Some(ov) = self.word_order.object_before_verb() {
            // Adpositions. The strongest of the word-order correlations, and the one
            // §7.4's own example names.
            match (ov, self.adpositions) {
                (false, AdpositionOrder::Postpositions) => report.push_issue(disharmony(
                    "vo_with_postpositions",
                    "verb-object order usually goes with prepositions, not postpositions \
                     (Greenberg's word-order universals); this combination is attested \
                     but uncommon",
                )),
                (true, AdpositionOrder::Prepositions) => report.push_issue(disharmony(
                    "ov_with_prepositions",
                    "object-verb order usually goes with postpositions, not prepositions; \
                     this combination is attested but uncommon",
                )),
                _ => {}
            }

            // Genitives. The same correlation, one step weaker but still a Warning.
            match (ov, self.genitive) {
                (false, GenitiveOrder::GenitiveNoun) => report.push_issue(disharmony(
                    "vo_with_genitive_first",
                    "verb-object order usually goes with noun-genitive (*the road of the \
                     king*); genitive-noun here is the rarer pairing",
                )),
                (true, GenitiveOrder::NounGenitive) => report.push_issue(disharmony(
                    "ov_with_genitive_last",
                    "object-verb order usually goes with genitive-noun (*the king's \
                     road*); noun-genitive here is the rarer pairing",
                )),
                _ => {}
            }

            // Relative clauses track headedness more weakly than either of the above,
            // so these are Notes. A report that sounded equally confident about claims
            // of different strength would be misinforming its reader.
            match (ov, self.relative_clause) {
                (false, RelativeClause::Prenominal) => report.note_issue(
                    "vo_with_prenominal_relatives",
                    "prenominal relative clauses are more usual in object-verb languages; \
                     with verb-object order they are less common, though far from unknown",
                ),
                (true, RelativeClause::Postnominal) => report.note_issue(
                    "ov_with_postnominal_relatives",
                    "postnominal relative clauses are more usual in verb-object languages; \
                     this pairing is a common enough mixture that it is worth no more than \
                     a note",
                ),
                _ => {}
            }
        }

        // --- Combinations that are not disharmonic, just worth saying out loud. ---
        if self.word_order == WordOrder::Free && self.alignment == Alignment::Neutral {
            report.push(disharmony(
                "free_order_without_case",
                "free constituent order with no case marking leaves nothing to identify \
                 who did what to whom; languages with free order almost always mark it \
                 somewhere — case, agreement, or both",
            ));
        }
        if self.pro_drop == ProDrop::Yes && self.alignment == Alignment::Neutral {
            report.note(
                "pro_drop_without_case",
                "dropping subject pronouns is easiest when the verb agrees with them; with \
                 neutral alignment and no agreement modelled, check that a listener can \
                 still recover the subject",
            );
        }

        // --- The summary line, so the profile states its own headedness. ---
        if self.headedness() == Headedness::Mixed {
            report.note(
                "mixed_headedness",
                "this language is head-initial in some constructions and head-final in \
                 others; that is ordinary — English and most of Europe are mixed — and it \
                 is recorded here because the grammar sketch will say so",
            );
        }

        report
    }
}

/// A harmony Warning. One constructor so every one of them has the same severity and
/// nobody can quietly promote one to an Error.
fn disharmony(code: &str, message: &str) -> Issue {
    Issue::new(Severity::Warning, code, message)
}

/// `ValidationReport::push` returns `&mut Self` for chaining and `note` returns `()`,
/// so a `match` whose arms use both will not type-check. This drops the borrow, which
/// keeps the harmony checks readable as the tables of correlations they are.
trait PushIssue {
    fn push_issue(&mut self, issue: Issue);
    fn note_issue(&mut self, code: &str, message: &str);
}
impl PushIssue for ValidationReport {
    fn push_issue(&mut self, issue: Issue) {
        self.push(issue);
    }
    fn note_issue(&mut self, code: &str, message: &str) {
        self.note(code, message);
    }
}

// ---------------------------------------------------------------- presentation
//
// Every parameter's label and plain-English gloss lives **here**, in the crate that
// defines the enums, and not in the renderer. Each of these types is
// `#[non_exhaustive]`, so a downstream `match` would need a wildcard arm — and a
// wildcard is exactly what stops a new variant from being a compile error at the
// place that has to learn about it. Inside this crate the match is checked, so adding
// `WordOrder::Vsо` breaks these functions and nothing else. `PartOfSpeech::name` and
// `Formation::summary` set the precedent.
//
// The gloss is not decoration: `ergative_absolutive` means nothing to a reader who
// does not already know, and a sketch that printed enum names would be a data dump
// rather than a description.

/// A parameter row, as the grammar sketch prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// What the parameter is called.
    pub label: &'static str,
    /// Its value, or `—` when nobody has decided.
    pub value: &'static str,
    /// A plain-English gloss, or empty where the value speaks for itself.
    pub gloss: &'static str,
}

/// The em-dash an unstated parameter prints as. **Printed, never skipped**: a sketch
/// that silently omitted its gaps would read as a complete grammar of a language that
/// is mostly undecided, which is the invisibility M15 built a whole milestone to
/// remove from the vocabulary.
pub const UNSTATED: &str = "—";

impl WordOrder {
    /// Its row.
    pub fn row(self) -> Row {
        let gloss = match self {
            Self::Unspecified => "not stated",
            Self::Free => "order carries information structure, not grammatical role",
            Self::Sov | Self::Osv | Self::Ovs => "object before verb",
            Self::Svo | Self::Vso | Self::Vos => "object after verb",
        };
        Row {
            label: "Word order",
            value: if self == Self::Unspecified {
                UNSTATED
            } else {
                self.name()
            },
            gloss,
        }
    }
}

impl AdpositionOrder {
    /// Its row.
    pub fn row(self) -> Row {
        let (value, gloss) = match self {
            Self::Unspecified => (UNSTATED, "not stated"),
            Self::Prepositions => ("prepositions", "*in the house*"),
            Self::Postpositions => ("postpositions", "*the house in*"),
            Self::None => ("none", "case or serial verbs do this work"),
        };
        Row {
            label: "Adpositions",
            value,
            gloss,
        }
    }
}

impl GenitiveOrder {
    /// Its row.
    pub fn row(self) -> Row {
        let (value, gloss) = match self {
            Self::Unspecified => (UNSTATED, "not stated"),
            Self::GenitiveNoun => ("genitive-noun", "*the king's road*"),
            Self::NounGenitive => ("noun-genitive", "*the road of the king*"),
        };
        Row {
            label: "Genitive",
            value,
            gloss,
        }
    }
}

impl AdjectiveOrder {
    /// Its row.
    pub fn row(self) -> Row {
        let (value, gloss) = match self {
            Self::Unspecified => (UNSTATED, "not stated"),
            Self::AdjectiveNoun => ("adjective-noun", "*the black stone*"),
            Self::NounAdjective => ("noun-adjective", "*the stone black*"),
        };
        Row {
            label: "Adjective",
            value,
            gloss,
        }
    }
}

impl Alignment {
    /// Its row.
    pub fn row(self) -> Row {
        let (value, gloss) = match self {
            Self::Unspecified => (UNSTATED, "not stated"),
            Self::NominativeAccusative => (
                "nominative-accusative",
                "the one who walks patterns with the one who hits",
            ),
            Self::ErgativeAbsolutive => (
                "ergative-absolutive",
                "the one who walks patterns with the one who is hit",
            ),
            Self::Tripartite => ("tripartite", "all three arguments marked differently"),
            Self::ActiveStative => (
                "active-stative",
                "grouping follows agency, not grammatical role",
            ),
            Self::Neutral => ("neutral", "no case distinguishes them"),
        };
        Row {
            label: "Alignment",
            value,
            gloss,
        }
    }
}

impl RelativeClause {
    /// Its row.
    pub fn row(self) -> Row {
        let (value, gloss) = match self {
            Self::Unspecified => (UNSTATED, "not stated"),
            Self::Prenominal => ("prenominal", "before the noun it modifies"),
            Self::Postnominal => ("postnominal", "after the noun it modifies"),
            Self::InternallyHeaded => ("internally headed", "the head noun sits inside the clause"),
            Self::Correlative => ("correlative", "a separate clause, resumed in the main one"),
        };
        Row {
            label: "Relative clause",
            value,
            gloss,
        }
    }
}

impl Negation {
    /// Its row.
    pub fn row(self) -> Row {
        let value = match self {
            Self::Unspecified => UNSTATED,
            Self::Particle => "particle",
            Self::Affix => "affix on the verb",
            Self::Auxiliary => "negative auxiliary",
            Self::Discontinuous => "discontinuous, bracketing the verb",
        };
        Row {
            label: "Negation",
            value,
            gloss: "",
        }
    }
}

impl QuestionFormation {
    /// Its row.
    pub fn row(self) -> Row {
        let value = match self {
            Self::Unspecified => UNSTATED,
            Self::Particle => "question particle",
            Self::Intonation => "intonation alone",
            Self::WordOrder => "a change of order",
            Self::VerbMorphology => "verbal morphology",
        };
        Row {
            label: "Questions",
            value,
            gloss: "",
        }
    }
}

impl ProDrop {
    /// Its row.
    pub fn row(self) -> Row {
        let value = match self {
            Self::Unspecified => UNSTATED,
            Self::Yes => "subject pronouns droppable",
            Self::No => "subject pronouns obligatory",
        };
        Row {
            label: "Pro-drop",
            value,
            gloss: "",
        }
    }
}

impl Evidentiality {
    /// Its row.
    pub fn row(self) -> Row {
        let value = match self {
            Self::Unspecified => UNSTATED,
            Self::None => "not grammaticalised",
            Self::TwoWay => "two-way",
            Self::Elaborated => "elaborated",
        };
        Row {
            label: "Evidentiality",
            value,
            gloss: "",
        }
    }
}

impl SwitchReference {
    /// Its row.
    pub fn row(self) -> Row {
        let value = match self {
            Self::Unspecified => UNSTATED,
            Self::None => "not grammaticalised",
            Self::Marked => "marked on the dependent verb",
        };
        Row {
            label: "Switch-reference",
            value,
            gloss: "",
        }
    }
}

impl SyntaxProfile {
    /// Every parameter as a printable row, in the order the sketch reads them.
    ///
    /// Headedness sits second, right under word order, because it is the summary the
    /// rest of the table explains — and it is labelled as **derived** so a reader is
    /// not left wondering whether it is a thirteenth parameter they could edit.
    pub fn rows(&self) -> Vec<Row> {
        vec![
            self.word_order.row(),
            Row {
                label: "Headedness",
                value: self.headedness().name(),
                gloss: "derived from the orders below, never stored",
            },
            self.adpositions.row(),
            self.genitive.row(),
            self.adjective.row(),
            self.alignment.row(),
            self.relative_clause.row(),
            self.negation.row(),
            self.question.row(),
            self.pro_drop.row(),
            self.evidentiality.row(),
            self.switch_reference.row(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harmonic_head_final() -> SyntaxProfile {
        SyntaxProfile {
            word_order: WordOrder::Sov,
            adpositions: AdpositionOrder::Postpositions,
            genitive: GenitiveOrder::GenitiveNoun,
            relative_clause: RelativeClause::Prenominal,
            alignment: Alignment::ErgativeAbsolutive,
            ..SyntaxProfile::default()
        }
    }

    fn harmonic_head_initial() -> SyntaxProfile {
        SyntaxProfile {
            word_order: WordOrder::Vso,
            adpositions: AdpositionOrder::Prepositions,
            genitive: GenitiveOrder::NounGenitive,
            relative_clause: RelativeClause::Postnominal,
            alignment: Alignment::NominativeAccusative,
            ..SyntaxProfile::default()
        }
    }

    #[test]
    fn an_unstated_profile_is_empty_and_says_so_rather_than_claiming_svo() {
        let profile = SyntaxProfile::default();
        assert!(profile.is_empty());
        assert_eq!(profile.word_order, WordOrder::Unspecified);
        assert_eq!(profile.headedness(), Headedness::Unknown);
        let report = profile.validate();
        assert!(report.issues.iter().any(|i| i.code == "unspecified"));
        assert!(report.is_ok(), "having no grammar yet is not a fault");
    }

    #[test]
    fn headedness_is_derived_from_the_parameters_that_have_a_side() {
        assert_eq!(harmonic_head_final().headedness(), Headedness::HeadFinal);
        assert_eq!(
            harmonic_head_initial().headedness(),
            Headedness::HeadInitial
        );
    }

    /// Adjective order is excluded on purpose: it is the parameter that famously does
    /// not track the others, so counting it would report half the world as mixed for
    /// a reason that is not about headedness.
    #[test]
    fn adjective_order_does_not_affect_headedness() {
        let mut profile = harmonic_head_final();
        profile.adjective = AdjectiveOrder::NounAdjective;
        assert_eq!(profile.headedness(), Headedness::HeadFinal);
        profile.adjective = AdjectiveOrder::AdjectiveNoun;
        assert_eq!(profile.headedness(), Headedness::HeadFinal);
    }

    #[test]
    fn a_language_with_a_side_in_each_direction_is_mixed() {
        let mut profile = harmonic_head_final();
        profile.adpositions = AdpositionOrder::Prepositions;
        assert_eq!(profile.headedness(), Headedness::Mixed);
    }

    /// `Free` and `Unspecified` have no verb–object answer, so they must not be
    /// counted — otherwise every harmony check would fire on a language that never
    /// made the claim.
    #[test]
    fn free_and_unspecified_word_orders_have_no_object_verb_answer() {
        assert_eq!(WordOrder::Free.object_before_verb(), None);
        assert_eq!(WordOrder::Unspecified.object_before_verb(), None);
        assert_eq!(WordOrder::Sov.object_before_verb(), Some(true));
        assert_eq!(WordOrder::Vos.object_before_verb(), Some(false));
    }

    // --- harmony, reported not enforced ---

    #[test]
    fn a_harmonic_profile_earns_no_warning() {
        for profile in [harmonic_head_final(), harmonic_head_initial()] {
            let report = profile.validate();
            assert!(
                report.warnings().next().is_none(),
                "a harmonic language should be quiet: {report}"
            );
            assert!(report.is_ok());
        }
    }

    /// **ROADMAP M17's acceptance.** A harmonically odd combination earns a Warning
    /// and still validates.
    #[test]
    fn a_vo_language_with_postpositions_warns_and_still_validates() {
        let profile = SyntaxProfile {
            adpositions: AdpositionOrder::Postpositions,
            ..harmonic_head_initial()
        };
        let report = profile.validate();
        let issue = report
            .warnings()
            .find(|i| i.code == "vo_with_postpositions")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(
            issue.message.contains("uncommon"),
            "the message must say it is rare, not wrong: {}",
            issue.message
        );
        assert!(
            report.is_ok(),
            "rare is not forbidden — §17, and CLAUDE.md's `resist the urge to reject a \
             weird language`"
        );
    }

    #[test]
    fn an_ov_language_with_prepositions_warns_the_other_way() {
        let profile = SyntaxProfile {
            adpositions: AdpositionOrder::Prepositions,
            ..harmonic_head_final()
        };
        assert!(
            profile
                .validate()
                .warnings()
                .any(|i| i.code == "ov_with_prepositions")
        );
    }

    #[test]
    fn the_genitive_correlation_is_reported_in_both_directions() {
        let vo = SyntaxProfile {
            genitive: GenitiveOrder::GenitiveNoun,
            ..harmonic_head_initial()
        };
        assert!(
            vo.validate()
                .warnings()
                .any(|i| i.code == "vo_with_genitive_first")
        );

        let ov = SyntaxProfile {
            genitive: GenitiveOrder::NounGenitive,
            ..harmonic_head_final()
        };
        assert!(
            ov.validate()
                .warnings()
                .any(|i| i.code == "ov_with_genitive_last")
        );
    }

    /// The relative-clause correlation is weaker than the adposition one, so it is a
    /// **Note**. A report that sounded equally confident about claims of different
    /// strength would be misinforming its reader.
    #[test]
    fn the_weaker_relative_clause_correlation_is_only_a_note() {
        let profile = SyntaxProfile {
            relative_clause: RelativeClause::Prenominal,
            ..harmonic_head_initial()
        };
        let report = profile.validate();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "vo_with_prenominal_relatives" && i.severity == Severity::Note),
            "{report}"
        );
        assert!(
            !report
                .warnings()
                .any(|i| i.code == "vo_with_prenominal_relatives"),
            "a weaker tendency must not be reported at Warning severity"
        );
    }

    #[test]
    fn free_order_with_no_case_marking_is_reported() {
        let profile = SyntaxProfile {
            word_order: WordOrder::Free,
            alignment: Alignment::Neutral,
            ..SyntaxProfile::default()
        };
        let report = profile.validate();
        assert!(
            report
                .warnings()
                .any(|i| i.code == "free_order_without_case"),
            "{report}"
        );
        assert!(report.is_ok());
    }

    /// The whole impl is Warnings and Notes. An Error here would mean Stemma refusing
    /// to model a language somebody wants, which is the thing `CLAUDE.md` forbids in
    /// as many words.
    #[test]
    fn no_syntactic_combination_is_ever_an_error() {
        let orders = [
            WordOrder::Sov,
            WordOrder::Svo,
            WordOrder::Vso,
            WordOrder::Vos,
            WordOrder::Ovs,
            WordOrder::Osv,
            WordOrder::Free,
            WordOrder::Unspecified,
        ];
        let adpositions = [
            AdpositionOrder::Prepositions,
            AdpositionOrder::Postpositions,
            AdpositionOrder::None,
            AdpositionOrder::Unspecified,
        ];
        let genitives = [
            GenitiveOrder::GenitiveNoun,
            GenitiveOrder::NounGenitive,
            GenitiveOrder::Unspecified,
        ];
        let alignments = [
            Alignment::NominativeAccusative,
            Alignment::ErgativeAbsolutive,
            Alignment::Tripartite,
            Alignment::ActiveStative,
            Alignment::Neutral,
            Alignment::Unspecified,
        ];

        let mut checked = 0usize;
        for word_order in orders {
            for adposition in adpositions {
                for genitive in genitives {
                    for alignment in alignments {
                        let profile = SyntaxProfile {
                            word_order,
                            adpositions: adposition,
                            genitive,
                            alignment,
                            ..SyntaxProfile::default()
                        };
                        let report = profile.validate();
                        assert!(
                            report.is_ok(),
                            "{word_order:?}/{adposition:?}/{genitive:?}/{alignment:?} was \
                             refused: {report}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, 8 * 4 * 3 * 6);
    }

    #[test]
    fn no_harmony_message_quotes_a_frequency_it_cannot_support() {
        // A message with a digit in it is almost certainly a fabricated statistic.
        // The tendencies here are real; the numbers would not be.
        let profile = SyntaxProfile {
            adpositions: AdpositionOrder::Postpositions,
            genitive: GenitiveOrder::GenitiveNoun,
            relative_clause: RelativeClause::Prenominal,
            ..harmonic_head_initial()
        };
        for issue in &profile.validate().issues {
            assert!(
                !issue.message.chars().any(|c| c.is_ascii_digit()),
                "`{}` quotes a number this program cannot verify: {}",
                issue.code,
                issue.message
            );
        }
    }

    // --- round trip ---

    #[test]
    fn a_profile_round_trips_through_ron() {
        let profile = harmonic_head_final();
        let text = ron::ser::to_string(&profile).expect("serialise");
        let back: SyntaxProfile = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, profile);
    }

    /// `#[serde(default)]` on the struct: a file may state one parameter and leave
    /// the rest unspecified, which is how an author fills a grammar in over time.
    #[test]
    fn a_partial_profile_loads_with_the_rest_unspecified() {
        let profile: SyntaxProfile = ron::from_str(r#"(word_order: sov)"#).expect("loads");
        assert_eq!(profile.word_order, WordOrder::Sov);
        assert_eq!(profile.adpositions, AdpositionOrder::Unspecified);
        assert!(!profile.is_empty(), "one stated parameter is not empty");
    }

    #[test]
    fn a_misspelled_parameter_fails_to_load_rather_than_defaulting() {
        assert!(ron::from_str::<SyntaxProfile>(r#"(word_ordr: sov)"#).is_err());
        assert!(
            ron::from_str::<SyntaxProfile>(r#"(word_order: svp)"#).is_err(),
            "an unknown *value* must fail too, or a typo silently means `unspecified`"
        );
    }
}
