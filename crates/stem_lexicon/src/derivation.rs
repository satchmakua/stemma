//! Derivation: compounds and productive affixation (ROADMAP M14, `DESIGN.md` §7.3).
//!
//! # What this milestone is for
//!
//! M13 grew the concept list to 673, and every one of those words is an independent
//! draw from the same urn. A real lexicon is not like that: most of it is **made of
//! the rest of it**. `starlight` is `star` + `light`; `baker` is `bake` + an agent
//! suffix; and knowing that is most of what it means to know the word. M14 gives a
//! large vocabulary an **etymology** instead of a size.
//!
//! # Two formations, and why only one of them is productive
//!
//! [`Formation::Affix`] is productive: a derivational affix applies to *every* word
//! of a given part of speech, so one pattern over 351 nouns coins 351 words. That is
//! honest, because productivity is exactly what a derivational affix has — English
//! `-ness` really does attach to essentially any adjective.
//!
//! [`Formation::Compound`] is **authored**, and that asymmetry is deliberate.
//! Compounding every noun with every noun would coin 123,201 words for a 351-noun
//! language, and whether `star` + `stone` means *meteor* or nothing at all is a fact
//! about a culture, not a rule. Coining all of them would be the tool making a claim
//! rather than reporting one — `DESIGN.md` §7.5's argument, applied where it is
//! actually right. So the author names the pairs, and M15's culture profile is what
//! will eventually propose them.
//!
//! # The derivation draw contract
//!
//! Normative, in the register of [`crate::build`]'s. Patterns are processed in
//! **authored order**; within a pattern, bases are taken in **lexicon stored
//! order**; ordinals continue from the end of the base lexicon. Nothing draws — this
//! is mechanical concatenation, with no RNG anywhere, so two runs are byte-identical
//! for the reason `apply_rules` is.
//!
//! The consequence matters as much as it does for concepts: a pattern's output is a
//! contiguous block, so **appending a pattern cannot move a word an earlier pattern
//! coined**. Inserting one anywhere else renumbers every derived word after it.
//! Same contract, same hazard, same rule: append.
//!
//! # Why the parts are recorded and not recomputed
//!
//! A compound's parts are stored as [`BaseRef`] spans into the composition form, and
//! recovered through [`crate::WordEntry::morpheme_surface`] — the identical
//! discipline M8 gave a morpheme, for the identical reason. It is what makes the
//! milestone's second acceptance true: run a sound change over `sosem-nakuli` and it
//! erodes to something whose parts **no eye could recover**, while the record still
//! says exactly which two words went in and which segments each became. That is what
//! lexicalisation is, and a tool that recomputed the decomposition by matching
//! strings would lose the word at exactly the moment it got interesting.

use serde::{Deserialize, Serialize};
use stem_core::{CognateSetId, LanguageId, MorphemeId, Result, StemmaError, WordId};

use crate::build::scoped_cognate_set;
use crate::concept::{ConceptKey, PartOfSpeech};
use crate::lexicon::Lexicon;
use crate::morpheme::{Morpheme, MorphemeRef, MorphemeRole, lay_out};
use crate::word::{WordEntry, WordSource};

// ------------------------------------------------------------------ the record

/// One **word** this word was built from, tagged with the flat half-open span
/// `[start, end)` it occupies in the composition (pre-sound-change) form.
///
/// # Why this is not a [`MorphemeRef`]
///
/// A [`MorphemeRef`] names a [`MorphemeId`] — an entry in the genome's declared
/// morpheme inventory. A compound's parts are not affixes; they are **lexicon
/// entries**, and there are 673 of them. Lifting every word into a parallel morpheme
/// inventory would duplicate every root's form in a second place, which is the
/// desync `docs/adr/0007` forbids, and would bloat every project file by its whole
/// lexicon over again.
///
/// So this is the word-level twin, holding the same four things a `MorphemeRef`
/// holds — an identity, an echoed label, and a span — plus the one a morpheme has no
/// use for: the base's [`CognateSetId`].
///
/// # Why the cognate set is echoed
///
/// The ROADMAP's phrase is that a derived word's parts must be *cognate-visible*.
/// Within one language `word` resolves and the set is redundant; across a family it
/// is the opposite way round, because [`CognateSetId`] is the family-scoped thread
/// and a [`WordId`] is not. A daughter's `starstone` and its sister's `starstone` are
/// built from the same two ancestral words, and this field is what says so without a
/// second file open. It is an echo in the [`crate::SenseRef::gloss`] sense — copied
/// from one source of truth, with `derivation.stale_base` as its paired guard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseRef {
    /// The lexicon entry this part is, in **this** language.
    pub word: WordId,
    /// That entry's descent class, echoed so the etymology is legible across a
    /// family without resolving `word` first.
    pub cognate_set: CognateSetId,
    /// That entry's gloss at composition time, echoed so the record is
    /// self-contained — the `MorphemeRef::gloss` precedent.
    pub gloss: String,
    /// Flat start index into the composition form (inclusive).
    pub start: u32,
    /// Flat end index into the composition form (exclusive).
    pub end: u32,
}

// ----------------------------------------------------------------- the patterns

/// How a pattern builds a word.
///
/// `#[non_exhaustive]` so that reduplication, conversion (zero-derivation) or a
/// three-base compound can arrive without a breaking change. Nothing on disk refers
/// to a variant by index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Formation {
    /// **Productive.** Attach `affix` to every lexicon word whose part of speech is
    /// `applies_to`, in stored order.
    Affix {
        /// The derivational affix, resolved against the genome's morphemes. Its own
        /// [`MorphemeRole`] decides whether it lands before or after the base.
        affix: MorphemeId,
        /// Which words this pattern is productive over.
        applies_to: PartOfSpeech,
    },
    /// **Authored.** Compound exactly these pairs of bases, named by concept.
    Compound {
        /// The pairs, in authored order — which is the order they are coined in.
        pairs: Vec<CompoundPair>,
    },
}

impl Formation {
    /// A one-line description, for `stemma derivations` and the M11 UI.
    ///
    /// Lives here rather than in the CLI because [`Formation`] is
    /// `#[non_exhaustive]`: a downstream `match` would need a wildcard arm, and a
    /// wildcard is exactly what stops a new variant from being a compile error at
    /// the place that has to learn about it. Inside the defining crate the match is
    /// checked, so adding `Reduplication` breaks this function and nothing else —
    /// `PartOfSpeech::name`'s precedent.
    pub fn summary(&self) -> String {
        match self {
            Self::Affix { affix, applies_to } => format!("`{affix}` on every {applies_to}"),
            Self::Compound { pairs } => format!("{} authored compound(s)", pairs.len()),
        }
    }
}

/// One authored compound: two bases, named by the concept each realises.
///
/// Named by [`ConceptKey`] rather than [`WordId`], because a `w_0081` in a *seeded*
/// lexicon is whatever the concept list happened to put at position 81 — it would
/// silently mean a different word after M13's append. A concept key means the same
/// thing in every language, which is the whole reason concepts exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundPair {
    /// The concept realised by the left-hand (first) base.
    pub left: ConceptKey,
    /// The concept realised by the right-hand (second) base.
    pub right: ConceptKey,
}

/// One way this language makes new words (`DESIGN.md` §7.3).
///
/// Lives in [`crate::Morphology`] beside the paradigms, because how a language forms
/// words is a fact about the language — the `Paradigm` precedent, and for the same
/// reason M12 put project concepts in the genome: a language must stay reproducible
/// from its own file alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivationPattern {
    /// A short id, e.g. `"AGENT"`.
    pub id: String,
    /// A display name, e.g. `"Agent noun"`.
    pub name: String,
    /// How it builds.
    pub formation: Formation,
    /// The gloss template. `{1}` is the first base's gloss and `{2}` the second, so
    /// `"one who {1}s"` and `"{1}-{2}"` both work. A placeholder with no base is
    /// left in place literally and reported (`derivation.unfilled_gloss_slot`)
    /// rather than silently dropped — a dictionary full of `one who {2}s` should be
    /// obvious, not invisible.
    pub gloss: String,
    /// The part of speech the coined word carries. Derivation routinely changes it —
    /// that is most of what it is for.
    pub part_of_speech: PartOfSpeech,
    /// Cap on how many words this pattern may coin, taking the first eligible bases
    /// in lexicon order. `None` means every eligible base.
    ///
    /// A cap, never a sample: a *random* subset would need an RNG on a path that has
    /// none, and "the first N" is reproducible from the file alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Authorial prose. Not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl DerivationPattern {
    /// The gloss for a word derived from these bases' glosses.
    ///
    /// Positional substitution, one pass, no recursion: `{1}` and `{2}` are replaced
    /// by the first and second base gloss. A pattern that names a slot it has no
    /// base for leaves the placeholder visible, which `check_against_derivations`
    /// reports.
    pub fn render_gloss(&self, bases: &[&str]) -> String {
        let mut out = self.gloss.clone();
        for (i, base) in bases.iter().enumerate() {
            out = out.replace(&format!("{{{}}}", i + 1), base);
        }
        out
    }

    /// How many base slots this formation fills — 1 for affixation, 2 for a compound.
    pub fn arity(&self) -> usize {
        match &self.formation {
            Formation::Affix { .. } => 1,
            Formation::Compound { .. } => 2,
        }
    }

    /// The highest `{n}` the gloss template names, or 0 when it names none.
    ///
    /// Scans for `{1}`..`{9}` only. A template is a two-slot convenience, not a
    /// format language — anything richer belongs to the author, not to this type.
    pub fn highest_slot(&self) -> usize {
        (1..=9)
            .rfind(|n| self.gloss.contains(&format!("{{{n}}}")))
            .unwrap_or(0)
    }
}

// -------------------------------------------------------------------- the build

/// Coins one [`WordEntry`] per (pattern × eligible base), appending to `lexicon`.
///
/// Pure, total and RNG-free — mechanical concatenation and positional ordinals, so
/// two runs are byte-identical. Each coined word is an **ordinary `WordEntry`**, so
/// it flows through `apply_rules`, `fork`, `trace`, `cognates` and export exactly
/// like any other word, which is what makes "a later sound change makes the compound
/// opaque" fall out of the untouched engine rather than needing bespoke support
/// (`docs/adr/0010`'s argument, transferred from inflection).
///
/// # What the caller passes
///
/// `lexicon` is the **base** lexicon: every entry in it is a candidate base, and
/// ordinals continue from `lexicon.len() + 1`. The CLI drops previously derived
/// entries before calling, which is the `new-lexicon` "replace, never append" rule
/// and is what makes running `stemma derive` twice byte-identical. Passing a lexicon
/// that already holds derived words is legitimate layered derivation — a compound of
/// a compound — and the caller owns that choice.
///
/// # Cognate sets
///
/// Each coined word mints its **own** set via [`scoped_cognate_set`], the only
/// sanctioned mint site. A derived word is a new lexeme, not a reflex of its base:
/// `star` and `starstone` are different entries with different forms and different
/// meanings, so they must be different sets or M5's table would show them on one row
/// (`docs/adr/0007`). Their relationship is recorded by [`BaseRef`], which is what
/// that record is *for*.
///
/// # Failure
///
/// A named affix that is absent or is a `Stem`, or a compound naming a concept the
/// lexicon does not realise, is a [`StemmaError::NotFound`] — a spec bug surfaced,
/// never a silent drop. That is `inflect`'s policy, unchanged.
pub fn derive(
    patterns: &[DerivationPattern],
    morphemes: &[Morpheme],
    lexicon: &Lexicon,
    language: &LanguageId,
) -> Result<Vec<WordEntry>> {
    let mut coined: Vec<WordEntry> = Vec::new();
    let mut ordinal = lexicon.len();

    for pattern in patterns {
        // Per **pattern**, never a running total across patterns. A shared counter
        // reads plausibly and is wrong in a way nothing shouts about: with
        // `--limit 2`, thirteen affix patterns would fill the budget and the
        // fourteenth — the compounds — would coin silently nothing.
        let mut made = 0usize;
        let capped = |made: usize| pattern.limit.is_some_and(|cap| made >= cap);

        match &pattern.formation {
            Formation::Affix { affix, applies_to } => {
                let affix = morphemes
                    .iter()
                    .find(|m| &m.id == affix)
                    .filter(|m| m.role != MorphemeRole::Stem)
                    .ok_or_else(|| StemmaError::not_found("derivational affix", affix))?;

                for base in lexicon.iter() {
                    if base.part_of_speech != *applies_to {
                        continue;
                    }
                    if capped(made) {
                        break;
                    }
                    ordinal += 1;
                    made += 1;
                    coined.push(build(pattern, &[base], Some(affix), ordinal, language));
                }
            }
            Formation::Compound { pairs } => {
                for pair in pairs {
                    if capped(made) {
                        break;
                    }
                    let left = resolve_base(lexicon, &pair.left)?;
                    let right = resolve_base(lexicon, &pair.right)?;
                    ordinal += 1;
                    made += 1;
                    coined.push(build(pattern, &[left, right], None, ordinal, language));
                }
            }
        }
    }

    // Ordinals continue from `lexicon.len()`, which is collision-free for a lexicon
    // whose ids are the sequential `w_0001..w_000N` that `build_proto_lexicon` and
    // `inflect` produce — but not for a hand-authored one with gaps. A duplicate
    // word id is an Error `Lexicon::validate` would report, and a command that
    // writes a file the validator immediately calls broken while reporting success
    // is the validator and the engine disagreeing. Refuse instead, naming the id.
    let existing: std::collections::BTreeSet<&str> =
        lexicon.iter().map(|e| e.id.as_str()).collect();
    if let Some(clash) = coined.iter().find(|e| existing.contains(e.id.as_str())) {
        return Err(StemmaError::not_found(
            "a free word id (the base lexicon's ids are not sequential, so derivation \
             would collide at)",
            &clash.id,
        ));
    }

    Ok(coined)
}

/// The one lexicon entry realising `key`, or a `NotFound` naming it.
///
/// Takes the **first** in stored order when a language has synonyms, which is
/// `by_meaning`'s policy verbatim — the two resolvers must not disagree about which
/// word a meaning means.
fn resolve_base<'a>(lexicon: &'a Lexicon, key: &ConceptKey) -> Result<&'a WordEntry> {
    lexicon
        .by_concept(key)
        .first()
        .copied()
        .ok_or_else(|| StemmaError::not_found("base word for concept", key))
}

/// Builds one coined entry from its bases and optional affix.
///
/// Surface order is **prefix · bases (given order) · suffix**, laid out by
/// [`lay_out`] so the spans and the stress rule are computed in exactly the same
/// place `compose` computes them.
fn build(
    pattern: &DerivationPattern,
    bases: &[&WordEntry],
    affix: Option<&Morpheme>,
    ordinal: usize,
    language: &LanguageId,
) -> WordEntry {
    let prefix = affix.filter(|a| a.role == MorphemeRole::Prefix);
    let suffix = affix.filter(|a| a.role != MorphemeRole::Prefix);

    // Surface order, as one list of forms, so the layout is a single pass.
    let forms: Vec<&stem_phonology::Root> = prefix
        .map(|a| &a.form)
        .into_iter()
        .chain(bases.iter().map(|b| &b.phonemic_form))
        .chain(suffix.map(|a| &a.form))
        .collect();
    let (form, spans) = lay_out(forms);

    // Spans come back in the same order the forms went in, so they split cleanly:
    // an optional leading prefix, then one per base, then an optional suffix.
    let mut span = spans.iter().copied();
    let prefix_span = prefix.map(|_| span.next().expect("a prefix contributed a span"));
    let base_spans: Vec<(u32, u32)> = bases
        .iter()
        .map(|_| span.next().expect("every base contributes a span"))
        .collect();
    let suffix_span = suffix.map(|_| span.next().expect("a suffix contributed a span"));

    let base_refs: Vec<BaseRef> = bases
        .iter()
        .zip(&base_spans)
        .map(|(base, &(start, end))| BaseRef {
            word: base.id.clone(),
            cognate_set: base.cognate_set.clone(),
            // `display_gloss` can only be `None` for an entry `lexicon.no_gloss`
            // already calls an Error, so falling back to the id here records
            // something true rather than inventing a meaning.
            gloss: base
                .display_gloss()
                .unwrap_or_else(|| base.id.as_str())
                .to_owned(),
            start,
            end,
        })
        .collect();

    let affix_refs: Vec<MorphemeRef> = prefix_span
        .into_iter()
        .chain(suffix_span)
        .map(|(start, end)| {
            let affix = affix.expect("a span exists only when the affix does");
            MorphemeRef {
                morpheme: affix.id.clone(),
                role: affix.role,
                gloss: affix.gloss.clone(),
                start,
                end,
            }
        })
        .collect();

    let glosses: Vec<&str> = base_refs.iter().map(|b| b.gloss.as_str()).collect();

    WordEntry {
        id: WordId::sequential(ordinal),
        // A derived word realises no built-in concept: `starstone` is not on any
        // comparison list, and claiming a concept it does not have would put it on
        // an M5 cognate-table row it has no business being on. Its meaning is the
        // rendered template, carried in `glosses` like any coined word's.
        concept: None,
        phonemic_form: form,
        glosses: vec![pattern.render_gloss(&glosses)],
        part_of_speech: pattern.part_of_speech,
        cognate_set: scoped_cognate_set(language, ordinal),
        // The same variant `inflect` writes, deliberately. Inflection and derivation
        // are genuinely different things, but `WordSource` records *how a word came
        // to exist* and both answers are "built by composition from parts". The
        // distinction is already visible without a variant — an inflected cell has
        // `morphemes` and no `bases` — so adding one would store a derivable value,
        // the defect class `WordSource`'s own docs warn about.
        source: WordSource::Derived,
        trace: None,
        morphemes: affix_refs,
        bases: base_refs,
        senses: Vec::new(),
        sense_history: None,
    }
}

// ---------------------------------------------------------------- the reporting

/// The derivation checks that need the lexicon and the morphology together.
///
/// A free function taking context, for the reason `check_against_inventory` is one:
/// `Validate::validate(&self)` takes no arguments, and none of these questions can be
/// answered from inside a bare [`crate::Morphology`].
///
/// Four checks, all reporting rather than policing (§17):
///
/// - `unknown_affix` — a pattern names an affix the morphology has not declared, or
///   one declared as a `Stem`. A **Warning** here even though [`derive`] errors on
///   it, because validation is a report on a file at rest and the file is still
///   perfectly loadable; you find out at the moment you ask for the derivation.
/// - `unfilled_gloss_slot` — the template names `{2}` but the formation has one base.
/// - `stale_base` — a stored [`BaseRef`]'s **cognate set** disagrees with the entry
///   it names. A Warning: the etymology now points at a thread its base is not on.
///   The paired guard for that echo, exactly as `semantics.stale_sense_gloss` guards
///   `SenseRef`'s.
/// - `base_gloss_drifted` — the base's **gloss** has changed since the compound was
///   formed. Only a Note, and a separate code, because this one is *ordinary*: M9
///   drift is supposed to move meanings, and the record is deliberately of the
///   meaning at composition time. `star`-`stone` stays "star-stone" after `star`
///   comes to mean "omen", which is exactly how `hlāfweard` kept its loaf.
/// - `dangling_base` — a `BaseRef` names a word this lexicon does not hold.
pub fn check_against_derivations(
    lexicon: &Lexicon,
    patterns: &[DerivationPattern],
    morphemes: &[Morpheme],
) -> stem_core::ValidationReport {
    use stem_core::{Issue, Severity};
    let mut report = stem_core::ValidationReport::new();

    for pattern in patterns {
        if let Formation::Affix { affix, .. } = &pattern.formation
            && !morphemes
                .iter()
                .any(|m| &m.id == affix && m.role != MorphemeRole::Stem)
        {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "unknown_affix",
                    format!(
                        "derivation `{}` names affix `{affix}`, which this language does not \
                         declare as an affix; `stemma derive` will refuse it",
                        pattern.id
                    ),
                )
                .about(&pattern.id),
            );
        }

        let highest = pattern.highest_slot();
        if highest > pattern.arity() {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "unfilled_gloss_slot",
                    format!(
                        "derivation `{}` glosses as \"{}\", but its formation supplies only {} \
                         base(s), so `{{{highest}}}` will be printed literally in every word it \
                         coins",
                        pattern.id,
                        pattern.gloss,
                        pattern.arity()
                    ),
                )
                .about(&pattern.id),
            );
        }
    }

    for entry in lexicon.iter() {
        for base in &entry.bases {
            match lexicon.get(&base.word) {
                Some(source) => {
                    if source.cognate_set != base.cognate_set {
                        report.push(
                            Issue::new(
                                Severity::Warning,
                                "stale_base",
                                format!(
                                    "word `{}` records base `{}` in cognate set `{}`, but that \
                                     entry is in `{}`; the echo is stale",
                                    entry.id, base.word, base.cognate_set, source.cognate_set
                                ),
                            )
                            .about(&entry.id),
                        );
                    }
                    if let Some(shown) = source.display_gloss()
                        && shown != base.gloss
                    {
                        report.push(
                            Issue::new(
                                Severity::Note,
                                // A **different code** from `stale_base`, not the
                                // same one at a lower severity: one code meaning two
                                // things at two severities makes every
                                // `warnings().any(|i| i.code == …)` in a test or a
                                // caller ambiguous about which it caught.
                                "base_gloss_drifted",
                                format!(
                                    "word `{}` records base `{}` as \"{}\", but that entry now \
                                     displays \"{shown}\" — a meaning drifted after the \
                                     compound was formed, which is ordinary; the record is of \
                                     the meaning at composition time",
                                    entry.id, base.word, base.gloss
                                ),
                            )
                            .about(&entry.id),
                        );
                    }
                }
                None => {
                    report.push(
                        Issue::new(
                            Severity::Warning,
                            "dangling_base",
                            format!(
                                "word `{}` is derived from `{}`, which is not in this lexicon",
                                entry.id, base.word
                            ),
                        )
                        .about(&entry.id),
                    );
                }
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::PartOfSpeech;
    use crate::trace::{Derivation, RuleApplication, SiteTrace};
    use stem_core::{PhonemeId, RuleId};
    use stem_phonology::{Root, Syllable};

    fn syllable(pattern: &str, segments: &[&str]) -> Syllable {
        Syllable {
            pattern: pattern.to_owned(),
            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
            stress: None,
        }
    }

    /// A base word. Uses `scoped_cognate_set` rather than `CognateSetId::new`,
    /// because the mint scan does not exempt `#[cfg(test)]`.
    fn word(ordinal: usize, concept: &str, gloss: &str, syllables: Vec<Syllable>) -> WordEntry {
        WordEntry {
            id: WordId::sequential(ordinal),
            concept: Some(ConceptKey::new(concept)),
            phonemic_form: Root { syllables },
            glosses: vec![gloss.to_owned()],
            part_of_speech: PartOfSpeech::Noun,
            cognate_set: scoped_cognate_set(&LanguageId::new("x"), ordinal),
            source: WordSource::Generated,
            trace: None,
            morphemes: Vec::new(),
            bases: Vec::new(),
            senses: Vec::new(),
            sense_history: None,
        }
    }

    /// `tira` "star" and `sula` "stone", plus a verb so a POS filter has something
    /// to exclude.
    fn base_lexicon() -> Lexicon {
        let mut star = word(
            1,
            "STAR",
            "star",
            vec![
                syllable("CV", &["ph_t", "ph_i"]),
                syllable("CV", &["ph_r", "ph_a"]),
            ],
        );
        star.part_of_speech = PartOfSpeech::Noun;
        let stone = word(
            2,
            "STONE",
            "stone",
            vec![
                syllable("CV", &["ph_s", "ph_u"]),
                syllable("CV", &["ph_l", "ph_a"]),
            ],
        );
        let mut run = word(
            3,
            "WALK",
            "walk",
            vec![syllable("CVC", &["ph_k", "ph_a", "ph_n"])],
        );
        run.part_of_speech = PartOfSpeech::Verb;
        Lexicon::from_entries([star, stone, run])
    }

    /// `-ri`, an agent suffix.
    fn agent_suffix() -> Morpheme {
        Morpheme {
            id: MorphemeId::new("m_agent"),
            role: MorphemeRole::Suffix,
            gloss: "AGT".to_owned(),
            form: Root {
                syllables: vec![syllable("CV", &["ph_r", "ph_i"])],
            },
            part_of_speech: PartOfSpeech::Noun,
        }
    }

    fn agent_pattern() -> DerivationPattern {
        DerivationPattern {
            id: "AGENT".to_owned(),
            name: "Agent noun".to_owned(),
            formation: Formation::Affix {
                affix: MorphemeId::new("m_agent"),
                applies_to: PartOfSpeech::Verb,
            },
            gloss: "one who {1}s".to_owned(),
            part_of_speech: PartOfSpeech::Noun,
            limit: None,
            note: String::new(),
        }
    }

    fn compound_pattern() -> DerivationPattern {
        DerivationPattern {
            id: "COMPOUND".to_owned(),
            name: "Noun compound".to_owned(),
            formation: Formation::Compound {
                pairs: vec![CompoundPair {
                    left: ConceptKey::new("STAR"),
                    right: ConceptKey::new("STONE"),
                }],
            },
            gloss: "{1}-{2}".to_owned(),
            part_of_speech: PartOfSpeech::Noun,
            limit: None,
            note: String::new(),
        }
    }

    fn segs(root: &Root) -> Vec<&str> {
        root.segments().map(|s| s.as_str()).collect()
    }

    #[test]
    fn a_compound_concatenates_its_bases_and_records_both_with_their_spans() {
        let lexicon = base_lexicon();
        let coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");

        assert_eq!(coined.len(), 1);
        let compound = &coined[0];
        assert_eq!(
            segs(&compound.phonemic_form),
            [
                "ph_t", "ph_i", "ph_r", "ph_a", "ph_s", "ph_u", "ph_l", "ph_a"
            ],
            "tira + sula = tirasula"
        );
        assert_eq!(compound.glosses, ["star-stone"]);
        assert_eq!(
            compound.bases.len(),
            2,
            "§3.3: both parts are on the record"
        );
        assert_eq!((compound.bases[0].start, compound.bases[0].end), (0, 4));
        assert_eq!((compound.bases[1].start, compound.bases[1].end), (4, 8));
        assert_eq!(compound.bases[0].word.as_str(), "w_0001");
        assert_eq!(compound.bases[1].gloss, "stone");
        // Ordinals continue past the base lexicon, so nothing collides.
        assert_eq!(compound.id.as_str(), "w_0004");
        assert_eq!(compound.source, WordSource::Derived);
    }

    #[test]
    fn a_derived_word_carries_its_bases_cognate_sets_so_the_parts_stay_visible() {
        let lexicon = base_lexicon();
        let coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");
        let compound = &coined[0];
        for base in &compound.bases {
            let source = lexicon.get(&base.word).expect("the base is in the lexicon");
            assert_eq!(
                base.cognate_set, source.cognate_set,
                "the echo must match its source, or the etymology points nowhere"
            );
        }
        assert_ne!(
            compound.cognate_set, compound.bases[0].cognate_set,
            "a compound is a new lexeme, not a reflex of its left base"
        );
    }

    #[test]
    fn productive_affixation_coins_one_word_per_base_of_the_right_part_of_speech() {
        let coined = derive(
            &[agent_pattern()],
            &[agent_suffix()],
            &base_lexicon(),
            &LanguageId::new("x"),
        )
        .expect("derives");

        assert_eq!(coined.len(), 1, "only the verb is eligible");
        assert_eq!(coined[0].glosses, ["one who walks"]);
        assert_eq!(
            segs(&coined[0].phonemic_form),
            ["ph_k", "ph_a", "ph_n", "ph_r", "ph_i"]
        );
        assert_eq!(
            coined[0].part_of_speech,
            PartOfSpeech::Noun,
            "derivation changes the part of speech; that is most of what it is for"
        );
        // The affix is a MorphemeRef; the base is a BaseRef. Both spans recorded.
        assert_eq!(coined[0].bases.len(), 1);
        assert_eq!(coined[0].morphemes.len(), 1);
        assert_eq!(
            (coined[0].morphemes[0].start, coined[0].morphemes[0].end),
            (3, 5)
        );
    }

    #[test]
    fn a_prefix_lands_before_the_base() {
        let prefix = Morpheme {
            id: MorphemeId::new("m_neg"),
            role: MorphemeRole::Prefix,
            gloss: "NEG".to_owned(),
            form: Root {
                syllables: vec![syllable("V", &["ph_a"])],
            },
            part_of_speech: PartOfSpeech::Noun,
        };
        let pattern = DerivationPattern {
            id: "NEG".to_owned(),
            formation: Formation::Affix {
                affix: MorphemeId::new("m_neg"),
                applies_to: PartOfSpeech::Verb,
            },
            gloss: "not {1}".to_owned(),
            ..agent_pattern()
        };
        let coined = derive(
            &[pattern],
            &[prefix],
            &base_lexicon(),
            &LanguageId::new("x"),
        )
        .expect("derives");
        assert_eq!(
            segs(&coined[0].phonemic_form),
            ["ph_a", "ph_k", "ph_a", "ph_n"]
        );
        assert_eq!(
            (coined[0].morphemes[0].start, coined[0].morphemes[0].end),
            (0, 1)
        );
        assert_eq!((coined[0].bases[0].start, coined[0].bases[0].end), (1, 4));
    }

    #[test]
    fn deriving_twice_produces_identical_entries() {
        let lexicon = base_lexicon();
        let patterns = [compound_pattern(), agent_pattern()];
        let run = || {
            derive(
                &patterns,
                &[agent_suffix()],
                &lexicon,
                &LanguageId::new("x"),
            )
            .expect("derives")
        };
        assert_eq!(run(), run(), "no RNG, so two runs are byte-identical");
    }

    /// The derivation draw contract: a pattern's block is contiguous, so appending a
    /// pattern cannot move a word an earlier one coined.
    #[test]
    fn appending_a_pattern_cannot_move_a_word_an_earlier_pattern_coined() {
        let lexicon = base_lexicon();
        let morphemes = [agent_suffix()];
        let language = LanguageId::new("x");

        let first =
            derive(&[compound_pattern()], &morphemes, &lexicon, &language).expect("derives");
        let both = derive(
            &[compound_pattern(), agent_pattern()],
            &morphemes,
            &lexicon,
            &language,
        )
        .expect("derives");

        assert!(both.len() > first.len());
        for (a, b) in first.iter().zip(both.iter()) {
            assert_eq!(a, b, "an already-coined derived word moved");
        }
    }

    #[test]
    fn a_limit_caps_a_productive_pattern_at_the_first_bases_in_lexicon_order() {
        let mut lexicon = base_lexicon();
        let mut second_verb = word(4, "RUN", "run", vec![syllable("CV", &["ph_m", "ph_a"])]);
        second_verb.part_of_speech = PartOfSpeech::Verb;
        lexicon.push(second_verb);

        let pattern = DerivationPattern {
            limit: Some(1),
            ..agent_pattern()
        };
        let coined = derive(
            &[pattern],
            &[agent_suffix()],
            &lexicon,
            &LanguageId::new("x"),
        )
        .expect("derives");
        assert_eq!(coined.len(), 1, "capped");
        assert_eq!(
            coined[0].glosses,
            ["one who walks"],
            "the cap takes the FIRST eligible base in stored order, reproducibly"
        );
    }

    /// **A limit is per pattern, not a shared budget.** The first draft compared a
    /// compound pattern's cap against the *running total* across every pattern, so a
    /// capped run filled its budget on the affix patterns and coined silently zero
    /// compounds — no error, no warning, just a formation quietly missing from the
    /// output. Cheap to write, invisible to spot.
    #[test]
    fn a_limit_is_per_pattern_and_not_a_budget_shared_across_them() {
        let lexicon = base_lexicon();
        let patterns = [
            DerivationPattern {
                limit: Some(1),
                ..agent_pattern()
            },
            DerivationPattern {
                limit: Some(1),
                ..compound_pattern()
            },
        ];
        let coined = derive(
            &patterns,
            &[agent_suffix()],
            &lexicon,
            &LanguageId::new("x"),
        )
        .expect("derives");

        assert_eq!(coined.len(), 2, "one from each pattern, not one in total");
        assert_eq!(coined[0].glosses, ["one who walks"], "the affix pattern");
        assert_eq!(
            coined[1].glosses,
            ["star-stone"],
            "the compound pattern must not be starved by the one before it"
        );
    }

    /// Ordinals continue from `lexicon.len()`, which is only collision-free when the
    /// base ids are sequential. A hand-authored lexicon with a gap would otherwise
    /// produce a file `Lexicon::validate` immediately calls broken, from a command
    /// that reported success.
    #[test]
    fn deriving_over_a_lexicon_with_non_sequential_ids_refuses_rather_than_colliding() {
        // Two entries, but the second is `w_0004` — so `lexicon.len()` is 2 and the
        // first derived word would want `w_0003`… and the third would want `w_0004`.
        let mut star = word(1, "STAR", "star", vec![syllable("CV", &["ph_t", "ph_i"])]);
        star.part_of_speech = PartOfSpeech::Verb;
        let mut stone = word(4, "STONE", "stone", vec![syllable("CV", &["ph_s", "ph_u"])]);
        stone.part_of_speech = PartOfSpeech::Verb;
        let lexicon = Lexicon::from_entries([star, stone]);

        let err = derive(
            &[agent_pattern()],
            &[agent_suffix()],
            &lexicon,
            &LanguageId::new("x"),
        )
        .expect_err("w_0004 is already taken");
        assert!(err.to_string().contains("w_0004"), "{err}");
    }

    /// A drifted base gloss is **ordinary**, so it is a Note under its own code —
    /// not the same code as the cognate-set mismatch, which is a real defect.
    #[test]
    fn a_base_whose_meaning_drifted_is_noted_separately_from_a_broken_thread() {
        let lexicon = base_lexicon();
        let coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");

        // The base now means something else — M9 drift doing its job.
        let mut all: Vec<WordEntry> = lexicon.iter().cloned().collect();
        all[0].glosses = vec!["omen".to_owned()];
        all.extend(coined);
        let report = check_against_derivations(&Lexicon::from_entries(all), &[], &[]);

        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "base_gloss_drifted" && i.severity == stem_core::Severity::Note),
            "a drifted meaning is ordinary, not a fault: {report}"
        );
        assert!(
            !report.issues.iter().any(|i| i.code == "stale_base"),
            "the thread is intact, so nothing is stale: {report}"
        );
        assert!(report.is_ok(), "{report}");
    }

    #[test]
    fn deriving_from_an_undeclared_affix_is_an_error_not_a_silent_drop() {
        let err = derive(
            &[agent_pattern()],
            &[],
            &base_lexicon(),
            &LanguageId::new("x"),
        )
        .expect_err("m_agent was not declared");
        assert!(err.to_string().contains("m_agent"), "{err}");
    }

    #[test]
    fn compounding_a_concept_the_lexicon_does_not_realise_is_an_error() {
        let pattern = DerivationPattern {
            formation: Formation::Compound {
                pairs: vec![CompoundPair {
                    left: ConceptKey::new("STAR"),
                    right: ConceptKey::new("DRAGON"),
                }],
            },
            ..compound_pattern()
        };
        let err = derive(&[pattern], &[], &base_lexicon(), &LanguageId::new("x"))
            .expect_err("DRAGON is not coined");
        assert!(err.to_string().contains("DRAGON"), "{err}");
    }

    /// **ROADMAP M14's second acceptance, at the unit level.** Erode the compound and
    /// the parts stop being recoverable by eye — but `morpheme_surface` still walks
    /// each recorded span through the trace and says what each base became.
    #[test]
    fn a_sound_change_can_make_a_compound_opaque_without_losing_the_record() {
        let lexicon = base_lexicon();
        let mut coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");
        let compound = &mut coined[0];

        // Delete the two vowels at the seam (indices 3 and 4): tirasula -> tirsula...
        // committed descending, per the application contract.
        compound.trace = Some(Derivation {
            input: compound.phonemic_form.clone(),
            steps: vec![RuleApplication {
                rule: RuleId::new("r_syncope"),
                index: 0,
                sites: vec![
                    SiteTrace {
                        at: 4,
                        before: PhonemeId::new("ph_s"),
                        after: Some(PhonemeId::new("ph_t")),
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                    SiteTrace {
                        at: 3,
                        before: PhonemeId::new("ph_a"),
                        after: None,
                        resolution: None,
                        left: vec![],
                        right: vec![],
                        emptied_syllable: None,
                    },
                ],
                blocked: vec![],
            }],
        });
        compound.phonemic_form = compound.trace.as_ref().unwrap().final_form();

        // By eye: the left base was `tira`, and the surface no longer contains it.
        assert_eq!(
            segs(&compound.phonemic_form),
            ["ph_t", "ph_i", "ph_r", "ph_t", "ph_u", "ph_l", "ph_a"],
            "the seam has eroded"
        );

        // On the record: both parts still resolve, through their stored spans.
        let left = compound.morpheme_surface(
            compound.bases[0].start as usize,
            compound.bases[0].end as usize,
        );
        let right = compound.morpheme_surface(
            compound.bases[1].start as usize,
            compound.bases[1].end as usize,
        );
        assert_eq!(
            left.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            ["ph_t", "ph_i", "ph_r"],
            "`tira` lost its final vowel and the record says so"
        );
        assert_eq!(
            right.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            ["ph_t", "ph_u", "ph_l", "ph_a"],
            "`sula`'s /s/ assimilated, and the record still points at it"
        );
        assert_eq!(
            compound.bases[0].gloss, "star",
            "the etymology survives the erosion that hid it"
        );
    }

    #[test]
    fn a_gloss_template_slot_with_no_base_is_reported_rather_than_silently_dropped() {
        let pattern = DerivationPattern {
            gloss: "one who {1}s the {2}".to_owned(),
            ..agent_pattern()
        };
        let report = check_against_derivations(&Lexicon::new(), &[pattern], &[agent_suffix()]);
        assert!(
            report.warnings().any(|i| i.code == "unfilled_gloss_slot"),
            "{report}"
        );
        assert!(report.is_ok(), "unusual is not broken: {report}");
    }

    #[test]
    fn a_pattern_naming_an_undeclared_affix_is_reported_at_rest() {
        let report = check_against_derivations(&Lexicon::new(), &[agent_pattern()], &[]);
        assert!(
            report.warnings().any(|i| i.code == "unknown_affix"),
            "{report}"
        );
    }

    #[test]
    fn a_base_ref_whose_cognate_set_drifted_from_its_word_is_caught() {
        let lexicon = base_lexicon();
        let mut coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");
        coined[0].bases[0].cognate_set = scoped_cognate_set(&LanguageId::new("x"), 99);

        let mut all: Vec<WordEntry> = lexicon.iter().cloned().collect();
        all.extend(coined);
        let report = check_against_derivations(&Lexicon::from_entries(all), &[], &[]);
        assert!(
            report.warnings().any(|i| i.code == "stale_base"),
            "{report}"
        );
    }

    #[test]
    fn a_base_that_is_not_in_the_lexicon_is_reported() {
        let lexicon = base_lexicon();
        let coined =
            derive(&[compound_pattern()], &[], &lexicon, &LanguageId::new("x")).expect("derives");
        // The compound alone, without the bases it names.
        let report = check_against_derivations(&Lexicon::from_entries(coined), &[], &[]);
        assert!(
            report.warnings().any(|i| i.code == "dangling_base"),
            "{report}"
        );
    }

    #[test]
    fn a_derivation_pattern_round_trips_through_ron() {
        let pattern = compound_pattern();
        let text = ron::ser::to_string(&pattern).expect("serialise");
        let back: DerivationPattern = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, pattern);
    }

    #[test]
    fn a_misspelled_derivation_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(id: "X", name: "X", formashun: compound(pairs: []),
                       gloss: "{1}", part_of_speech: noun)"#;
        assert!(ron::from_str::<DerivationPattern>(text).is_err());
    }
}
