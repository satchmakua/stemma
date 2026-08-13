//! One entry in one language's lexicon.

use serde::{Deserialize, Serialize};
use stem_core::{CognateSetId, Result, WordId};
use stem_phonology::{PhonemeInventory, Root};

use crate::concept::{Concept, ConceptKey, PartOfSpeech, concept};

/// How a word came to be in this language.
///
/// Two variants because M2 produces exactly two. M4's fork does **not** add
/// `Inherited`: it copies each entry's `source` verbatim (a forked word keeps
/// being what it was — authored or generated — in the daughter), so there is
/// still no producer for an `Inherited` variant. It arrives, with `Borrowed`
/// (M7's contact) and `Derived` (M8's morphology), when a producer writes a
/// value the copy cannot; shipping either now would be scaffolding
/// (`docs/adr/0008`).
///
/// A unit variant, deliberately not `Generated { draw: u32 }`. The draw index *is*
/// the entry's ordinal under the draw contract in [`crate::build`], so storing it
/// stores a derivable value — the same defect class as storing a rendered form. If
/// per-word draw provenance is ever wanted it arrives as a separate
/// `#[serde(default)] draw: Option<u32>` **field on [`WordEntry`]**, which is
/// additive; a unit variant that later grows a payload is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WordSource {
    /// Drawn by the proto-lexicon builder from this language's phonotactics.
    Generated,
    /// Written by hand in the project file.
    ///
    /// The default, so a hand-authored entry may omit the field and still say
    /// something true. A generated lexicon always writes `generated` explicitly.
    #[default]
    Authored,
    /// Built by inflection from morphemes (M8's `inflect`). The variant `word.rs`
    /// reserved for morphology now has its producer.
    Derived,
}

impl WordSource {
    /// The lowercase name, as it appears in RON and in exports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Authored => "authored",
            Self::Derived => "derived",
        }
    }
}

impl std::fmt::Display for WordSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One entry in one language's lexicon (`DESIGN.md` §8.3).
///
/// §8.3 lists fourteen fields; M2 ships six of them and adds one (`concept`) the
/// design's list omits. Each deferral is argued in `docs/adr/0007`. The short
/// version: nothing on disk refers to a `WordEntry` field by position and nothing
/// orders by field, so appending one cannot change what an M2 file means.
///
/// `deny_unknown_fields` for the reason [`stem_genome::LanguageGenome`] gives: a
/// misspelled `cognate_st:` must be a load error, not a silently defaulted empty
/// string that quietly severs a word from its family. It does not conflict with
/// the `#[serde(default)]` rule — `default` is what makes an *old* file load in
/// *new* code, which is the direction that rule is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordEntry {
    /// Stable identity **within one language**.
    ///
    /// Two sister languages both holding `w_0007` is correct and expected — they
    /// are different words. Cross-language identity is [`WordEntry::cognate_set`]'s
    /// job, and conflating the two is what breaks the comparative view. Minted by
    /// `WordId::sequential`, never randomly (`docs/adr/0003`).
    pub id: WordId,

    /// The meaning this word was coined for, as a key into [`crate::concept`].
    ///
    /// Not in §8.3's field list, and added deliberately: §8.3 offers `glosses`
    /// (free strings, presentational, and they drift) and `semantic_nodes` (M9,
    /// and `SemanticNodeId` does not exist and must not be invented here). Neither
    /// can serve as the cross-language meaning key §10.3's cognate table needs.
    ///
    /// `Option` because `None` is honest for a word that is not on the list — a
    /// hand-authored item, and M7's innovations. Every word M2 generates has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept: Option<ConceptKey>,

    /// The phonemic form: this language's phonemes, syllabified.
    ///
    /// **Not a `String`.** §3.3's traceability constraint requires a form to be
    /// walkable back to its segments, and M3's rules match feature bundles
    /// resolved from `PhonemeId`s. The syllabification is carried rather than
    /// recomputed because M3's "final unstressed vowel loss" is defined over the
    /// last syllable, and a flat segment vector destroys that boundary
    /// *irrecoverably* — the template was a random choice, not a derivable
    /// property of the output.
    ///
    /// The written and IPA strings are **views**: [`WordEntry::written`] and
    /// [`WordEntry::ipa`]. §8.3's separate `form` field is deliberately not stored,
    /// because a second source of truth for a form desynchronises the first time
    /// M3 mutates a segment, and nothing can detect it.
    pub phonemic_form: Root,

    /// Glosses for this entry, most salient first, overriding the concept's own.
    ///
    /// Empty means "use the concept's gloss", which is true of every entry M2
    /// generates — so a hundred identical strings stay out of the project file.
    /// Free text, presentational. M9's semantic nodes attach *alongside* these
    /// rather than replacing them: a dictionary a human reads still wants a gloss
    /// once the semantics are modelled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glosses: Vec<String>,

    /// The part of speech **in this language**. Seeded from the concept at
    /// generation, freely overridable. See [`PartOfSpeech`].
    pub part_of_speech: PartOfSpeech,

    /// The family-wide descent class — **the thread that survives forking** (§8.6).
    ///
    /// Equal across two entries exactly when they descend from one and the same
    /// proto-entry. Minted once, at proto-lexicon creation, scoped to the minting
    /// language; copied verbatim by M4's fork and never re-minted. Never derived
    /// from the form — a form changes under every rule — and never from a
    /// per-language counter, which restarts on fork. The precise invariant is in
    /// `docs/adr/0007` and restated in this crate's docs.
    pub cognate_set: CognateSetId,

    /// How this entry came to exist.
    #[serde(default)]
    pub source: WordSource,

    /// This word's recorded derivation (`docs/adr/0007`'s deferred `trace`,
    /// arriving in exactly the shape that ADR promised).
    ///
    /// `None` means this word has never been passed through a rule. ADR-0007: "a
    /// proto-word has no derivation, so an empty trace on one is not a missing
    /// trace, it is a category error." An `Option` says that; a bare `Vec` erases
    /// it.
    ///
    /// `Some(Derivation { steps: [], .. })` is the genuinely different third
    /// state: exposed to the whole rule sequence and changed by none of it. The
    /// acceptance fixture's `*sawel` is exactly that, and only this encoding can
    /// say both.
    ///
    /// No proto file changes a byte: `skip_serializing_if` keeps the field out of
    /// every file that never evolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<crate::trace::Derivation>,

    /// The morphemes this word is composed of (`docs/adr/0007`'s deferred
    /// `morphemes`, arriving at M8). Empty for a monomorphemic root — every M0–M7
    /// entry and both reference fixtures — so `#[serde(default)]` keeps every
    /// saved file loading and `skip_serializing_if` keeps them byte-identical.
    ///
    /// The §3.3 obligation made concrete: a *composed* form with an empty
    /// `morphemes` would be a form with no recorded composition — the same bug
    /// class as a transformed form with no `RuleApplicationTrace`. `inflect`
    /// always writes it; `apply_rules`/`fork` carry it verbatim (`entry.clone()`).
    /// It records the decomposition (which morpheme, which span), never surface
    /// segments — those live in `phonemic_form`, and storing them twice is the
    /// desync `docs/adr/0007` forbids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub morphemes: Vec<crate::morpheme::MorphemeRef>,

    /// The **words** this word was derived from (M14): its etymology.
    ///
    /// Empty for everything that is not a compound or an affixed derivative — every
    /// M0–M13 entry, both reference fixtures, and every inflected cell — so
    /// `#[serde(default)]` keeps every saved file loading and `skip_serializing_if`
    /// keeps them byte-identical.
    ///
    /// The word-level twin of [`Self::morphemes`], and the two are complementary
    /// rather than alternative: an affixed derivative like `walk` + `-ri` records the
    /// **base** here and the **affix** there, because one is a lexicon entry and the
    /// other is a declared morpheme. A non-empty `bases` is also what distinguishes a
    /// derived lexeme from an inflected cell, both of which are
    /// [`WordSource::Derived`] — the distinction is visible in the record, so it is
    /// not also stored as a variant.
    ///
    /// Like a `MorphemeRef`, a [`BaseRef`] holds a span into the composition form and
    /// **no surface segments**; the surface is recovered through
    /// [`Self::morpheme_surface`], which is what keeps a compound's parts on the
    /// record after a sound change has eroded them past recognition.
    ///
    /// [`BaseRef`]: crate::derivation::BaseRef
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bases: Vec<crate::derivation::BaseRef>,

    /// The senses this word currently holds, **most salient first** (M9).
    ///
    /// The meaning twin of [`Self::phonemic_form`]: the current state, stored, with
    /// the record that produced it in [`Self::sense_history`]. That the two are
    /// redundant is deliberate, and it is the same redundancy `phonemic_form` and
    /// `trace` already carry — pinned by the §16.3 property that
    /// `sense_history.final_senses()` equals these ids as a set.
    ///
    /// Empty for every pre-M9 word — both reference fixtures included — so
    /// `skip_serializing_if` keeps their bytes identical. §8.3 sketches a bare
    /// `semantic_nodes: Vec<SemanticNodeId>`; this ships the id *with its gloss*
    /// echoed, so [`Self::display_gloss`] stays context-free and the sound-change
    /// engine never has to name a semantic type (`docs/adr/0011`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senses: Vec<crate::semantics::SenseRef>,

    /// This word's recorded semantic history (§3.3, for meaning — M9).
    ///
    /// `None` means no drift event has ever named this word. Unlike [`Self::trace`]
    /// there is **no empty-steps third state**: a sound-change rule is word-general,
    /// so every word in the lexicon was *exposed* to it and an empty derivation says
    /// something true ("exposed, unchanged"). A drift event names **one word**, so a
    /// word no event named was never exposed, and an empty history on it would be a
    /// claim that never happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sense_history: Option<crate::semantics::SenseHistory>,
}

impl WordEntry {
    /// The concept record, or `None` when the entry names no concept or names one
    /// the built-in list does not hold. `validate` reports the latter at Warning
    /// severity, with a suggestion.
    pub fn concept(&self) -> Option<&'static Concept> {
        self.concept.as_ref().and_then(concept)
    }

    /// The gloss to display: this entry's first non-blank **sense** (M9), else its
    /// first non-blank override, else the concept's own.
    ///
    /// `Option`, not a `""` fallback. An entry with none of the three is the one
    /// genuinely broken state here — a dictionary headword with no meaning — and it
    /// is an Error (`lexicon.no_gloss`), so a caller that must render is entitled to
    /// fail loudly rather than print a blank. No `unwrap_or_default`.
    ///
    /// # Why the sense outranks the override (M9)
    ///
    /// A daughter inherits `glosses` **verbatim** through `fork`, so if the authored
    /// override won, a drifted meaning would be invisible in every view that reads
    /// this — the cognate table, `by_meaning`, the dictionary. Drift must be able to
    /// shadow an inherited label, and shadowing is already this method's pinned
    /// behaviour (an override shadows the concept's gloss, which is how `*rekan`
    /// displays "king" over concept MAN).
    ///
    /// It **shadows, never overwrites**: `apply_drift` does not touch `glosses`, so
    /// the authored label is still on disk and still wins the moment the sense is
    /// removed. An engine that clobbered an authored field would be indistinguishable
    /// afterwards from the author.
    ///
    /// # Why it takes no context
    ///
    /// `stem_soundchange::render_derivation` calls this. A signature taking a
    /// `&SemanticSpace` would make the sound-change engine name a semantic type,
    /// spending exactly the separation `docs/adr/0010` bought — which is why a
    /// [`crate::semantics::SenseRef`] echoes its gloss.
    pub fn display_gloss(&self) -> Option<&str> {
        self.senses
            .iter()
            .map(|s| s.gloss.as_str())
            .find(|g| !g.trim().is_empty())
            .or_else(|| {
                self.glosses
                    .iter()
                    .map(String::as_str)
                    .find(|g| !g.trim().is_empty())
            })
            .or_else(|| self.concept().map(|c| c.gloss))
    }

    /// The romanised form.
    ///
    /// `Result` via `PhonemeInventory::require` — silently rendering a missing
    /// segment as nothing would corrupt the word.
    pub fn written(&self, inventory: &PhonemeInventory) -> Result<String> {
        self.phonemic_form.written(inventory)
    }

    /// The IPA form.
    pub fn ipa(&self, inventory: &PhonemeInventory) -> Result<String> {
        self.phonemic_form.ipa(inventory)
    }

    /// The syllable shape at generation, `"CV.CVC"`.
    ///
    /// Presentation only — see [`stem_phonology::Syllable`]'s docs on why nothing
    /// reads `pattern` for semantics.
    pub fn template(&self) -> String {
        self.phonemic_form
            .syllables
            .iter()
            .map(|s| s.pattern.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Every segment of the form, flattened.
    pub fn segments(&self) -> impl Iterator<Item = &stem_core::PhonemeId> {
        self.phonemic_form.segments()
    }

    /// The surface segments of the morpheme occupying the composition span
    /// `[start, end)` — one morpheme's realised allomorph in this word (M8).
    ///
    /// **This is the single place that encodes the invariant `Derivation.input` IS
    /// the composition form.** When the word has evolved, the span is walked through
    /// the recorded rules by [`crate::trace::Derivation::surface_of_input_span`];
    /// when it has not, `phonemic_form` *is* the composition form (no rule has moved
    /// a segment), so the span is a raw slice of it. Both the allomorph measure
    /// ([`crate::morphological_irregularity`]) and the paradigm renderer read a
    /// morpheme's surface through here, so the allomorph counts they report cannot
    /// diverge — the shared-source discipline `docs/adr/0009` requires of any two
    /// values that must agree.
    pub fn morpheme_surface(&self, start: usize, end: usize) -> Vec<stem_core::PhonemeId> {
        match &self.trace {
            Some(trace) => trace.surface_of_input_span(start, end),
            None => self
                .phonemic_form
                .segments()
                .skip(start)
                .take(end.saturating_sub(start))
                .cloned()
                .collect(),
        }
    }
}
