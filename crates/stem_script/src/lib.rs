//! Writing systems: glyphs, and the mapping from sound to sign (ROADMAP M20,
//! `DESIGN.md` §7.6).
//!
//! # Why script is a phase and not a font picker
//!
//! §7.6's claim is that **a glyph should have ancestry just like a word**. That is
//! what makes this worth building: a script is not a rendering of a language, it is a
//! second history running alongside it, and the two come apart. A glyph can outlive
//! the sound it once wrote; a spelling can freeze while a pronunciation moves. M20
//! gives a glyph the identity that lets it descend from something; M21 gives it the
//! descent.
//!
//! # The one thing this milestone must be honest about
//!
//! ROADMAP M20's acceptance: *the mapping is reported where it is lossy (an abjad
//! drops vowels — that is the point, and the tool says so rather than pretending
//! round-trip).*
//!
//! An abjad writing `ktb` for *kataba* has not failed. It has done exactly what an
//! abjad does, and a reader supplies the vowels from knowing the language. The
//! failure mode this module exists to avoid is a tool that either (a) invents vowel
//! signs so the round trip works, or (b) drops them silently so a user believes the
//! spelling is complete. So [`write`] returns what it **could not** write alongside
//! what it did, every time, and [`Written::is_lossy`] is a question with an answer.
//!
//! # Sound to sign, and one exception
//!
//! Four of the five script kinds map from **phonology**: an alphabet one glyph per
//! phoneme, an abjad the consonants only, an abugida a consonant with a vowel
//! diacritic, a syllabary one glyph per syllable-sized sequence. Logography maps from
//! **meaning** instead — which is why [`Mapping`] has a `Concept` variant, and why
//! this crate depends on `stem_lexicon` at all.
//!
//! Nothing here maps from *morphology*. §7.6 mentions it and it is real (Chinese
//! radicals, Egyptian determinatives), but a morphographic mapping needs a morpheme
//! to point at and this project's morphemes are language-scoped citation forms rather
//! than the shared components a determinative system uses. It waits for a milestone
//! that has one.

use serde::{Deserialize, Serialize};
use stem_core::{
    GlyphId, Issue, PhonemeId, Result, Severity, StemmaError, Validate, ValidationReport,
};
use stem_lexicon::{ConceptKey, Lexicon};
use stem_phonology::{PhonemeInventory, Root, SegmentKind};

/// What a glyph was *doing* at one point in its life (M21, §7.6).
///
/// §7.6's worked chain runs `pictogram → logogram → determinative → rebus → simplified
/// manuscript form → modern marker`, and these are its rungs. A sign changes job as
/// often as it changes shape, and the job is the more interesting half: the moment a
/// picture of a star stops meaning *star* and starts spelling /sa/ is the moment
/// writing becomes a phonographic technology.
///
/// `#[non_exhaustive]`, and the labels live here for the `ScriptKind` reason: a
/// downstream wildcard arm would hide a new rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GlyphRole {
    /// Not stated.
    #[default]
    Unspecified,
    /// A picture of the thing it names.
    Pictogram,
    /// A sign for a meaning — the picture, conventionalised past recognition.
    Logogram,
    /// A silent classifier: written, never read aloud, narrowing what follows.
    Determinative,
    /// A sign for a *sound*, borrowed from the meaning it used to carry. The rebus
    /// principle, and the hinge of the whole history — after it, a script can write
    /// words nobody has drawn a picture for.
    Rebus,
    /// A sign for a sound, plainly, with the borrowing forgotten.
    Phonogram,
    /// Neither sound nor meaning: kept because it has always been kept.
    Marker,
}

impl GlyphRole {
    /// Its name, for `stemma glyph-trace`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Pictogram => "pictogram",
            Self::Logogram => "logogram",
            Self::Determinative => "determinative",
            Self::Rebus => "rebus sign",
            Self::Phonogram => "phonogram",
            Self::Marker => "marker",
        }
    }
}

/// One stage of a glyph's life — the glyph analogue of a [`RuleApplicationTrace`].
///
/// [`RuleApplicationTrace`]: stem_soundchange::RuleApplicationTrace
///
/// # Why there is no year on it
///
/// M19's rule: *a trigger is verified, never asserted.* A glyph's biography is
/// authored prose about shapes — the engine cannot check any of it — so putting a date
/// on a stage would dress an assertion up as a measurement. The **order** is the
/// claim, and order is all §7.6's chain actually states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphStage {
    /// What the sign was doing then.
    #[serde(default)]
    pub role: GlyphRole,
    /// How it was drawn then. Empty means "as it is drawn now" — a stage that changed
    /// the *job* without changing the shape, which is most of them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub form: String,
    /// The sound it stood for at this stage, if any.
    ///
    /// This is the field that makes §7.6's independence claim checkable: a sign that
    /// once wrote /w/ can still be on the page long after no word contains /w/, and
    /// [`script_drift`] finds exactly that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrote: Option<PhonemeId>,
    /// The meaning it stood for at this stage, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meant: Option<ConceptKey>,
    /// Authorial prose: what happened, and why. Not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// One sign of one writing system.
///
/// Modelled the way a [`stem_phonology::Phoneme`] and a `Morpheme` are: an **id**, a
/// form, and a name — so it is an entity that can have a history rather than a
/// character in a string. M21's descent hangs from that id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    /// Stable identity within this script.
    pub id: GlyphId,
    /// How it is drawn, as text. Any string: a Unicode character, a digraph, or a
    /// transliteration stand-in for a shape that has no code point.
    ///
    /// **Not the identity.** A form changes over time — that is most of what M21 is
    /// about — while the glyph stays the same glyph, exactly as a word keeps its
    /// `WordId` through every sound change.
    pub form: String,
    /// A human name: `aleph`, `the ox-head`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Authorial prose. Not interpreted.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// What this sign used to be, **oldest first** (M21).
    ///
    /// Exactly `Derivation`'s shape and for exactly its reason: `history[0]` is the
    /// pictogram the way `Derivation::input` is the proto-form, the entries are the
    /// steps, and **the present is not in here** — the current shape is
    /// [`Self::form`] and the current job is whatever the script's `mappings` say.
    /// Storing the present twice is the desynchronisation M2 banned when it refused to
    /// keep a rendered string beside `phonemic_form`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<GlyphStage>,
}

impl Glyph {
    /// The oldest recorded stage — §7.6's pictogram, where there is one.
    pub fn origin(&self) -> Option<&GlyphStage> {
        self.history.first()
    }

    /// Every sound this sign is *recorded* as having written, oldest first, without
    /// repeats. The current mapping is not included: this is the past only.
    ///
    /// A `Vec` in recorded order, never a set reaching output (§9.4).
    pub fn sounds_it_once_wrote(&self) -> Vec<&PhonemeId> {
        let mut out: Vec<&PhonemeId> = Vec::new();
        for stage in &self.history {
            if let Some(id) = &stage.wrote
                && !out.contains(&id)
            {
                out.push(id);
            }
        }
        out
    }
}

/// What kind of writing system this is.
///
/// The kind is **not** what decides the mapping — the mapping is declared, glyph by
/// glyph. What the kind decides is what counts as *expected*: an abjad with no vowel
/// signs is doing its job, and an alphabet with none has a gap. Validation reads it
/// for exactly that, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ScriptKind {
    /// Not stated.
    #[default]
    Unspecified,
    /// One sign per phoneme, consonants and vowels alike.
    Alphabet,
    /// Consonants only; the reader supplies the vowels.
    Abjad,
    /// A consonant sign carrying an inherent vowel, modified for the others.
    Abugida,
    /// One sign per syllable-sized sequence.
    Syllabary,
    /// Signs for meanings rather than for sounds.
    Logography,
}

impl ScriptKind {
    /// Its name, for the report and the sketch.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unspecified => "unspecified",
            Self::Alphabet => "alphabet",
            Self::Abjad => "abjad",
            Self::Abugida => "abugida",
            Self::Syllabary => "syllabary",
            Self::Logography => "logography",
        }
    }

    /// Whether a sound going unwritten is **expected** of this kind.
    ///
    /// The one question the kind is consulted for, and the difference between "the
    /// script is doing its job" and "the author has a hole in their alphabet".
    ///
    /// - An **abjad** is *defined* by unwritten vowels.
    /// - An **abugida** writes them as marks on a consonant, which this v0 does not
    ///   model as a separate sign, so an unmapped vowel is likewise unremarkable.
    /// - A **logography** writes no sounds *at all* — the sign stands for the
    ///   meaning — so `is_vowel` does not enter into it. Answering this one per-vowel
    ///   would report a logography's consonants as gaps in a mapping that was never
    ///   phonographic.
    pub fn expects_unwritten(self, is_vowel: bool) -> bool {
        match self {
            Self::Logography => true,
            Self::Abjad | Self::Abugida => is_vowel,
            _ => false,
        }
    }
}

/// One rule of the sound-to-sign correspondence.
///
/// `#[non_exhaustive]` so a morphographic or positional variant is not a breaking
/// change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Mapping {
    /// One phoneme, one glyph.
    Phoneme {
        /// The sound.
        phoneme: PhonemeId,
        /// The sign.
        glyph: GlyphId,
    },
    /// A run of phonemes written with one glyph — a syllabary's `ka`, or a digraph.
    ///
    /// **Longest match wins** when several could apply, so a `ka` sign beats a `k`
    /// sign followed by an `a` sign. Same rule as `Root::parse`'s, and for the same
    /// reason: a script that could be read two ways at one position is one the author
    /// should hear about, not one the tool should guess at.
    Sequence {
        /// The sounds, in order.
        phonemes: Vec<PhonemeId>,
        /// The sign.
        glyph: GlyphId,
    },
    /// A meaning written with one glyph — logography.
    Concept {
        /// The meaning.
        concept: ConceptKey,
        /// The sign.
        glyph: GlyphId,
    },
}

/// One language's writing system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WritingSystem {
    /// Stable id, e.g. `"asterian_abjad"`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// What kind of script it is.
    #[serde(default)]
    pub kind: ScriptKind,
    /// Authorial prose: whose script it is, and when.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// The signs, in authored order — which is the order the sketch lists them and
    /// therefore part of the determinism contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub glyphs: Vec<Glyph>,
    /// The correspondence, in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mappings: Vec<Mapping>,
}

impl WritingSystem {
    /// The glyph with this id, or `None`. Linear over an authored `Vec`; a map is
    /// forbidden on any path that reaches output (§9.4), and these lists are small.
    pub fn glyph(&self, id: &GlyphId) -> Option<&Glyph> {
        self.glyphs.iter().find(|g| &g.id == id)
    }

    /// Every phoneme this script can write, in mapping order.
    ///
    /// Used by validation to find the gaps; a `Vec`, never a set reaching output.
    pub fn written_phonemes(&self) -> Vec<&PhonemeId> {
        let mut out = Vec::new();
        for mapping in &self.mappings {
            match mapping {
                Mapping::Phoneme { phoneme, .. } => out.push(phoneme),
                Mapping::Sequence { phonemes, .. } => out.extend(phonemes.iter()),
                Mapping::Concept { .. } => {}
            }
        }
        out
    }

    /// Whether this script has any recorded glyph history at all.
    pub fn has_history(&self) -> bool {
        self.glyphs.iter().any(|g| !g.history.is_empty())
    }

    /// Whether this script encodes **sound** at all.
    ///
    /// A logography does not, and that is not a deficiency — it is the property that
    /// let Chinese characters survive two millennia of phonological change unbothered.
    /// It is also why such a script cannot "fall behind a pronunciation": there is no
    /// pronunciation in it to fall behind. Asked of the mappings rather than of the
    /// `kind`, because the kind never decides what the mapping is (M20).
    pub fn writes_sound(&self) -> bool {
        self.mappings
            .iter()
            .any(|m| !matches!(m, Mapping::Concept { .. }))
    }

    /// Whether **this sign** stands for a sound under the current mapping.
    ///
    /// Lives here rather than in the renderer for the `ScriptKind::name` reason:
    /// `Mapping` is `#[non_exhaustive]`, so a downstream match needs a wildcard arm,
    /// and a wildcard would quietly answer `false` for a positional or morphographic
    /// variant added later. Inside the defining crate the compiler asks the question.
    pub fn glyph_writes_sound(&self, glyph: &GlyphId) -> bool {
        self.mappings.iter().any(|m| match m {
            Mapping::Phoneme { glyph: g, .. } | Mapping::Sequence { glyph: g, .. } => g == glyph,
            Mapping::Concept { .. } => false,
        })
    }
}

// ------------------------------------------------------------- script history

/// The number of separate sound/sign mismatches at or above which an orthography is
/// reported as **deep** (`DESIGN.md` §17's script-history row, M21).
///
/// **Read by both the profile band and the validation Note**, so the two cannot
/// disagree about when a spelling has fallen remarkably far behind its pronunciation
/// (`docs/adr/0009` — the fourth instance of that paired-constant rule).
///
/// Every living orthography has *some* drift; one or two facts is ordinary and must
/// not trip the Note. This is a deliberately loose tripwire for many at once, and —
/// like [`LONG_SENSE_CHAIN`](stem_lexicon::LONG_SENSE_CHAIN) — it is **not** a cited
/// typological constant. There is no attested "orthographic depth" scale, and
/// inventing one would be the fabrication §17 forbids.
///
/// The reference family's drifted daughter sits deliberately below it, by M8's rule
/// that the tool's own showcase must not trip the extreme bar.
pub const DEEP_ORTHOGRAPHY: usize = 8;

/// What the engine found when it held a script up against the language it writes.
///
/// # Both halves are findings
///
/// Nothing here is authored. The author writes a glyph's biography — which the engine
/// cannot check — and the engine measures whether the script still fits the language,
/// by reading the lexicon. That is M19's discipline carried into Phase 6: the
/// consequence is proposed by a human, the *trigger* is verified.
///
/// # Why the lexicon and not the inventory
///
/// `apply_rules` only ever **grows** an inventory — a phoneme stays in it after the
/// last word containing it has changed, so earlier trace steps keep resolving. So
/// "this language no longer has /w/" is a false question to ask of the inventory and a
/// true one to ask of the word list. Every count below comes from the lexicon.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptDrift {
    /// Sounds that occur in the lexicon and that this script has no sign for, in
    /// inventory order — the spelling could not keep up with the pronunciation.
    ///
    /// Excludes what the kind expects to leave unwritten: an abjad's vowels are its
    /// design, not its drift, and counting them would make every abjad look decayed.
    pub unwritable: Vec<PhonemeId>,
    /// Signs whose sound occurs in no word any more, in mapping order — the glyph
    /// outlived the sound it was made for. §7.6's claim, measured.
    pub fossils: Vec<GlyphId>,
    /// How many of the lexicon's words this script now writes incompletely. The raw
    /// basis, carried so the band can be checked rather than trusted — the
    /// `recorded_changes` discipline.
    pub affected_words: usize,
}

impl ScriptDrift {
    /// How many separate facts separate the spelling from the pronunciation: sounds
    /// with no sign, plus signs with no sound.
    pub fn distance(&self) -> usize {
        self.unwritable.len() + self.fossils.len()
    }

    /// Whether the spelling has fallen behind at all.
    pub fn is_historical(&self) -> bool {
        self.distance() > 0
    }
}

/// Holds `script` up against the language it writes, and reports the gap.
///
/// Pure, total and RNG-free. A script made for its language returns
/// [`ScriptDrift::default`] — every field empty — which is the true description of an
/// orthography that has not yet fallen behind.
///
/// # What this does not measure
///
/// A **logogram whose meaning the language has lost.** That is a real kind of fossil,
/// but it arrives through semantic drift (M9) rather than through the sound change
/// this milestone is about, and answering it properly needs sense reasoning rather
/// than a mapping lookup. `fossils` is phonographic signs only, and says so.
pub fn script_drift(
    script: &WritingSystem,
    lexicon: &Lexicon,
    inventory: &PhonemeInventory,
) -> ScriptDrift {
    // Which sounds the language still says. Authored inventory order, so the output
    // is stable and no map reaches it (§9.4).
    let occurring: Vec<&PhonemeId> = inventory
        .iter()
        .map(|p| &p.id)
        .filter(|id| {
            lexicon
                .iter()
                .any(|e| e.phonemic_form.segments().any(|s| s == *id))
        })
        .collect();
    let written = script.written_phonemes();

    // Half one: a sound the language says and the script cannot write.
    let unwritable: Vec<PhonemeId> = inventory
        .iter()
        .filter(|p| occurring.contains(&&p.id))
        .filter(|p| !written.contains(&&p.id))
        .filter(|p| !script.kind.expects_unwritten(p.kind == SegmentKind::Vowel))
        .map(|p| p.id.clone())
        .collect();

    // Half two: a sign for a sound the language no longer says. A `Sequence` counts as
    // fossil when *any* of its sounds is gone, because it can never match again.
    let mut fossils: Vec<GlyphId> = Vec::new();
    for mapping in &script.mappings {
        let (glyph, sounds): (&GlyphId, Vec<&PhonemeId>) = match mapping {
            Mapping::Phoneme { phoneme, glyph } => (glyph, vec![phoneme]),
            Mapping::Sequence { phonemes, glyph } => (glyph, phonemes.iter().collect()),
            // Deliberately not measured — see the doc comment above.
            Mapping::Concept { .. } => continue,
        };
        if sounds.iter().any(|id| !occurring.contains(id)) && !fossils.contains(glyph) {
            fossils.push(glyph.clone());
        }
    }

    let affected_words = lexicon
        .iter()
        .filter(|e| {
            e.phonemic_form
                .segments()
                .any(|s| unwritable.iter().any(|u| u == s))
        })
        .count();

    ScriptDrift {
        unwritable,
        fossils,
        affected_words,
    }
}

// -------------------------------------------------------------------- writing

/// One sign, written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenGlyph {
    /// Which glyph.
    pub glyph: GlyphId,
    /// Its form, echoed so a renderer needs no second lookup.
    pub form: String,
    /// The phonemes it stands for, in order. Empty for a logographic sign, which
    /// stands for a meaning rather than for sounds.
    pub covers: Vec<PhonemeId>,
}

/// A sound this script has no sign for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unwritten {
    /// The sound that went unwritten.
    pub phoneme: PhonemeId,
    /// Whether the script's kind **expects** this — an abjad's vowels.
    ///
    /// The difference between "the script is doing its job" and "the author has a
    /// hole in their alphabet", and the reason this is a field rather than a
    /// judgement made at the point of printing.
    pub expected: bool,
}

/// A word, written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The signs, in order.
    pub glyphs: Vec<WrittenGlyph>,
    /// The sounds that did not make it onto the page, in order.
    ///
    /// **Always reported.** A spelling that silently omitted them would let a reader
    /// believe the script round-trips when it does not, which is the one thing M20's
    /// acceptance forbids.
    pub unwritten: Vec<Unwritten>,
}

impl Written {
    /// The written form, as text.
    pub fn text(&self) -> String {
        self.glyphs.iter().map(|g| g.form.as_str()).collect()
    }

    /// Whether anything was lost — i.e. whether this spelling can be read back.
    pub fn is_lossy(&self) -> bool {
        !self.unwritten.is_empty()
    }
}

/// Writes `form` in `script`, `concept` being the word's meaning where it has one.
///
/// Pure, total and RNG-free. **Longest match first** over [`Mapping::Sequence`], then
/// single phonemes; a sound with no mapping is recorded in `unwritten` and the walk
/// continues, because a script that stopped at the first unwritable sound could not
/// spell anything at all in an abjad.
///
/// # The meaning is tried first
///
/// A [`Mapping::Concept`] for this word wins outright: one sign for the whole word,
/// covering no sounds, and **every** segment recorded as unwritten. That last clause
/// is the honest one — you cannot read a pronunciation off a logogram, and a spelling
/// that quietly claimed otherwise is the failure this module exists to prevent.
///
/// A word with no concept (a derived lexeme, an inflected cell) simply has no logogram
/// and falls through to the sounds. That is also how a mixed script behaves — a sign
/// for the stem, letters for the rest — though v0 goes no further than word-at-a-time.
///
/// # Failure
///
/// None. A word that cannot be written at all comes back with no glyphs and every
/// sound in `unwritten`, which is a true description of a script that cannot write
/// it — and a more useful one than an error, since the caller wanted to see the
/// spelling.
pub fn write(form: &Root, concept: Option<&ConceptKey>, script: &WritingSystem) -> Written {
    let segments: Vec<PhonemeId> = form.segments().cloned().collect();
    let mut glyphs = Vec::new();
    let mut unwritten = Vec::new();
    let mut at = 0usize;

    if let Some(key) = concept
        && let Some(glyph) = script.mappings.iter().find_map(|m| match m {
            Mapping::Concept { concept, glyph } if concept == key => Some(glyph),
            _ => None,
        })
    {
        return Written {
            glyphs: vec![WrittenGlyph {
                glyph: glyph.clone(),
                form: glyph_form(script, glyph),
                covers: Vec::new(),
            }],
            // `expected` is filled in by `write_with_inventory`, as below.
            unwritten: segments
                .into_iter()
                .map(|phoneme| Unwritten {
                    phoneme,
                    expected: false,
                })
                .collect(),
        };
    }

    // The longest sequence any mapping names, so the greedy window is bounded by the
    // data rather than by a constant.
    let longest = script
        .mappings
        .iter()
        .map(|m| match m {
            Mapping::Sequence { phonemes, .. } => phonemes.len(),
            _ => 1,
        })
        .max()
        .unwrap_or(1)
        .max(1);

    while at < segments.len() {
        let remaining = segments.len() - at;
        let matched = (1..=longest.min(remaining)).rev().find_map(|take| {
            let window = &segments[at..at + take];
            script.mappings.iter().find_map(|mapping| match mapping {
                Mapping::Sequence { phonemes, glyph } if phonemes.as_slice() == window => {
                    Some((take, glyph))
                }
                Mapping::Phoneme { phoneme, glyph } if take == 1 && phoneme == &window[0] => {
                    Some((take, glyph))
                }
                _ => None,
            })
        });

        match matched {
            Some((take, glyph)) => {
                glyphs.push(WrittenGlyph {
                    glyph: glyph.clone(),
                    form: glyph_form(script, glyph),
                    covers: segments[at..at + take].to_vec(),
                });
                at += take;
            }
            None => {
                unwritten.push(Unwritten {
                    phoneme: segments[at].clone(),
                    // Filled in by the caller, which has the inventory; see
                    // `write_with_inventory`.
                    expected: false,
                });
                at += 1;
            }
        }
    }

    Written { glyphs, unwritten }
}

/// How a glyph is drawn, or its id in angle brackets when the script names a sign it
/// never declared.
///
/// The missing-glyph case is reported by [`WritingSystem::validate`]; printing `<g_a>`
/// rather than nothing keeps the hole visible on the page instead of silently
/// shortening the word, which would read as a different spelling.
fn glyph_form(script: &WritingSystem, glyph: &GlyphId) -> String {
    script
        .glyph(glyph)
        .map(|g| g.form.clone())
        .unwrap_or_else(|| format!("<{glyph}>"))
}

/// The same, with the inventory in hand so an unwritten sound can say whether it was
/// **expected** to be unwritten.
///
/// Split from [`write`] rather than folded into it because the answer needs the
/// phoneme's `kind`, and a `Root` does not carry one — the same reason
/// `Lexicon::check_against_inventory` is a free function taking context rather than a
/// method.
pub fn write_with_inventory(
    form: &Root,
    concept: Option<&ConceptKey>,
    script: &WritingSystem,
    inventory: &PhonemeInventory,
) -> Written {
    let mut written = write(form, concept, script);

    // A logogram carries the meaning, so its sounds going unwritten is the design. A
    // logography with **no sign for this word** wrote nothing at all, and calling that
    // "by design" would let a blank line pass as a spelling — the same lie as an abjad
    // that dropped its vowels quietly. So the kind's general answer is refined here by
    // the per-word fact, which only the writer knows.
    let wrote_a_sign = !written.glyphs.is_empty();
    for gap in &mut written.unwritten {
        let is_vowel = inventory
            .get(&gap.phoneme)
            .is_some_and(|p| p.kind == SegmentKind::Vowel);
        gap.expected = match script.kind {
            ScriptKind::Logography => wrote_a_sign,
            kind => kind.expects_unwritten(is_vowel),
        };
    }
    written
}

/// A one-line statement of what this spelling does and does not carry.
///
/// The acceptance's "the tool says so" clause, in one place so every front end says
/// it the same way.
pub fn lossiness(written: &Written, script: &WritingSystem) -> String {
    if !written.is_lossy() {
        return "every sound is written; this spelling can be read back exactly.".to_owned();
    }
    // Nothing reached the page. Distinct from a lossy spelling and it must read as
    // distinct: a logography can only write the words that earned a sign, and an empty
    // line explained as "what a logography does" would suggest a reader could recover
    // something from it. There is nothing there to recover from.
    if written.glyphs.is_empty() {
        return format!(
            "`{}` has no sign for this word, so none of its {} sound(s) reached the \
             page at all. Nothing was written — which is not the same as something \
             written incompletely.",
            script.id,
            written.unwritten.len()
        );
    }
    let expected = written.unwritten.iter().filter(|u| u.expected).count();
    let unexpected = written.unwritten.len() - expected;

    match (expected, unexpected) {
        (n, 0) => format!(
            "{n} sound(s) are not written, which is what {} does — a reader supplies \
             them from knowing the language. This spelling does not round-trip, and is \
             not meant to.",
            an(script.kind)
        ),
        (0, n) => format!(
            "{n} sound(s) have no sign in this script at all. That is a gap in the \
             mapping rather than a property of {}, and `stemma validate` names each one.",
            an(script.kind)
        ),
        (e, u) => format!(
            "{e} sound(s) are left unwritten by design and {u} have no sign at all; \
             the second kind is a gap in the mapping, and `stemma validate` names them."
        ),
    }
}

fn an(kind: ScriptKind) -> String {
    match kind {
        ScriptKind::Abjad | ScriptKind::Abugida | ScriptKind::Alphabet => {
            format!("an {}", kind.name())
        }
        other => format!("a {}", other.name()),
    }
}

// ---------------------------------------------------------------- validation

impl Validate for WritingSystem {
    /// Structural checks that need nothing but the script itself.
    ///
    /// A duplicate glyph id is an **Error**: two signs under one identity make every
    /// mapping ambiguous, and it is the same fault class as `duplicate_word_id`.
    /// Everything else here reports (§17).
    fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();

        if self.glyphs.is_empty() {
            report.note("no_glyphs", "this script declares no signs yet");
        }
        for (i, glyph) in self.glyphs.iter().enumerate() {
            if self.glyphs[..i].iter().any(|g| g.id == glyph.id) {
                report.push(
                    Issue::new(
                        Severity::Error,
                        "duplicate_glyph_id",
                        format!(
                            "two glyphs share the id `{}`; every mapping naming it \
                             becomes ambiguous",
                            glyph.id
                        ),
                    )
                    .about(&glyph.id),
                );
            }
            if glyph.form.is_empty() {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "glyph_without_form",
                        format!(
                            "glyph `{}` has no written form, so it prints as nothing",
                            glyph.id
                        ),
                    )
                    .about(&glyph.id),
                );
            }
        }

        for mapping in &self.mappings {
            let glyph = match mapping {
                Mapping::Phoneme { glyph, .. }
                | Mapping::Sequence { glyph, .. }
                | Mapping::Concept { glyph, .. } => glyph,
            };
            if self.glyph(glyph).is_none() {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "unknown_glyph",
                        format!(
                            "a mapping names glyph `{glyph}`, which this script does not \
                             declare; it will write as `<{glyph}>`"
                        ),
                    )
                    .about(glyph),
                );
            }
            if let Mapping::Sequence { phonemes, .. } = mapping
                && phonemes.is_empty()
            {
                report.push(
                    Issue::new(
                        Severity::Warning,
                        "empty_sequence",
                        format!("glyph `{glyph}` is mapped from no sounds, so it is unreachable"),
                    )
                    .about(glyph),
                );
            }
        }

        // M21: a glyph's biography. Authored prose the engine cannot check — so the
        // only checks possible are structural, and both are Notes. Judging whether
        // `pictogram → marker` is a *plausible* path would need a typology of script
        // change this project does not have and §20.1 fences out.
        for glyph in &self.glyphs {
            if glyph.history.is_empty() {
                continue;
            }
            if glyph
                .history
                .first()
                .is_some_and(|s| s.role != GlyphRole::Pictogram)
            {
                report.note(
                    "history_without_a_pictogram",
                    format!(
                        "`{}` records a history that does not begin as a pictogram; \
                         `stemma glyph-trace` will walk back only as far as it goes",
                        glyph.id
                    ),
                );
            }
            // A stage that changed neither shape nor job nor value records nothing.
            for (i, stage) in glyph.history.iter().enumerate() {
                if stage.form.is_empty()
                    && stage.wrote.is_none()
                    && stage.meant.is_none()
                    && stage.role == GlyphRole::Unspecified
                {
                    report.note(
                        "empty_glyph_stage",
                        format!(
                            "stage {i} of `{}` records no shape, role, sound or meaning, \
                             so it says nothing about what changed",
                            glyph.id
                        ),
                    );
                }
            }
        }

        report
    }
}

/// The checks that need the language's phoneme inventory too.
///
/// A free function taking context, for the reason `check_against_inventory` is one:
/// `Validate::validate(&self)` takes no arguments, and a script cannot know which
/// sounds exist without being told.
///
/// - `unknown_phoneme` — a mapping names a sound this language does not have. A
///   **Warning**: the mapping is inert, so the script silently writes less than its
///   author thinks.
/// - `unwritten_phoneme` — a sound with no sign. A **Note** when the kind expects it
///   (an abjad's vowels), a **Warning** when it does not (a hole in an alphabet).
/// - `lossy_script` — a Note stating the script cannot round-trip, and why.
pub fn check_against_inventory(
    script: &WritingSystem,
    inventory: &PhonemeInventory,
) -> ValidationReport {
    let mut report = ValidationReport::new();

    for phoneme in script.written_phonemes() {
        if inventory.get(phoneme).is_none() {
            report.push(
                Issue::new(
                    Severity::Warning,
                    "unknown_phoneme",
                    format!(
                        "`{}` maps `{phoneme}`, which is not in this language's inventory, \
                         so the mapping can never apply",
                        script.id
                    ),
                )
                .about(phoneme),
            );
        }
    }

    let written = script.written_phonemes();
    let mut expected_gaps = 0usize;
    for phoneme in inventory.iter() {
        if written.contains(&&phoneme.id) {
            continue;
        }
        let expected = script
            .kind
            .expects_unwritten(phoneme.kind == SegmentKind::Vowel);
        if expected {
            expected_gaps += 1;
            continue;
        }
        report.push(
            Issue::new(
                Severity::Warning,
                "unwritten_phoneme",
                format!(
                    "/{}/ has no sign in `{}`, so every word containing it is written \
                     incompletely",
                    phoneme.ipa, script.id
                ),
            )
            .about(&phoneme.id),
        );
    }

    if expected_gaps > 0 {
        // Two different silences, and they must not borrow each other's explanation:
        // an abjad declines to write the vowels, a logography does not write sounds at
        // all.
        let what = if script.kind == ScriptKind::Logography {
            "writes meanings rather than sounds"
        } else {
            "writes no vowels"
        };
        report.note(
            "lossy_script",
            format!(
                "`{}` {what}, which is what {} does — a spelling in it cannot be read \
                 back to one pronunciation, and that is the design rather than a fault",
                script.id,
                an(script.kind)
            ),
        );
    }

    report
}

/// The M21 check: has this script fallen behind the language it writes?
///
/// A free function taking the lexicon, for the reason [`check_against_inventory`] is
/// one. Everything it reports is a **finding**, computed by [`script_drift`], and every
/// severity is a Note or a Warning — an orthography drifting from its pronunciation is
/// the normal fate of writing, not a defect (§17).
///
/// - `sign_outlived_its_sound` — a Note per fossil. §7.6's independence claim, found.
/// - `spelling_behind_pronunciation` — a Warning naming the sounds with no sign. A
///   Warning rather than a Note because unlike an abjad's vowels these are words the
///   script genuinely cannot spell, and the author may not know.
/// - `deep_orthography` — a Note when the distance reaches [`DEEP_ORTHOGRAPHY`], the
///   constant the profile band is paired to.
pub fn check_against_lexicon(
    script: &WritingSystem,
    lexicon: &Lexicon,
    inventory: &PhonemeInventory,
) -> ValidationReport {
    let mut report = ValidationReport::new();
    let drift = script_drift(script, lexicon, inventory);

    for glyph in &drift.fossils {
        let once = script
            .glyph(glyph)
            .and_then(|g| g.origin())
            .map(|s| format!(", drawn first as {}", s.role.name()))
            .unwrap_or_default();
        report.note(
            "sign_outlived_its_sound",
            format!(
                "`{glyph}` writes a sound no word of this language contains any more{once}; \
                 the sign is still on the page after the sound it was made for is gone"
            ),
        );
    }

    if !drift.unwritable.is_empty() {
        let sounds: Vec<String> = drift
            .unwritable
            .iter()
            .map(|id| {
                inventory
                    .get(id)
                    .map(|p| format!("/{}/", p.ipa))
                    .unwrap_or_else(|| id.to_string())
            })
            .collect();
        report.push(
            Issue::new(
                Severity::Warning,
                "spelling_behind_pronunciation",
                format!(
                    "`{}` has no sign for {} — {} word(s) can no longer be written in full",
                    script.id,
                    sounds.join(" "),
                    drift.affected_words
                ),
            )
            .about(&script.id),
        );
    }

    if drift.distance() >= DEEP_ORTHOGRAPHY {
        report.note(
            "deep_orthography",
            format!(
                "`{}` and this language have drifted apart on {} separate counts; the \
                 spelling is now historical rather than phonetic",
                script.id,
                drift.distance()
            ),
        );
    }

    report
}

/// Resolves a script by id, or the first one when no id is given.
///
/// Errors rather than guessing when the named script is absent: a language may have
/// several, and quietly writing in the wrong one would be worse than saying so.
pub fn resolve<'a>(scripts: &'a [WritingSystem], id: Option<&str>) -> Result<&'a WritingSystem> {
    match id {
        Some(wanted) => scripts
            .iter()
            .find(|s| s.id == wanted)
            .ok_or_else(|| StemmaError::not_found("writing system", wanted)),
        None => scripts
            .first()
            .ok_or_else(|| StemmaError::not_found("writing system", "(this language has none)")),
    }
}

/// Resolves a glyph by id across every script, for `stemma glyph-trace`.
///
/// A `GlyphId` is **script-scoped** — two scripts may legitimately name a sign `g_a` —
/// so an unqualified id can be ambiguous. When it is, this errors and names the scripts
/// rather than picking one, the [`resolve`] discipline: tracing the wrong sign's
/// biography would be worse than being asked which one.
pub fn resolve_glyph<'a>(
    scripts: &'a [WritingSystem],
    glyph: &str,
    script: Option<&str>,
) -> Result<(&'a WritingSystem, &'a Glyph)> {
    let id = GlyphId::new(glyph);

    if let Some(wanted) = script {
        let system = resolve(scripts, Some(wanted))?;
        let found = system
            .glyph(&id)
            .ok_or_else(|| StemmaError::not_found("glyph in that script", glyph))?;
        return Ok((system, found));
    }

    let hits: Vec<(&WritingSystem, &Glyph)> = scripts
        .iter()
        .filter_map(|s| s.glyph(&id).map(|g| (s, g)))
        .collect();
    match hits.len() {
        0 => Err(StemmaError::not_found("glyph", glyph)),
        1 => Ok(hits[0]),
        _ => {
            let names: Vec<&str> = hits.iter().map(|(s, _)| s.id.as_str()).collect();
            Err(StemmaError::not_found(
                "glyph (the id is used by several scripts; name one with --script)",
                format!("{glyph} — in {}", names.join(", ")),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stem_phonology::{Phoneme, Syllable};

    fn inventory() -> PhonemeInventory {
        PhonemeInventory::from_phonemes([
            Phoneme::new("ph_k", "k", SegmentKind::Consonant),
            Phoneme::new("ph_t", "t", SegmentKind::Consonant),
            Phoneme::new("ph_b", "b", SegmentKind::Consonant),
            Phoneme::new("ph_a", "a", SegmentKind::Vowel),
            Phoneme::new("ph_i", "i", SegmentKind::Vowel),
        ])
    }

    fn glyph(id: &str, form: &str) -> Glyph {
        Glyph {
            id: GlyphId::new(id),
            form: form.to_owned(),
            name: String::new(),
            note: String::new(),
            history: Vec::new(),
        }
    }

    fn phoneme_map(phoneme: &str, glyph: &str) -> Mapping {
        Mapping::Phoneme {
            phoneme: PhonemeId::new(phoneme),
            glyph: GlyphId::new(glyph),
        }
    }

    /// An alphabet: a sign for every sound.
    fn alphabet() -> WritingSystem {
        WritingSystem {
            id: "alpha".to_owned(),
            name: "The alphabet".to_owned(),
            kind: ScriptKind::Alphabet,
            note: String::new(),
            glyphs: vec![
                glyph("g_k", "K"),
                glyph("g_t", "T"),
                glyph("g_b", "B"),
                glyph("g_a", "A"),
                glyph("g_i", "I"),
            ],
            mappings: vec![
                phoneme_map("ph_k", "g_k"),
                phoneme_map("ph_t", "g_t"),
                phoneme_map("ph_b", "g_b"),
                phoneme_map("ph_a", "g_a"),
                phoneme_map("ph_i", "g_i"),
            ],
        }
    }

    /// A logography: signs for meanings, and none for sounds.
    fn logography() -> WritingSystem {
        WritingSystem {
            id: "logo".to_owned(),
            name: "The signs".to_owned(),
            kind: ScriptKind::Logography,
            note: String::new(),
            glyphs: vec![glyph("g_write", "書")],
            mappings: vec![Mapping::Concept {
                concept: ConceptKey::new("WRITE"),
                glyph: GlyphId::new("g_write"),
            }],
        }
    }

    /// An abjad: consonants only.
    fn abjad() -> WritingSystem {
        WritingSystem {
            id: "abjad".to_owned(),
            name: "The abjad".to_owned(),
            kind: ScriptKind::Abjad,
            note: String::new(),
            glyphs: vec![glyph("g_k", "k"), glyph("g_t", "t"), glyph("g_b", "b")],
            mappings: vec![
                phoneme_map("ph_k", "g_k"),
                phoneme_map("ph_t", "g_t"),
                phoneme_map("ph_b", "g_b"),
            ],
        }
    }

    /// `kataba`.
    fn word() -> Root {
        Root {
            syllables: vec![Syllable {
                pattern: "CVCVCV".to_owned(),
                segments: ["ph_k", "ph_a", "ph_t", "ph_a", "ph_b", "ph_a"]
                    .iter()
                    .map(|s| PhonemeId::new(*s))
                    .collect(),
                stress: None,
            }],
        }
    }

    // --------------------------------------------------------------- writing

    #[test]
    fn an_alphabet_writes_every_sound_and_round_trips() {
        let written = write_with_inventory(&word(), None, &alphabet(), &inventory());
        assert_eq!(written.text(), "KATABA");
        assert!(!written.is_lossy());
        assert!(lossiness(&written, &alphabet()).contains("read back exactly"));
    }

    /// **ROADMAP M20's acceptance.** An abjad drops the vowels — that is the point —
    /// and the tool says so rather than pretending round-trip.
    #[test]
    fn an_abjad_drops_the_vowels_and_says_that_it_did() {
        let written = write_with_inventory(&word(), None, &abjad(), &inventory());
        assert_eq!(
            written.text(),
            "ktb",
            "kataba, written as an abjad writes it"
        );

        assert!(written.is_lossy());
        assert_eq!(written.unwritten.len(), 3, "three /a/s");
        assert!(
            written.unwritten.iter().all(|u| u.expected),
            "an abjad not writing vowels is the design, not a gap"
        );

        let sentence = lossiness(&written, &abjad());
        assert!(sentence.contains("what an abjad does"), "{sentence}");
        assert!(
            sentence.contains("not meant to"),
            "the tool must say the round trip is not intended: {sentence}"
        );
    }

    /// A *hole* in an alphabet reads differently from an abjad's design, and the
    /// distinction is the whole reason `Unwritten::expected` is a field.
    #[test]
    fn a_hole_in_an_alphabet_is_reported_differently_from_an_abjads_design() {
        let mut holed = alphabet();
        holed.mappings.retain(
            |m| !matches!(m, Mapping::Phoneme { phoneme, .. } if phoneme.as_str() == "ph_a"),
        );

        let written = write_with_inventory(&word(), None, &holed, &inventory());
        assert!(written.is_lossy());
        assert!(
            written.unwritten.iter().all(|u| !u.expected),
            "an alphabet is supposed to write its vowels"
        );
        let sentence = lossiness(&written, &holed);
        assert!(sentence.contains("gap in the mapping"), "{sentence}");
    }

    /// Longest match first: a `ka` sign beats `k` followed by `a`.
    #[test]
    fn a_syllabary_sign_wins_over_its_own_first_sound() {
        let mut syllabary = alphabet();
        syllabary.kind = ScriptKind::Syllabary;
        syllabary.glyphs.push(glyph("g_ka", "㋕"));
        // Authored *after* the single-phoneme mappings, to prove the win is by
        // length rather than by declaration order.
        syllabary.mappings.push(Mapping::Sequence {
            phonemes: vec![PhonemeId::new("ph_k"), PhonemeId::new("ph_a")],
            glyph: GlyphId::new("g_ka"),
        });

        let written = write_with_inventory(&word(), None, &syllabary, &inventory());
        assert_eq!(written.text(), "㋕TABA");
        assert_eq!(written.glyphs[0].covers.len(), 2, "one sign, two sounds");
    }

    #[test]
    fn a_glyph_a_mapping_names_but_the_script_lacks_prints_visibly() {
        let mut broken = alphabet();
        broken.glyphs.retain(|g| g.id.as_str() != "g_a");
        let written = write(&word(), None, &broken);
        assert!(
            written.text().contains("<g_a>"),
            "a missing glyph must be visible on the page, not silently shorten the \
             word: {}",
            written.text()
        );
    }

    /// A logogram writes the **meaning**: one sign, no sounds covered, and every
    /// segment recorded as unwritten — because you cannot read a pronunciation off it.
    ///
    /// The variant existed from the first draft and nothing consumed it, which would
    /// have made `kind: logography` a label that changed nothing observable.
    #[test]
    fn a_logogram_writes_the_meaning_and_carries_none_of_the_sounds() {
        let key = ConceptKey::new("WRITE");
        let written = write_with_inventory(&word(), Some(&key), &logography(), &inventory());

        assert_eq!(written.text(), "書", "one sign for the whole word");
        assert!(
            written.glyphs[0].covers.is_empty(),
            "a logogram stands for a meaning, not for a run of sounds"
        );
        assert_eq!(
            written.unwritten.len(),
            6,
            "all six segments went unwritten"
        );
        assert!(
            written.unwritten.iter().all(|u| u.expected),
            "a logography not writing sounds is the design, consonants included"
        );
        assert!(
            lossiness(&written, &logography()).contains("what a logography does"),
            "{}",
            lossiness(&written, &logography())
        );
    }

    /// The meaning is tried **first**: a script carrying both kinds of mapping spells
    /// the word with its sign rather than letter by letter.
    #[test]
    fn a_word_with_a_sign_of_its_own_is_written_with_it_rather_than_spelled_out() {
        let mut mixed = alphabet();
        mixed.glyphs.push(glyph("g_write", "書"));
        mixed.mappings.push(Mapping::Concept {
            concept: ConceptKey::new("WRITE"),
            glyph: GlyphId::new("g_write"),
        });

        let key = ConceptKey::new("WRITE");
        assert_eq!(
            write(&word(), Some(&key), &mixed).text(),
            "書",
            "the sign wins over the letters"
        );
        assert_eq!(
            write(&word(), None, &mixed).text(),
            "KATABA",
            "a word with no concept falls through to the sounds"
        );
    }

    /// A logography can only write the words that earned a sign. When it has none, it
    /// wrote **nothing** — and that must not read as "unwritten by design", which would
    /// let a blank line pass for a spelling a reader could complete.
    #[test]
    fn a_word_the_logography_has_no_sign_for_is_unwritable_rather_than_lossy() {
        let key = ConceptKey::new("STONE");
        let written = write_with_inventory(&word(), Some(&key), &logography(), &inventory());

        assert!(written.glyphs.is_empty(), "nothing reached the page");
        assert!(
            written.unwritten.iter().all(|u| !u.expected),
            "there was no sign, so no design to appeal to"
        );

        let sentence = lossiness(&written, &logography());
        assert!(sentence.contains("has no sign for this word"), "{sentence}");
        assert!(
            !sentence.contains("what a logography does"),
            "an unwritable word must not borrow the design's explanation: {sentence}"
        );
    }

    /// A logography's unmapped **consonants** are not holes either. Answering the
    /// question per-vowel would report ten gaps in a mapping that was never
    /// phonographic.
    #[test]
    fn a_logography_is_not_reported_as_a_script_full_of_holes() {
        let report = check_against_inventory(&logography(), &inventory());
        assert!(
            report.warnings().next().is_none(),
            "a logography writes no sounds by definition: {report}"
        );
        let note = report
            .issues
            .iter()
            .find(|i| i.code == "lossy_script")
            .unwrap_or_else(|| panic!("{report}"));
        assert!(
            note.message.contains("writes meanings rather than sounds"),
            "an abjad's explanation must not be reused for it: {}",
            note.message
        );
    }

    // --------------------------------------------------------- script history

    /// A lexicon of one word, `kata`: /b/ and /i/ are in the inventory and in no word,
    /// which is exactly the state a sound change leaves behind and what makes a sign
    /// for either of them a fossil.
    fn drifted_lexicon() -> Lexicon {
        word_list(&[["ph_k", "ph_a", "ph_t", "ph_a"]])
    }

    /// A lexicon that uses **every** sound in the inventory, so nothing is stranded and
    /// a script made for the language reports no drift at all.
    fn whole_lexicon() -> Lexicon {
        word_list(&[
            ["ph_k", "ph_a", "ph_t", "ph_a"],
            ["ph_b", "ph_i", "ph_t", "ph_i"],
        ])
    }

    fn word_list(words: &[[&str; 4]]) -> Lexicon {
        // Built through `authored_word`, the only place a hand-added word is
        // constructed (M16) — so these tests cannot drift from how the rest of the
        // project makes one.
        let entries = words
            .iter()
            .enumerate()
            .map(|(i, segments)| {
                stem_lexicon::authored_word(
                    &stem_core::LanguageId::new("test"),
                    i + 1,
                    Root {
                        syllables: vec![Syllable {
                            pattern: "CVCV".to_owned(),
                            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
                            stress: None,
                        }],
                    },
                    "a word",
                    None,
                    stem_lexicon::PartOfSpeech::Noun,
                )
            })
            .collect::<Vec<_>>();
        Lexicon::from_entries(entries)
    }

    /// The two halves of drift, on one script: a sound with no sign, and a sign whose
    /// sound is gone.
    #[test]
    fn drift_finds_both_a_sound_with_no_sign_and_a_sign_with_no_sound() {
        // Drop /t/'s letter, so a sound the word contains cannot be written.
        let mut script = alphabet();
        script.mappings.retain(
            |m| !matches!(m, Mapping::Phoneme { phoneme, .. } if phoneme.as_str() == "ph_t"),
        );

        let drift = script_drift(&script, &drifted_lexicon(), &inventory());
        assert_eq!(
            drift.unwritable,
            vec![PhonemeId::new("ph_t")],
            "the word says /t/ and the script cannot write it"
        );
        assert!(
            drift.fossils.iter().any(|g| g.as_str() == "g_b"),
            "and `B` writes a /b/ no word contains: {:?}",
            drift.fossils
        );
        assert_eq!(drift.affected_words, 1);
        assert!(drift.is_historical());
    }

    /// **An abjad's unwritten vowels are its design, not its decay.** Counting them
    /// would make every abjad ever written look like a decayed alphabet.
    #[test]
    fn an_abjads_vowels_are_never_counted_as_drift() {
        let drift = script_drift(&abjad(), &whole_lexicon(), &inventory());
        assert!(
            drift.unwritable.is_empty(),
            "both vowels are spoken and both go unwritten by design: {:?}",
            drift.unwritable
        );
        assert!(!drift.is_historical(), "{drift:?}");
    }

    /// A script made for its language has not drifted, and the measure says zero rather
    /// than finding something to report.
    #[test]
    fn a_script_that_still_fits_its_language_reports_no_drift() {
        assert_eq!(
            script_drift(&alphabet(), &whole_lexicon(), &inventory()).distance(),
            0,
            "every sound has a letter and every letter has a sound"
        );
    }

    /// A logography writes meanings, so no pronunciation can leave it behind — the
    /// property that let logographic scripts outlive the pronunciations around them.
    #[test]
    fn a_logography_cannot_drift_from_a_pronunciation_it_never_encoded() {
        assert!(!logography().writes_sound());
        let drift = script_drift(&logography(), &whole_lexicon(), &inventory());
        assert!(!drift.is_historical(), "{drift:?}");
    }

    /// The fossil finding must come from the **lexicon**: `apply_rules` only grows an
    /// inventory, so asking it whether the language still has /b/ answers yes forever.
    #[test]
    fn a_fossil_is_found_even_though_the_sound_is_still_in_the_inventory() {
        let inventory = inventory();
        assert!(
            inventory.get(&PhonemeId::new("ph_b")).is_some(),
            "the inventory keeps it"
        );
        let drift = script_drift(&alphabet(), &drifted_lexicon(), &inventory);
        assert!(
            drift.fossils.iter().any(|g| g.as_str() == "g_b"),
            "and the sign is a fossil anyway: {:?}",
            drift.fossils
        );
    }

    /// A glyph keeps its recorded past in order, and the walk stops at the pictogram.
    #[test]
    fn a_glyph_remembers_what_it_used_to_be() {
        let mut sign = glyph("g_k", "K");
        sign.history = vec![
            GlyphStage {
                role: GlyphRole::Pictogram,
                form: "⌂".to_owned(),
                wrote: None,
                meant: Some(ConceptKey::new("HOUSE")),
                note: String::new(),
            },
            GlyphStage {
                role: GlyphRole::Rebus,
                form: String::new(),
                wrote: Some(PhonemeId::new("ph_k")),
                meant: None,
                note: String::new(),
            },
        ];

        assert_eq!(sign.origin().expect("oldest").role, GlyphRole::Pictogram);
        assert_eq!(sign.sounds_it_once_wrote(), vec![&PhonemeId::new("ph_k")]);
        assert_eq!(
            sign.history[1].form, "",
            "an empty form means the shape did not change at that stage"
        );
    }

    /// Drift is **reported, never policed**: every issue is a Note or a Warning, and a
    /// drifted orthography is still a valid one.
    #[test]
    fn a_drifted_script_is_never_an_error() {
        let mut script = alphabet();
        script.mappings.retain(
            |m| !matches!(m, Mapping::Phoneme { phoneme, .. } if phoneme.as_str() == "ph_t"),
        );
        let report = check_against_lexicon(&script, &drifted_lexicon(), &inventory());

        assert!(report.is_ok(), "{report}");
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == "sign_outlived_its_sound"),
            "{report}"
        );
        assert!(
            report
                .warnings()
                .any(|i| i.code == "spelling_behind_pronunciation"),
            "{report}"
        );
        assert!(
            !report.issues.iter().any(|i| i.code == "deep_orthography"),
            "two mismatches is ordinary, well under the bar of {DEEP_ORTHOGRAPHY}: {report}"
        );
    }

    #[test]
    fn writing_twice_produces_an_identical_result() {
        assert_eq!(
            write_with_inventory(&word(), None, &abjad(), &inventory()),
            write_with_inventory(&word(), None, &abjad(), &inventory())
        );
    }

    // ------------------------------------------------------------ validation

    #[test]
    fn a_duplicate_glyph_id_is_an_error() {
        let mut script = alphabet();
        script.glyphs.push(glyph("g_k", "K'"));
        let report = script.validate();
        assert!(
            report.errors().any(|i| i.code == "duplicate_glyph_id"),
            "{report}"
        );
    }

    #[test]
    fn a_mapping_naming_an_undeclared_glyph_is_reported() {
        let mut script = alphabet();
        script.mappings.push(phoneme_map("ph_k", "g_nowhere"));
        assert!(
            script
                .validate()
                .warnings()
                .any(|i| i.code == "unknown_glyph")
        );
    }

    /// An abjad's unwritten vowels are a **Note** naming the design; an alphabet's
    /// are a **Warning** naming a hole. Same fact, two readings, decided by `kind`.
    #[test]
    fn an_unwritten_vowel_reads_as_design_or_as_a_hole_depending_on_the_kind() {
        let abjad_report = check_against_inventory(&abjad(), &inventory());
        assert!(
            abjad_report.warnings().next().is_none(),
            "an abjad not writing vowels must not be a warning: {abjad_report}"
        );
        assert!(
            abjad_report.issues.iter().any(|i| i.code == "lossy_script"),
            "{abjad_report}"
        );

        let mut holed = alphabet();
        holed.mappings.retain(
            |m| !matches!(m, Mapping::Phoneme { phoneme, .. } if phoneme.as_str() == "ph_a"),
        );
        let alphabet_report = check_against_inventory(&holed, &inventory());
        assert!(
            alphabet_report
                .warnings()
                .any(|i| i.code == "unwritten_phoneme"),
            "{alphabet_report}"
        );
    }

    #[test]
    fn a_complete_alphabet_is_quiet() {
        let report = check_against_inventory(&alphabet(), &inventory());
        assert!(report.issues.is_empty(), "{report}");
    }

    #[test]
    fn a_mapping_for_a_sound_this_language_lacks_is_reported() {
        let mut script = alphabet();
        script.glyphs.push(glyph("g_z", "Z"));
        script.mappings.push(phoneme_map("ph_z", "g_z"));
        assert!(
            check_against_inventory(&script, &inventory())
                .warnings()
                .any(|i| i.code == "unknown_phoneme")
        );
    }

    // ---------------------------------------------------------- resolution

    #[test]
    fn a_script_resolves_by_id_or_defaults_to_the_first() {
        let scripts = vec![alphabet(), abjad()];
        assert_eq!(resolve(&scripts, None).expect("first").id, "alpha");
        assert_eq!(resolve(&scripts, Some("abjad")).expect("named").id, "abjad");
        let err = resolve(&scripts, Some("nowhere")).expect_err("no such script");
        assert!(err.to_string().contains("nowhere"), "{err}");
    }

    #[test]
    fn a_language_with_no_script_says_so_rather_than_panicking() {
        let err = resolve(&[], None).expect_err("no scripts");
        assert!(err.to_string().contains("has none"), "{err}");
    }

    // ------------------------------------------------------------ round trip

    #[test]
    fn a_writing_system_round_trips_through_ron() {
        let script = abjad();
        let text = ron::ser::to_string(&script).expect("serialise");
        let back: WritingSystem = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, script);
    }

    #[test]
    fn a_misspelled_script_field_fails_to_load_rather_than_defaulting() {
        assert!(ron::from_str::<WritingSystem>(r#"(id: "x", name: "X", glyfs: [])"#).is_err());
    }
}
