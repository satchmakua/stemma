//! The first sentence (ROADMAP M18, `DESIGN.md` §7.4, §3.3).
//!
//! # What this is
//!
//! A **proposition** plus a [`SyntaxProfile`] yields an ordered, inflected sentence
//! made of words that already exist in the language's lexicon — so every word in it
//! already has a sound-change history, an etymology, and a cognate set. Nothing here
//! coins a word or invents a form; it arranges and inflects what is already there.
//!
//! # §3.3 applies unchanged
//!
//! *Every generated form must have a recorded causal history.* A word carries a
//! `Derivation`; a sentence carries a [`Vec<Construction>`], and it is produced the
//! same way — the generator emits a record as it goes, not afterwards from the
//! output. A sentence whose word order came out right with no `Clause` construction
//! recorded is a bug even when the string is correct, exactly as a sound change
//! without a `RuleApplicationTrace` is.
//!
//! That is why [`Sentence`] stores its words as *slots* naming a [`WordId`] and a
//! [`CognateSetId`] rather than as rendered strings: the string is a view, and a
//! stored one would desynchronise the first time anything changed
//! (`docs/adr/0007`, and `WordEntry::phonemic_form`'s own rule).
//!
//! # The scope fence
//!
//! §20.1 names scope explosion as the top risk and this is the milestone most able
//! to cause it. v0 generates **one clause**: a predicate and up to two arguments,
//! each of which may carry one adjective and one possessor. There is no recursion, no
//! subordination, no coordination, no tense, no agreement, no relative clause — even
//! though [`SyntaxProfile`] records a relative-clause strategy, because recording a
//! parameter and building an engine for it are different milestones (M17 and this
//! one, and the ordering was deliberate).
//!
//! There is also **no parser for natural language**. `SEE(KING, STAR)` is a
//! proposition in a notation of about forty lines; understanding English is not on
//! this roadmap at all.

use std::fmt;

use stem_core::{CognateSetId, Result, StemmaError, WordId};
use stem_lexicon::{ConceptKey, Lexicon, Morpheme, MorphemeRef, MorphemeRole, Morphology, compose};
use stem_phonology::Root;

use crate::{Alignment, SyntaxProfile, WordOrder};

// ------------------------------------------------------------- the proposition

/// One argument of a proposition: a head concept, optionally modified.
///
/// `STAR`, `STAR:BIG` (adjective), `KING/PRIEST` (possessor), `KING:OLD/PRIEST`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    /// The head noun's concept.
    pub head: ConceptKey,
    /// An attributive adjective, by concept.
    pub adjective: Option<ConceptKey>,
    /// A possessor, by concept.
    pub possessor: Option<ConceptKey>,
}

impl Argument {
    /// A bare head with no modifiers.
    pub fn bare(head: impl Into<String>) -> Self {
        Self {
            head: ConceptKey::new(head),
            adjective: None,
            possessor: None,
        }
    }
}

/// A meaning to express: who did what to whom.
///
/// **Language-neutral by construction** — it names concepts, not words, so the same
/// proposition can be put through two languages and the difference in the output is
/// entirely theirs. That is M18's acceptance, and it is the reason the proposition is
/// built out of [`ConceptKey`]s rather than [`WordId`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposition {
    /// The predicate — the verb.
    pub predicate: ConceptKey,
    /// The agent, if there is one.
    pub agent: Option<Argument>,
    /// The patient, if there is one. A proposition with an agent and no patient is
    /// intransitive, which is what makes the alignment distinction visible.
    pub patient: Option<Argument>,
}

impl Proposition {
    /// Whether this proposition has two arguments.
    ///
    /// The **only** thing [`Alignment`] reads, because that is what alignment is
    /// about: how the single argument of an intransitive verb is grouped with the two
    /// of a transitive one.
    pub fn is_transitive(&self) -> bool {
        self.agent.is_some() && self.patient.is_some()
    }

    /// Parses `SEE(KING, STAR)`.
    ///
    /// # The notation
    ///
    /// ```text
    /// PREDICATE(ARG, ARG)      a transitive clause
    /// PREDICATE(ARG)           an intransitive one
    /// ARG   :=  CONCEPT [":" ADJECTIVE] ["/" POSSESSOR]
    /// ```
    ///
    /// Concept keys, uppercase, exactly as they appear in a lexicon — so the notation
    /// needs no dictionary of its own and cannot drift from one. `:` and `/` are both
    /// shell-safe unquoted, which matters for a thing whose whole purpose is to be
    /// typed at a prompt.
    ///
    /// # Failure
    ///
    /// Every parse error names the offending text and what was expected. This is read
    /// from a command line by a person, so "invalid proposition" would be useless.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        let open = text
            .find('(')
            .ok_or_else(|| invalid(text, "expected `PREDICATE(ARGUMENT, …)` — no `(` found"))?;
        if !text.ends_with(')') {
            return Err(invalid(text, "expected a closing `)`"));
        }

        let predicate = text[..open].trim();
        if predicate.is_empty() {
            return Err(invalid(text, "no predicate before the `(`"));
        }

        let inside = text[open + 1..text.len() - 1].trim();
        let mut arguments: Vec<Argument> = Vec::new();
        if !inside.is_empty() {
            for part in inside.split(',') {
                arguments.push(parse_argument(part.trim(), text)?);
            }
        }
        if arguments.len() > 2 {
            return Err(invalid(
                text,
                "v0 generates one clause with at most two arguments (agent, patient); \
                 ditransitives and obliques are not modelled yet",
            ));
        }

        let mut arguments = arguments.into_iter();
        Ok(Self {
            predicate: ConceptKey::new(predicate),
            agent: arguments.next(),
            patient: arguments.next(),
        })
    }
}

fn parse_argument(text: &str, whole: &str) -> Result<Argument> {
    if text.is_empty() {
        return Err(invalid(whole, "an empty argument"));
    }
    // Possessor first, so `KING:OLD/PRIEST` splits the way it reads.
    let (before_slash, possessor) = match text.split_once('/') {
        Some((head, owner)) => (head.trim(), Some(ConceptKey::new(owner.trim()))),
        None => (text, None),
    };
    let (head, adjective) = match before_slash.split_once(':') {
        Some((head, adjective)) => (head.trim(), Some(ConceptKey::new(adjective.trim()))),
        None => (before_slash, None),
    };
    if head.is_empty() {
        return Err(invalid(whole, "an argument with no head noun"));
    }
    Ok(Argument {
        head: ConceptKey::new(head),
        adjective,
        possessor,
    })
}

fn invalid(text: &str, why: &str) -> StemmaError {
    let mut report = stem_core::ValidationReport::new();
    report.push(stem_core::Issue::new(
        stem_core::Severity::Error,
        "unreadable_proposition",
        why.to_owned(),
    ));
    StemmaError::Invalid(format!("the proposition `{text}`"), report)
}

impl fmt::Display for Proposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.predicate)?;
        let mut first = true;
        for argument in [&self.agent, &self.patient].into_iter().flatten() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{}", argument.head)?;
            if let Some(adjective) = &argument.adjective {
                write!(f, ":{adjective}")?;
            }
            if let Some(possessor) = &argument.possessor {
                write!(f, "/{possessor}")?;
            }
        }
        write!(f, ")")
    }
}

// ------------------------------------------------------------- the constructions

/// One construction the generator applied, and the parameter that decided it.
///
/// The §3.3 record for a sentence. Every one names **which profile parameter** made
/// the decision, so "why is the verb last?" has the same kind of answer "why does
/// this word start with /t/?" has had since M3: because this rule, from this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Construction {
    /// A stable id, e.g. `"clause"`, `"noun_phrase"`, `"case_marking"`.
    pub id: &'static str,
    /// What it did, in words.
    pub effect: String,
    /// The syntax parameter that decided it, named as the field an author would edit.
    pub because: String,
}

/// One word of a generated sentence, with what it came from.
///
/// Holds the lexicon entry's **identity**, never a rendered string: the surface is a
/// view over `form`, and a stored string would desynchronise the moment anything
/// upstream changed (`docs/adr/0007`). `cognate_set` is echoed for the reason
/// [`stem_lexicon::BaseRef`] echoes one — a sentence stays legible across a family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// The lexicon entry this word is.
    pub word: WordId,
    /// Its descent class, echoed so a sentence is comparable across a family.
    pub cognate_set: CognateSetId,
    /// Its grammatical role: `agent`, `predicate`, `patient`, `adjective`,
    /// `possessor`.
    pub role: &'static str,
    /// The composed form — the stem plus whatever affixes were applied.
    pub form: Root,
    /// Which morphemes were attached, with their spans. Empty when nothing inflected
    /// it, exactly as it is for an uninflected word (M8's record, reused).
    pub morphemes: Vec<MorphemeRef>,
}

/// A generated sentence: its words in order, and the record of what built it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The proposition it expresses, kept so the record is self-contained.
    pub proposition: Proposition,
    /// The words, in surface order.
    pub slots: Vec<Slot>,
    /// The constructions applied, in the order they were applied.
    pub constructions: Vec<Construction>,
    /// Anything the generator could not do and did not fake — a case this language
    /// has no marker for, a concept its lexicon has no word for.
    ///
    /// **Reported, never silently skipped.** A sentence that quietly dropped its case
    /// marking would look like a language that has none, which is the invisibility
    /// M15 built a milestone to remove from the vocabulary.
    pub gaps: Vec<String>,
}

impl Sentence {
    /// The sentence as a written string, one word per slot.
    pub fn written(&self, inventory: &stem_phonology::PhonemeInventory) -> Result<String> {
        let mut words = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            words.push(slot.form.written(inventory)?);
        }
        Ok(words.join(" "))
    }
}

// ---------------------------------------------------------------- the generator

/// Builds a sentence from a proposition, a syntax profile, and a lexicon.
///
/// Pure, total and **RNG-free** — the same claim `apply_rules` makes and for the same
/// reason: two runs must be byte-identical, and a generator that chose anything at
/// random could not be. Every decision comes from the profile.
///
/// # What it does, in order
///
/// 1. Resolves each concept to a word in `lexicon`.
/// 2. Builds each argument's noun phrase, ordering modifiers by
///    [`SyntaxProfile::adjective`] and [`SyntaxProfile::genitive`].
/// 3. Marks case per [`SyntaxProfile::alignment`], using morphemes the language
///    declares.
/// 4. Orders the clause per [`SyntaxProfile::word_order`].
///
/// Each step appends to `constructions`, so the record is produced *by* the
/// generation rather than reconstructed from its output.
///
/// # Failure
///
/// A concept the lexicon has no word for is a [`StemmaError::NotFound`] naming it —
/// there is no sentence to be had, and inventing a word would be the fabrication this
/// project exists to avoid. Everything else the generator cannot do is a **gap**
/// recorded on the sentence, not an error.
pub fn generate(
    proposition: &Proposition,
    profile: &SyntaxProfile,
    lexicon: &Lexicon,
    morphology: &Morphology,
) -> Result<Sentence> {
    let mut constructions = Vec::new();
    let mut gaps = Vec::new();

    // --- 1 & 2: the noun phrases, each already internally ordered. ---
    let agent = proposition
        .agent
        .as_ref()
        .map(|a| noun_phrase(a, profile, lexicon, "agent", &mut constructions))
        .transpose()?;
    let patient = proposition
        .patient
        .as_ref()
        .map(|a| noun_phrase(a, profile, lexicon, "patient", &mut constructions))
        .transpose()?;

    let verb = resolve(lexicon, &proposition.predicate)?;
    let predicate = vec![Slot {
        word: verb.id.clone(),
        cognate_set: verb.cognate_set.clone(),
        role: "predicate",
        form: verb.phonemic_form.clone(),
        morphemes: Vec::new(),
    }];

    // --- 3: case marking, which is what alignment *is*. ---
    let (agent, patient) = mark_case(
        agent,
        patient,
        proposition.is_transitive(),
        profile,
        morphology,
        &mut constructions,
        &mut gaps,
    );

    // --- 4: the clause. ---
    let slots = order_clause(agent, predicate, patient, profile, &mut constructions);

    Ok(Sentence {
        proposition: proposition.clone(),
        slots,
        constructions,
        gaps,
    })
}

/// Resolves a concept to the word that realises it, first in stored order.
///
/// `by_concept().first()` is `by_meaning`'s policy verbatim — the two resolvers must
/// not disagree about which word a meaning means when a language has synonyms (which
/// M15's elaborations make routine).
fn resolve<'a>(lexicon: &'a Lexicon, key: &ConceptKey) -> Result<&'a stem_lexicon::WordEntry> {
    lexicon
        .by_concept(key)
        .first()
        .copied()
        .ok_or_else(|| StemmaError::not_found("word for concept", key))
}

/// Builds one argument's noun phrase, in this language's own order.
fn noun_phrase(
    argument: &Argument,
    profile: &SyntaxProfile,
    lexicon: &Lexicon,
    role: &'static str,
    constructions: &mut Vec<Construction>,
) -> Result<Vec<Slot>> {
    let head = resolve(lexicon, &argument.head)?;
    let mut before: Vec<Slot> = Vec::new();
    let mut after: Vec<Slot> = Vec::new();

    let slot = |entry: &stem_lexicon::WordEntry, role: &'static str| Slot {
        word: entry.id.clone(),
        cognate_set: entry.cognate_set.clone(),
        role,
        form: entry.phonemic_form.clone(),
        morphemes: Vec::new(),
    };

    if let Some(key) = &argument.adjective {
        let adjective = resolve(lexicon, key)?;
        let (side, where_) = match profile.adjective {
            crate::AdjectiveOrder::NounAdjective => (&mut after, "after"),
            // Unspecified falls to adjective-first: it is the commoner order, and the
            // construction record says the parameter was unstated, so the choice is
            // visible rather than silently assumed.
            _ => (&mut before, "before"),
        };
        side.push(slot(adjective, "adjective"));
        constructions.push(Construction {
            id: "noun_phrase",
            effect: format!(
                "put the adjective `{}` {where_} the {role} `{}`",
                key, argument.head
            ),
            because: format!("adjective = {}", stated(profile.adjective.row().value)),
        });
    }

    if let Some(key) = &argument.possessor {
        let possessor = resolve(lexicon, key)?;
        let (side, where_) = match profile.genitive {
            crate::GenitiveOrder::NounGenitive => (&mut after, "after"),
            _ => (&mut before, "before"),
        };
        side.push(slot(possessor, "possessor"));
        constructions.push(Construction {
            id: "noun_phrase",
            effect: format!(
                "put the possessor `{}` {where_} the {role} `{}`",
                key, argument.head
            ),
            because: format!("genitive = {}", stated(profile.genitive.row().value)),
        });
    }

    let mut phrase = before;
    phrase.push(slot(head, role));
    phrase.extend(after);
    Ok(phrase)
}

/// Applies case marking to the two arguments, per this language's alignment.
///
/// This is the parameter with the most visible consequence, and the one that makes
/// the acceptance's "two daughters differ *because* their profiles differ" a claim
/// about grammar rather than about word order alone: an ergative language marks the
/// agent of a transitive clause and leaves the intransitive one bare, and a
/// nominative one does the opposite.
///
/// A language that declares no morpheme for a case it needs gets a **gap**, not a
/// fabricated affix.
#[allow(clippy::too_many_arguments)]
fn mark_case(
    agent: Option<Vec<Slot>>,
    patient: Option<Vec<Slot>>,
    transitive: bool,
    profile: &SyntaxProfile,
    morphology: &Morphology,
    constructions: &mut Vec<Construction>,
    gaps: &mut Vec<String>,
) -> (Option<Vec<Slot>>, Option<Vec<Slot>>) {
    // Which case each argument takes. `None` means "this alignment marks nothing
    // here", which is a real answer and not a missing one.
    let (agent_case, patient_case) = match (profile.alignment, transitive) {
        (Alignment::NominativeAccusative, true) => (Some("NOM"), Some("ACC")),
        (Alignment::NominativeAccusative, false) => (Some("NOM"), None),
        (Alignment::ErgativeAbsolutive, true) => (Some("ERG"), Some("ABS")),
        // The single argument of an intransitive verb takes the *absolutive* — that
        // is the whole content of the term.
        (Alignment::ErgativeAbsolutive, false) => (Some("ABS"), None),
        (Alignment::Tripartite, true) => (Some("ERG"), Some("ACC")),
        (Alignment::Tripartite, false) => (Some("NOM"), None),
        _ => (None, None),
    };

    let mut apply = |phrase: Option<Vec<Slot>>, case: Option<&str>, role: &str| {
        let mut phrase = phrase?;
        // **Not `case?`.** A `?` here returns `None` for the whole closure and
        // therefore deletes the noun phrase — an alignment that marks nothing would
        // silently produce a sentence with no arguments in it. Marking nothing means
        // the phrase passes through untouched, which is what `Neutral` *is*.
        let Some(case) = case else {
            return Some(phrase);
        };
        match find_case_morpheme(morphology, case) {
            Some(morpheme) => {
                // The head of the phrase carries the case. Which word in a phrase
                // hosts a case marker is itself a typological parameter in the real
                // world; v0 puts it on the head and says so rather than pretending
                // the question does not exist.
                if let Some(head) = phrase.iter_mut().find(|s| s.role == role) {
                    let stem = as_stem(head);
                    let (form, refs) = compose(&stem, &[morpheme]);
                    head.form = form;
                    head.morphemes = refs;
                }
                constructions.push(Construction {
                    id: "case_marking",
                    effect: format!("marked the {role} `{case}`"),
                    because: format!("alignment = {}", stated(profile.alignment.row().value)),
                });
            }
            None => gaps.push(format!(
                "this language declares no `{case}` morpheme, so the {role} is unmarked; \
                 declare one in `morphology.morphemes` with gloss \"{case}\""
            )),
        }
        Some(phrase)
    };

    let agent = apply(agent, agent_case, "agent");
    let patient = apply(patient, patient_case, "patient");
    (agent, patient)
}

/// The morpheme this language uses for `case`, matched on its gloss.
///
/// M8 defined an affix's `gloss` as "a feature label (`PL`, `PST`)", so `ERG` is
/// where an ergative marker lives. Matching on it rather than adding a typed `case`
/// field keeps the morpheme model exactly as small as M8 left it — and a typed field
/// would need a closed enum of cases, which is a taxonomy this project has no reason
/// to compile in before something needs it.
fn find_case_morpheme<'a>(morphology: &'a Morphology, case: &str) -> Option<&'a Morpheme> {
    morphology
        .morphemes
        .iter()
        .find(|m| m.role != MorphemeRole::Stem && m.gloss == case)
}

/// A slot as a `Morpheme`, so [`compose`] can lay it out.
///
/// `compose` takes morphemes because M8 built it for inflection; wrapping the slot is
/// what lets a sentence reuse the *one* composition kernel rather than growing a
/// second one. M14 made the same call from the other direction (`lay_out`).
fn as_stem(slot: &Slot) -> Morpheme {
    Morpheme {
        id: stem_core::MorphemeId::new(slot.word.as_str()),
        role: MorphemeRole::Stem,
        gloss: slot.role.to_owned(),
        form: slot.form.clone(),
        part_of_speech: stem_lexicon::PartOfSpeech::Noun,
    }
}

/// Orders agent, predicate and patient into one clause.
fn order_clause(
    agent: Option<Vec<Slot>>,
    predicate: Vec<Slot>,
    patient: Option<Vec<Slot>>,
    profile: &SyntaxProfile,
    constructions: &mut Vec<Construction>,
) -> Vec<Slot> {
    let subject = agent.unwrap_or_default();
    let object = patient.unwrap_or_default();

    // The order is read straight off the parameter. `Free` and `Unspecified` fall to
    // SVO — the commonest order — and the construction record *says* the parameter
    // was not stated, so the fallback is visible rather than a silent claim.
    let order = match profile.word_order {
        WordOrder::Sov => [Part::Subject, Part::Object, Part::Verb],
        WordOrder::Svo => [Part::Subject, Part::Verb, Part::Object],
        WordOrder::Vso => [Part::Verb, Part::Subject, Part::Object],
        WordOrder::Vos => [Part::Verb, Part::Object, Part::Subject],
        WordOrder::Ovs => [Part::Object, Part::Verb, Part::Subject],
        WordOrder::Osv => [Part::Object, Part::Subject, Part::Verb],
        _ => [Part::Subject, Part::Verb, Part::Object],
    };

    constructions.push(Construction {
        id: "clause",
        effect: format!(
            "ordered the clause {}",
            match profile.word_order {
                WordOrder::Unspecified | WordOrder::Free => "SVO",
                other => other.name(),
            }
        ),
        because: format!("word_order = {}", stated(profile.word_order.row().value)),
    });

    let mut slots = Vec::new();
    for part in order {
        match part {
            Part::Subject => slots.extend(subject.iter().cloned()),
            Part::Verb => slots.extend(predicate.iter().cloned()),
            Part::Object => slots.extend(object.iter().cloned()),
        }
    }
    slots
}

#[derive(Clone, Copy)]
enum Part {
    Subject,
    Verb,
    Object,
}

/// A parameter's value for a `because` line, saying so when it was not stated.
///
/// An unstated parameter still produced a decision, and the record has to be honest
/// about which — otherwise "because word_order = —" reads as though the profile chose
/// the order, when in fact the generator fell back.
fn stated(value: &str) -> String {
    if value == crate::UNSTATED {
        "not stated; fell back to the commonest order".to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdjectiveOrder, GenitiveOrder};
    use stem_lexicon::{PartOfSpeech, WordEntry, WordSource, scoped_cognate_set};
    use stem_phonology::{Phoneme, PhonemeInventory, SegmentKind, Syllable};

    fn syllable(segments: &[&str]) -> Syllable {
        Syllable {
            pattern: "X".to_owned(),
            segments: segments
                .iter()
                .map(|s| stem_core::PhonemeId::new(*s))
                .collect(),
            stress: None,
        }
    }

    fn inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel),
            Phoneme::new("ph_r", "r", SegmentKind::Consonant),
            Phoneme::new("ph_n", "n", SegmentKind::Consonant),
            Phoneme::new("ph_s", "s", SegmentKind::Consonant),
            Phoneme::new("ph_u", "u", SegmentKind::Vowel),
        ])
    }

    fn word(ordinal: usize, concept: &str, segments: &[&str], pos: PartOfSpeech) -> WordEntry {
        WordEntry {
            id: WordId::sequential(ordinal),
            concept: Some(ConceptKey::new(concept)),
            phonemic_form: Root {
                syllables: vec![syllable(segments)],
            },
            glosses: Vec::new(),
            part_of_speech: pos,
            cognate_set: scoped_cognate_set(&stem_core::LanguageId::new("x"), ordinal),
            source: WordSource::Generated,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: Vec::new(),
            sense_history: None,
        }
    }

    /// `KING` = tak, `STAR` = kir, `SEE` = san, `BIG` = tu, `PRIEST` = nan.
    fn lexicon() -> Lexicon {
        Lexicon::from_entries([
            word(1, "KING", &["ph_t", "ph_a", "ph_k"], PartOfSpeech::Noun),
            word(2, "STAR", &["ph_k", "ph_i", "ph_r"], PartOfSpeech::Noun),
            word(3, "SEE", &["ph_s", "ph_a", "ph_n"], PartOfSpeech::Verb),
            word(4, "BIG", &["ph_t", "ph_u"], PartOfSpeech::Adjective),
            word(5, "PRIEST", &["ph_n", "ph_a", "ph_n"], PartOfSpeech::Noun),
        ])
    }

    /// `-ir` ergative, `-a` absolutive.
    fn morphology() -> Morphology {
        let affix = |id: &str, gloss: &str, segments: &[&str]| Morpheme {
            id: stem_core::MorphemeId::new(id),
            role: MorphemeRole::Suffix,
            gloss: gloss.to_owned(),
            form: Root {
                syllables: vec![syllable(segments)],
            },
            part_of_speech: PartOfSpeech::Noun,
        };
        Morphology {
            morphemes: vec![
                affix("m_erg", "ERG", &["ph_i", "ph_r"]),
                affix("m_abs", "ABS", &["ph_a"]),
                affix("m_nom", "NOM", &["ph_u"]),
                affix("m_acc", "ACC", &["ph_t", "ph_a"]),
            ],
            paradigms: Vec::new(),
            derivations: Vec::new(),
        }
    }

    fn sov() -> SyntaxProfile {
        SyntaxProfile {
            word_order: WordOrder::Sov,
            adjective: AdjectiveOrder::AdjectiveNoun,
            genitive: GenitiveOrder::GenitiveNoun,
            alignment: Alignment::ErgativeAbsolutive,
            ..SyntaxProfile::default()
        }
    }

    fn svo() -> SyntaxProfile {
        SyntaxProfile {
            word_order: WordOrder::Svo,
            adjective: AdjectiveOrder::NounAdjective,
            genitive: GenitiveOrder::NounGenitive,
            alignment: Alignment::NominativeAccusative,
            ..SyntaxProfile::default()
        }
    }

    fn say(text: &str, profile: &SyntaxProfile) -> Sentence {
        let proposition = Proposition::parse(text).expect("parses");
        generate(&proposition, profile, &lexicon(), &morphology()).expect("generates")
    }

    // ------------------------------------------------------------- the notation

    #[test]
    fn a_proposition_parses_a_predicate_and_two_arguments() {
        let p = Proposition::parse("SEE(KING, STAR)").expect("parses");
        assert_eq!(p.predicate.as_str(), "SEE");
        assert_eq!(p.agent.as_ref().unwrap().head.as_str(), "KING");
        assert_eq!(p.patient.as_ref().unwrap().head.as_str(), "STAR");
        assert!(p.is_transitive());
    }

    #[test]
    fn one_argument_is_an_intransitive_clause() {
        let p = Proposition::parse("SEE(KING)").expect("parses");
        assert!(!p.is_transitive());
        assert!(p.patient.is_none());
    }

    #[test]
    fn an_argument_takes_an_adjective_and_a_possessor() {
        let p = Proposition::parse("SEE(KING:BIG/PRIEST, STAR)").expect("parses");
        let agent = p.agent.as_ref().unwrap();
        assert_eq!(agent.head.as_str(), "KING");
        assert_eq!(agent.adjective.as_ref().unwrap().as_str(), "BIG");
        assert_eq!(agent.possessor.as_ref().unwrap().as_str(), "PRIEST");
    }

    #[test]
    fn a_proposition_round_trips_through_its_own_notation() {
        for text in [
            "SEE(KING, STAR)",
            "SEE(KING)",
            "SEE(KING:BIG, STAR/PRIEST)",
            "SEE(KING:BIG/PRIEST, STAR)",
        ] {
            let parsed = Proposition::parse(text).expect("parses");
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn a_malformed_proposition_says_what_was_expected() {
        for (text, expected) in [
            ("SEE KING, STAR", "no `(` found"),
            ("SEE(KING, STAR", "closing `)`"),
            ("(KING)", "no predicate"),
            ("SEE(A, B, C)", "at most two arguments"),
        ] {
            let err = Proposition::parse(text).expect_err("{text} should not parse");
            assert!(
                err.to_string().contains(expected),
                "`{text}` said `{err}`, expected `{expected}`"
            );
        }
    }

    // ------------------------------------------------------------ the generator

    #[test]
    fn a_transitive_clause_comes_out_in_the_declared_order() {
        let sentence = say("SEE(KING, STAR)", &sov());
        assert_eq!(
            sentence.slots.iter().map(|s| s.role).collect::<Vec<_>>(),
            ["agent", "patient", "predicate"],
            "SOV"
        );
        assert_eq!(
            sentence.written(&inventory()).expect("renders"),
            "takir kira san",
            "the king-ERG the star-ABS sees"
        );
    }

    /// **ROADMAP M18's acceptance.** The same proposition through two profiles gives
    /// two different sentences, and the difference is the profiles'.
    #[test]
    fn one_proposition_through_two_profiles_gives_two_sentences() {
        let a = say("SEE(KING:BIG, STAR/PRIEST)", &sov());
        let b = say("SEE(KING:BIG, STAR/PRIEST)", &svo());

        let written = |s: &Sentence| s.written(&inventory()).expect("renders");
        assert_ne!(written(&a), written(&b));

        // And the difference is structural, not cosmetic: order, modifier position
        // and case marking all move.
        assert_eq!(
            a.slots.iter().map(|s| s.role).collect::<Vec<_>>(),
            ["adjective", "agent", "possessor", "patient", "predicate"]
        );
        assert_eq!(
            b.slots.iter().map(|s| s.role).collect::<Vec<_>>(),
            ["agent", "adjective", "predicate", "patient", "possessor"]
        );
    }

    /// Alignment is the parameter with the sharpest consequence, and this is what it
    /// *means*: the single argument of an intransitive verb patterns with the
    /// transitive **object** in an ergative language, and with the **subject** in a
    /// nominative one.
    #[test]
    fn alignment_decides_how_the_intransitive_argument_is_marked() {
        let ergative = say("SEE(KING)", &sov());
        assert_eq!(
            ergative.written(&inventory()).expect("renders"),
            "taka san",
            "intransitive subject takes the ABSOLUTIVE in an ergative language"
        );
        let transitive = say("SEE(KING, STAR)", &sov());
        assert!(
            transitive
                .written(&inventory())
                .expect("renders")
                .starts_with("takir"),
            "…but the transitive agent takes the ergative"
        );

        let nominative = say("SEE(KING)", &svo());
        assert_eq!(
            nominative.written(&inventory()).expect("renders"),
            "taku san",
            "a nominative language marks it the same as a transitive subject"
        );
    }

    // ------------------------------------------------- §3.3: the record it leaves

    /// **The constraint that governs this milestone.** A sentence carries a record of
    /// the constructions that built it, exactly as a word carries its derivation.
    #[test]
    fn every_sentence_records_the_constructions_that_built_it() {
        let sentence = say("SEE(KING:BIG/PRIEST, STAR)", &sov());
        let ids: Vec<&str> = sentence.constructions.iter().map(|c| c.id).collect();
        assert!(ids.contains(&"noun_phrase"), "{ids:?}");
        assert!(ids.contains(&"case_marking"), "{ids:?}");
        assert!(ids.contains(&"clause"), "{ids:?}");

        // And every one names the parameter that decided it — the same kind of answer
        // a sound change gives for a segment.
        for construction in &sentence.constructions {
            assert!(
                !construction.because.is_empty(),
                "`{}` recorded no reason",
                construction.id
            );
        }
        let clause = sentence
            .constructions
            .iter()
            .find(|c| c.id == "clause")
            .expect("a clause construction");
        assert!(clause.because.contains("word_order"), "{}", clause.because);
    }

    #[test]
    fn a_slot_records_the_word_it_came_from_and_its_descent_class() {
        let sentence = say("SEE(KING, STAR)", &sov());
        for slot in &sentence.slots {
            assert!(!slot.word.is_empty());
            assert!(
                slot.cognate_set.as_str().starts_with("cog_x_"),
                "a sentence must stay comparable across a family"
            );
        }
    }

    #[test]
    fn generating_twice_produces_an_identical_sentence() {
        assert_eq!(
            say("SEE(KING, STAR)", &sov()),
            say("SEE(KING, STAR)", &sov())
        );
    }

    // ------------------------------------------------------------- honest gaps

    /// A language with no case morphemes gets an unmarked sentence and a **stated
    /// gap** — never a fabricated affix.
    #[test]
    fn a_missing_case_marker_is_reported_rather_than_invented() {
        let bare = Morphology::default();
        let proposition = Proposition::parse("SEE(KING, STAR)").expect("parses");
        let sentence = generate(&proposition, &sov(), &lexicon(), &bare).expect("still generates");

        assert_eq!(
            sentence.written(&inventory()).expect("renders"),
            "tak kir san",
            "unmarked, because this language declares no case morphemes"
        );
        assert_eq!(sentence.gaps.len(), 2, "{:?}", sentence.gaps);
        assert!(sentence.gaps[0].contains("ERG"), "{:?}", sentence.gaps);
        assert!(
            sentence.gaps[0].contains("declare one"),
            "a gap should say what to do about it: {:?}",
            sentence.gaps
        );
    }

    /// Neutral alignment marks nothing, and that is a **decision**, not a gap.
    #[test]
    fn neutral_alignment_marks_nothing_and_reports_no_gap() {
        let profile = SyntaxProfile {
            alignment: Alignment::Neutral,
            ..sov()
        };
        let proposition = Proposition::parse("SEE(KING, STAR)").expect("parses");
        let sentence =
            generate(&proposition, &profile, &lexicon(), &morphology()).expect("generates");
        assert!(sentence.gaps.is_empty(), "{:?}", sentence.gaps);
        assert_eq!(
            sentence.written(&inventory()).expect("renders"),
            "tak kir san"
        );
    }

    /// **The bug this test was written for.** Case marking was written with `case?`,
    /// which returns `None` for the whole closure when there is no case to apply — and
    /// therefore deleted the noun phrase. Every language with `Neutral` or unstated
    /// alignment produced a sentence containing only its verb.
    ///
    /// Swept over every alignment, because the failing one was the *default*: a
    /// language that had said nothing about its alignment lost both its arguments.
    #[test]
    fn no_alignment_ever_drops_an_argument() {
        for alignment in [
            Alignment::Unspecified,
            Alignment::NominativeAccusative,
            Alignment::ErgativeAbsolutive,
            Alignment::Tripartite,
            Alignment::ActiveStative,
            Alignment::Neutral,
        ] {
            let profile = SyntaxProfile { alignment, ..sov() };
            let proposition = Proposition::parse("SEE(KING, STAR)").expect("parses");
            let sentence =
                generate(&proposition, &profile, &lexicon(), &morphology()).expect("generates");
            assert_eq!(
                sentence.slots.len(),
                3,
                "{alignment:?} produced {:?}",
                sentence.slots.iter().map(|s| s.role).collect::<Vec<_>>()
            );
        }
    }

    /// A concept the lexicon has no word for is an **error**: there is no sentence to
    /// be had, and coining one on the spot would be the fabrication this whole project
    /// is built to avoid.
    #[test]
    fn a_concept_with_no_word_is_an_error_naming_it() {
        let proposition = Proposition::parse("SEE(DRAGON, STAR)").expect("parses");
        let err = generate(&proposition, &sov(), &lexicon(), &morphology())
            .expect_err("no word for DRAGON");
        assert!(err.to_string().contains("DRAGON"), "{err}");
    }

    /// An unstated word order still produces a sentence, and the record says the
    /// parameter was unstated — the fallback is visible rather than a silent claim.
    #[test]
    fn an_unstated_parameter_falls_back_visibly() {
        let sentence = say("SEE(KING, STAR)", &SyntaxProfile::default());
        let clause = sentence
            .constructions
            .iter()
            .find(|c| c.id == "clause")
            .expect("a clause construction");
        assert!(clause.because.contains("not stated"), "{}", clause.because);
        assert!(clause.effect.contains("SVO"), "{}", clause.effect);
    }
}
