//! Morphemes, concatenative composition, paradigms, and the allomorph measure
//! (ROADMAP M8, `DESIGN.md` §7.3, `M8-SPEC`).
//!
//! # Why morphology lives here and needs no engine change
//!
//! A morpheme's `form` is a [`Root`], exactly as a [`crate::WordEntry`]'s is;
//! composition **concatenates syllable lists** into one `Root`. A morpheme boundary
//! is therefore just a segment adjacency, and `stem_soundchange`'s environment scan
//! already runs over flat indices *across* syllable boundaries (its `apply.rs`
//! module doc: "the /k/ of `ta.ka.la` is intervocalic across two boundaries"). So an
//! inflected cell is an ordinary `WordEntry`, and running it through `apply_rules`
//! gives cross-boundary sound change with **zero** change to the engine — preserving
//! "`apply_rules` is a pure, RNG-free function", the strongest determinism claim in
//! the project. `apply-rules`, `fork`, `trace`, `cognates`, and export all work on a
//! cell for free.
//!
//! # The one thing M8 exists to show
//!
//! *A regular paradigm becomes irregular purely as a consequence of an ordered sound
//! change, and the trace explains why.* A plural suffix `-ka` attaches regularly to
//! every stem; intervocalic voicing fires after vowel-final stems (`…a.ka…` →
//! `…a.ɡa…`) and not after consonant-final ones (`…n.ka` is not intervocalic),
//! splitting the one regular suffix into two surface allomorphs `-ɡa` / `-ka`. That
//! split *is* the irregular paradigm, and each cell's stored [`Derivation`] says
//! which rule fired where. This module supplies composition and the allomorph
//! *count*; the split itself is produced by the untouched engine.
//!
//! # v0 is concatenative, and deliberately small
//!
//! `DESIGN.md` §20.1 names scope explosion as the top risk, and morphology is where
//! it is most tempting. v0 is **prefix\* · stem · suffix\*** and nothing else — no
//! infixation, reduplication, templatic/introflexive or fusional exponence, no
//! grammaticalization (a morpheme changing *role* over time is later-milestone
//! diachronic morphology; here morphemes are static and sound change acts on their
//! *forms*), no typed morphosyntactic features (labels are free strings), no
//! typological profile, no syntax. Those are named in `docs/adr/0010` so the fence
//! is explicit.

use serde::{Deserialize, Serialize};
use stem_core::{LanguageId, MorphemeId, PhonemeId, Result, StemmaError, WordId};
use stem_phonology::{Root, Syllable};

use crate::build::scoped_cognate_set;
use crate::concept::PartOfSpeech;
use crate::lexicon::Lexicon;
use crate::word::{WordEntry, WordSource};

/// Where a morpheme sits relative to the stem.
///
/// v0 is concatenative, so the enum names only **linear positions**.
/// `#[non_exhaustive]` so `Infix`/`Reduplicant` can be added later without a
/// breaking change — but none is implemented now, and nothing on disk refers to a
/// variant by index, so appending one cannot change what a stored file means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MorphemeRole {
    /// The lexical core an affix attaches to.
    Stem,
    /// Attaches before the stem.
    Prefix,
    /// Attaches after the stem.
    Suffix,
}

/// A minimal unit of form and meaning (`DESIGN.md` §7.3), v0.
///
/// `form` is the **underlying** shape — the affix's citation form before any sound
/// change. A rule is what gives an affix its surface *allomorphs*; storing those
/// here would be a second source of truth that desynchronises the trace, the same
/// argument that keeps a rendered `form` string off [`crate::WordEntry`]
/// (`docs/adr/0007`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Morpheme {
    /// Stable identity within one language. Two sisters both holding `m_plural` is
    /// correct — they are different morphemes (`docs/adr/0003`).
    pub id: MorphemeId,

    /// Its linear position relative to a stem.
    pub role: MorphemeRole,

    /// A stem's gloss (`"star"`) or an affix's feature label (`"PL"`, `"PST"`).
    pub gloss: String,

    /// The underlying form, syllabified. Patterns are provenance only (the
    /// [`Syllable`] contract) — composition reads `segments`.
    pub form: Root,

    /// The part of speech a stem's inflected cells carry. A stem is a noun by
    /// default (most stems are, and the reference paradigm inflects nouns for
    /// number); an affix's value is unread, so its default is harmless.
    #[serde(default = "PartOfSpeech::noun_default")]
    pub part_of_speech: PartOfSpeech,
}

/// One morpheme occurrence in a composed word, tagged with the flat half-open span
/// `[start, end)` it occupies in the **composition (pre-sound-change) form**.
///
/// That composition form equals [`Derivation::input`] once the word is evolved, and
/// `input` never changes (a later stratum extends `steps`, not `input`), so a
/// morpheme's surface allomorph stays recoverable however deep the lineage — via
/// [`Derivation::surface_of_input_span`]. This is the "morphological decomposition"
/// `DESIGN.md` §10.2 renders (`tira-PL`); it stores **no** surface segments — those
/// live in `phonemic_form`, and storing them twice is the desync `docs/adr/0007`
/// forbids.
///
/// [`Derivation`]: crate::trace::Derivation
/// [`Derivation::input`]: crate::trace::Derivation::input
/// [`Derivation::surface_of_input_span`]: crate::trace::Derivation::surface_of_input_span
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MorphemeRef {
    /// Which morpheme.
    pub morpheme: MorphemeId,
    /// Its role, echoed so the decomposition renders without a morpheme lookup.
    pub role: MorphemeRole,
    /// Its gloss, echoed for the same reason — the ref is self-contained.
    pub gloss: String,
    /// Flat start index into the composition form (inclusive).
    pub start: u32,
    /// Flat end index into the composition form (exclusive).
    pub end: u32,
}

/// One language's morphology: its morphemes and the paradigms over them.
///
/// Both are ordered `Vec`s, never maps — order is part of the determinism contract
/// (`DESIGN.md` §9.4), and it is the order stems become table rows and cells become
/// columns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Morphology {
    /// The morpheme inventory, in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub morphemes: Vec<Morpheme>,
    /// The paradigms, in authored order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paradigms: Vec<Paradigm>,
}

impl Morphology {
    /// True when there is nothing to model — every pre-M8 file, both reference
    /// fixtures. `LanguageGenome` uses it to keep those files byte-identical.
    pub fn is_empty(&self) -> bool {
        self.morphemes.is_empty() && self.paradigms.is_empty()
    }

    /// The morpheme with this id, or `None`.
    pub fn morpheme(&self, id: &MorphemeId) -> Option<&Morpheme> {
        self.morphemes.iter().find(|m| &m.id == id)
    }

    /// The paradigm with this id, or `None`.
    pub fn paradigm(&self, id: &str) -> Option<&Paradigm> {
        self.paradigms.iter().find(|p| p.id == id)
    }
}

/// A table of one grammatical dimension: its stems (rows) inflected across its cells
/// (columns).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paradigm {
    /// A short id, e.g. `"NUMBER"`.
    pub id: String,
    /// A display name, e.g. `"Number"`.
    pub name: String,
    /// The stem morphemes to inflect, in row order.
    pub stems: Vec<MorphemeId>,
    /// The cells to inflect them into, in column order.
    pub cells: Vec<ParadigmCell>,
}

/// One cell of a paradigm: a label and the affixes that realise it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParadigmCell {
    /// The cell label, e.g. `"SG"`, `"PL"`.
    pub label: String,
    /// The affixes applied for this cell. **Empty means a zero-exponent cell** —
    /// the bare stem, which is how the singular of the reference paradigm is
    /// realised.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affixes: Vec<MorphemeId>,
}

/// Concatenates a stem and its affixes into one composition `Root`, plus a
/// [`MorphemeRef`] per morpheme carrying the flat span it occupies.
///
/// Surface order is **prefixes (authored order) · stem · suffixes (authored
/// order)**; each affix is placed by its own [`Morpheme::role`]. Pure, total and
/// RNG-free — mechanical concatenation. The composed `Root` carries the
/// concatenation of the morphemes' own syllables (`tira` = ti.ra plus `-ka` = ka →
/// ti.ra.ka); patterns go stale (nothing reads them, per the [`Syllable`] contract)
/// and no resyllabifier runs (unchanged from M3), but the segments are correct and
/// their adjacency crosses the boundary, so the suffix's `/k/` is intervocalic with
/// no special handling.
///
/// Every emitted syllable is `stress: None`: prosody is assigned once per whole word
/// inside `apply_rules`, so a morpheme's own citation stress must not leak in and
/// pre-empt it.
///
/// The returned refs are in surface (left-to-right) order, which is also span order,
/// so a renderer reads `tira-PL` straight off them. This takes the stem's own role
/// as authoritative rather than a redundant caller-supplied role — the deviation
/// from `M8-SPEC` §3's tuple signature is noted in `PROGRESS.md`.
pub fn compose(stem: &Morpheme, affixes: &[&Morpheme]) -> (Root, Vec<MorphemeRef>) {
    // Surface order: prefixes, then the stem, then everything else (suffixes).
    // `inflect` guarantees affixes are Prefix/Suffix, so a non-prefix is a suffix.
    let ordered: Vec<&Morpheme> = affixes
        .iter()
        .copied()
        .filter(|m| m.role == MorphemeRole::Prefix)
        .chain(std::iter::once(stem))
        .chain(
            affixes
                .iter()
                .copied()
                .filter(|m| m.role != MorphemeRole::Prefix),
        )
        .collect();

    let mut syllables: Vec<Syllable> = Vec::new();
    let mut refs: Vec<MorphemeRef> = Vec::new();
    let mut cursor: u32 = 0;
    for morpheme in ordered {
        let len = morpheme.form.len() as u32;
        for syllable in &morpheme.form.syllables {
            syllables.push(Syllable {
                pattern: syllable.pattern.clone(),
                segments: syllable.segments.clone(),
                stress: None,
            });
        }
        refs.push(MorphemeRef {
            morpheme: morpheme.id.clone(),
            role: morpheme.role,
            gloss: morpheme.gloss.clone(),
            start: cursor,
            end: cursor + len,
        });
        cursor += len;
    }

    (Root { syllables }, refs)
}

/// Materialises a paradigm into one [`WordEntry`] per (stem × cell), stem-major /
/// cell-minor — the **regular**, pre-sound-change forms.
///
/// Pure and deterministic (mechanical concatenation and positional ordinals, no
/// RNG). Each cell becomes a full `WordEntry`, so it flows through `apply_rules`,
/// `fork` and export exactly like any word — which is what makes cross-boundary
/// sound change fall out for free rather than needing a bespoke "inflect that
/// evolves".
///
/// Per cell: `id` = `WordId::sequential(ordinal)`; `phonemic_form` and `morphemes`
/// come from [`compose`] (the §3.3 composition record); `glosses` = `["{stem gloss}
/// {cell label}"]`, e.g. `"star PL"`; `part_of_speech` from the stem; `source` =
/// [`WordSource::Derived`]; `trace` = `None` (no rule has run yet).
///
/// # Cognate sets
///
/// Each `(stem, cell)` mints its **own** set via [`scoped_cognate_set`] — the only
/// sanctioned mint site. `a.cognate_set == b.cognate_set` must hold iff `a` and `b`
/// descend from one and the same proto entry (`docs/adr/0007`), and `tira-SG` and
/// `tira-PL` are *different* entries (different forms, different meanings). Distinct
/// sets, each copied verbatim by `fork`, so daughter A's `tira-PL` and daughter B's
/// `tira-PL` are cognate while SG and PL are not.
///
/// # v0 replaces, it does not append
///
/// The caller replaces the lexicon with these cells (the `new-lexicon` "replace,
/// never append" rule); the morphology fixture's base lexicon is empty, so the
/// ordinals `1..=N` never collide with an existing word. Appending onto a concept
/// lexicon is a later refinement.
///
/// # Failure
///
/// Resolves each [`MorphemeId`] against `morphemes`; a missing id, a `stems` entry
/// that is not a `Stem`, or an affix slot pointing at a `Stem` is a
/// [`StemmaError::NotFound`] — a spec bug surfaced, never a silent drop.
pub fn inflect(
    paradigm: &Paradigm,
    morphemes: &[Morpheme],
    language: &LanguageId,
) -> Result<Vec<WordEntry>> {
    let mut entries: Vec<WordEntry> = Vec::new();
    let mut ordinal: usize = 0;

    for stem_id in &paradigm.stems {
        let stem = resolve(morphemes, stem_id, Slot::Stem)?;
        for cell in &paradigm.cells {
            ordinal += 1;

            let affixes: Vec<&Morpheme> = cell
                .affixes
                .iter()
                .map(|id| resolve(morphemes, id, Slot::Affix))
                .collect::<Result<Vec<_>>>()?;

            let (form, refs) = compose(stem, &affixes);
            entries.push(WordEntry {
                id: WordId::sequential(ordinal),
                concept: None,
                phonemic_form: form,
                glosses: vec![format!("{} {}", stem.gloss, cell.label)],
                part_of_speech: stem.part_of_speech,
                cognate_set: scoped_cognate_set(language, ordinal),
                source: WordSource::Derived,
                trace: None,
                morphemes: refs,
                // An inflected cell starts with no modelled sense and no drift
                // history — its gloss comes from the stem's own label. M9 drift
                // then acts on it like any other entry (`docs/adr/0010`: a
                // `Derived` cell is an ordinary word, never a special case).
                senses: Vec::new(),
                sense_history: None,
            });
        }
    }

    Ok(entries)
}

/// Which kind of slot a [`MorphemeId`] is being resolved for.
#[derive(Clone, Copy)]
enum Slot {
    Stem,
    Affix,
}

/// Looks up a morpheme and checks its role fits the slot.
fn resolve<'a>(morphemes: &'a [Morpheme], id: &MorphemeId, slot: Slot) -> Result<&'a Morpheme> {
    let found = morphemes.iter().find(|m| &m.id == id);
    match (found, slot) {
        // A stem slot wants role Stem; an affix slot wants anything but Stem.
        (Some(m), Slot::Stem) if m.role == MorphemeRole::Stem => Ok(m),
        (Some(m), Slot::Affix) if m.role != MorphemeRole::Stem => Ok(m),
        // Found but mis-roled, or absent entirely — both are "no such usable
        // morpheme here", named by the slot so the message is actionable.
        (_, Slot::Stem) => Err(StemmaError::not_found("stem morpheme", id)),
        (_, Slot::Affix) => Err(StemmaError::not_found("affix morpheme", id)),
    }
}

// ---------------------------------------------------------- the allomorph measure

/// One affix morpheme's realisation across the words that use it: its distinct
/// surface allomorphs, deduped preserving first-seen (stored) order.
///
/// A `Vec`, never a set reaching output (`DESIGN.md` §9.4). The plural suffix of the
/// M8 demo has two allomorphs — `[ph_k, ph_a]` and `[ph_g, ph_a]` — so its count is
/// 2 and it is irregular.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllomorphSet {
    /// Which affix morpheme.
    pub morpheme: MorphemeId,
    /// Its label, echoed for rendering.
    pub gloss: String,
    /// Its distinct surface realisations, in first-appearance order. Each is a
    /// segment vector (`[ph_g, ph_a]`), not a rendered string — rendering needs an
    /// inventory the measure deliberately does not take.
    pub allomorphs: Vec<Vec<PhonemeId>>,
}

impl AllomorphSet {
    /// How many distinct surface allomorphs this affix has.
    pub fn count(&self) -> usize {
        self.allomorphs.len()
    }

    /// True when the affix surfaces more than one way — an irregular exponent.
    pub fn is_irregular(&self) -> bool {
        self.allomorphs.len() > 1
    }
}

/// The count at or above which an affix's allomorphy is reported as *extreme*
/// (`DESIGN.md` §17).
///
/// **Read by both the profile band and the validation Note**, so the two cannot
/// disagree about when allomorphy is remarkable (`docs/adr/0009`). A two-way
/// alternation — the M8 demo, and the overwhelmingly common case in real languages
/// — is unremarkable and must not trip the Note; three or more distinct surface
/// forms of one exponent is where §17's "extreme irregularity" begins.
pub const HIGH_ALLOMORPH_COUNT: usize = 3;

/// Every affix morpheme in the lexicon, with its distinct surface allomorphs, in
/// first-appearance order.
///
/// Reads only `WordEntry.morphemes` (the composition spans) and `WordEntry.trace`
/// (via [`Derivation::surface_of_input_span`]); it consults no rule and no
/// inventory. Stems are skipped — a stem is not an allomorph question in v0. An
/// unevolved cell (no trace) contributes the raw composition slice, so a regular
/// paradigm reports exactly one allomorph per affix; the split into two appears only
/// once the conditioning sound change has run.
///
/// [`Derivation::surface_of_input_span`]: crate::trace::Derivation::surface_of_input_span
pub fn morphological_irregularity(lexicon: &Lexicon) -> Vec<AllomorphSet> {
    let mut sets: Vec<AllomorphSet> = Vec::new();

    for entry in lexicon.iter() {
        for reference in &entry.morphemes {
            if reference.role == MorphemeRole::Stem {
                continue;
            }
            // The affix's realised allomorph in this word. `morpheme_surface`
            // carries the "trace ? walk-the-span : raw-slice" invariant in one
            // place, shared with `render_paradigm`, so the two cannot disagree
            // about a count (`docs/adr/0009`).
            let surface = entry.morpheme_surface(reference.start as usize, reference.end as usize);

            // Find or create this affix's set, preserving first-appearance order.
            let slot = match sets
                .iter_mut()
                .position(|s| s.morpheme == reference.morpheme)
            {
                Some(i) => &mut sets[i],
                None => {
                    sets.push(AllomorphSet {
                        morpheme: reference.morpheme.clone(),
                        gloss: reference.gloss.clone(),
                        allomorphs: Vec::new(),
                    });
                    sets.last_mut().expect("just pushed")
                }
            };
            if !slot.allomorphs.contains(&surface) {
                slot.allomorphs.push(surface);
            }
        }
    }

    sets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{Derivation, RuleApplication, SiteTrace};
    use stem_core::RuleId;

    fn syllable(pattern: &str, segments: &[&str]) -> Syllable {
        Syllable {
            pattern: pattern.to_owned(),
            segments: segments.iter().map(|s| PhonemeId::new(*s)).collect(),
            stress: None,
        }
    }

    /// `tira` = ti.ra, a vowel-final noun stem.
    fn stem_tira() -> Morpheme {
        Morpheme {
            id: MorphemeId::new("m_tira"),
            role: MorphemeRole::Stem,
            gloss: "star".to_owned(),
            form: Root {
                syllables: vec![
                    syllable("CV", &["ph_t", "ph_i"]),
                    syllable("CV", &["ph_r", "ph_a"]),
                ],
            },
            part_of_speech: PartOfSpeech::Noun,
        }
    }

    /// `tan` = tan, a consonant-final noun stem.
    fn stem_tan() -> Morpheme {
        Morpheme {
            id: MorphemeId::new("m_tan"),
            role: MorphemeRole::Stem,
            gloss: "man".to_owned(),
            form: Root {
                syllables: vec![syllable("CVC", &["ph_t", "ph_a", "ph_n"])],
            },
            part_of_speech: PartOfSpeech::Noun,
        }
    }

    /// `-ka` = ka, the plural suffix.
    fn suffix_plural() -> Morpheme {
        Morpheme {
            id: MorphemeId::new("m_plural"),
            role: MorphemeRole::Suffix,
            gloss: "PL".to_owned(),
            form: Root {
                syllables: vec![syllable("CV", &["ph_k", "ph_a"])],
            },
            part_of_speech: PartOfSpeech::Noun,
        }
    }

    fn number_paradigm() -> Paradigm {
        Paradigm {
            id: "NUMBER".to_owned(),
            name: "Number".to_owned(),
            stems: vec![MorphemeId::new("m_tira"), MorphemeId::new("m_tan")],
            cells: vec![
                ParadigmCell {
                    label: "SG".to_owned(),
                    affixes: vec![],
                },
                ParadigmCell {
                    label: "PL".to_owned(),
                    affixes: vec![MorphemeId::new("m_plural")],
                },
            ],
        }
    }

    fn segs(root: &Root) -> Vec<&str> {
        root.segments().map(|s| s.as_str()).collect()
    }

    #[test]
    fn compose_concatenates_a_suffix_after_the_stem_with_a_correct_span() {
        let (form, refs) = compose(&stem_tira(), &[&suffix_plural()]);
        assert_eq!(
            segs(&form),
            ["ph_t", "ph_i", "ph_r", "ph_a", "ph_k", "ph_a"]
        );
        assert_eq!(refs.len(), 2, "one ref for the stem, one for the suffix");
        // Surface order: stem then suffix.
        assert_eq!(refs[0].morpheme.as_str(), "m_tira");
        assert_eq!((refs[0].start, refs[0].end), (0, 4));
        assert_eq!(refs[1].morpheme.as_str(), "m_plural");
        assert_eq!(
            (refs[1].start, refs[1].end),
            (4, 6),
            "the suffix occupies the two segments after the four-segment stem"
        );
    }

    #[test]
    fn compose_places_a_prefix_before_the_stem() {
        let prefix = Morpheme {
            id: MorphemeId::new("m_pre"),
            role: MorphemeRole::Prefix,
            gloss: "DEF".to_owned(),
            form: Root {
                syllables: vec![syllable("V", &["ph_a"])],
            },
            part_of_speech: PartOfSpeech::Noun,
        };
        let (form, refs) = compose(&stem_tan(), &[&prefix]);
        assert_eq!(segs(&form), ["ph_a", "ph_t", "ph_a", "ph_n"]);
        // Surface order: prefix first, then the stem.
        assert_eq!(refs[0].morpheme.as_str(), "m_pre");
        assert_eq!((refs[0].start, refs[0].end), (0, 1));
        assert_eq!(refs[1].morpheme.as_str(), "m_tan");
        assert_eq!((refs[1].start, refs[1].end), (1, 4));
    }

    #[test]
    fn compose_emits_no_stress_so_prosody_is_assigned_over_the_whole_word() {
        let stressed = Morpheme {
            role: MorphemeRole::Stem,
            form: Root {
                syllables: vec![Syllable {
                    pattern: "CV".to_owned(),
                    segments: vec![PhonemeId::new("ph_t"), PhonemeId::new("ph_a")],
                    stress: Some(stem_phonology::prosody::Stress::Primary),
                }],
            },
            ..stem_tira()
        };
        let (form, _) = compose(&stressed, &[&suffix_plural()]);
        assert!(
            form.syllables.iter().all(|s| s.stress.is_none()),
            "a morpheme's citation stress must not pre-empt word-level assignment"
        );
    }

    #[test]
    fn inflect_materialises_one_entry_per_stem_times_cell_stem_major() {
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let cells =
            inflect(&number_paradigm(), &morphemes, &LanguageId::new("proto_x")).expect("inflects");
        assert_eq!(cells.len(), 4, "2 stems × 2 cells");
        // Stem-major / cell-minor: tira-SG, tira-PL, tan-SG, tan-PL.
        assert_eq!(cells[0].glosses, ["star SG"]);
        assert_eq!(cells[1].glosses, ["star PL"]);
        assert_eq!(cells[2].glosses, ["man SG"]);
        assert_eq!(cells[3].glosses, ["man PL"]);
        // The regular forms, before any sound change: -ka everywhere.
        assert_eq!(
            segs(&cells[1].phonemic_form).join(""),
            "ph_tph_iph_rph_aph_kph_a"
        );
        assert_eq!(
            segs(&cells[3].phonemic_form).join(""),
            "ph_tph_aph_nph_kph_a"
        );
    }

    #[test]
    fn inflect_stamps_derived_and_records_the_composition_and_mints_distinct_sets() {
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let cells =
            inflect(&number_paradigm(), &morphemes, &LanguageId::new("proto_x")).expect("inflects");
        for cell in &cells {
            assert_eq!(cell.source, WordSource::Derived);
            assert!(
                !cell.morphemes.is_empty(),
                "§3.3: a composed form must record its composition"
            );
        }
        // A PL cell records both morphemes; an SG cell records the lone stem.
        assert_eq!(cells[0].morphemes.len(), 1, "SG is the bare stem");
        assert_eq!(cells[1].morphemes.len(), 2, "PL is stem + suffix");
        // Every (stem, cell) gets its own cognate set — SG ≠ PL, and each is
        // scoped to the language.
        let sets: std::collections::BTreeSet<&str> =
            cells.iter().map(|c| c.cognate_set.as_str()).collect();
        assert_eq!(sets.len(), 4, "four distinct cognate sets");
        assert!(cells[0].cognate_set.as_str().starts_with("cog_proto_x_"));
    }

    #[test]
    fn inflect_is_deterministic() {
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let language = LanguageId::new("proto_x");
        let a = inflect(&number_paradigm(), &morphemes, &language).expect("inflects");
        let b = inflect(&number_paradigm(), &morphemes, &language).expect("inflects");
        assert_eq!(a, b, "no RNG, so two runs are byte-identical");
    }

    #[test]
    fn inflect_errors_on_a_missing_stem() {
        // The paradigm names m_tan, but only m_tira is supplied.
        let morphemes = vec![stem_tira(), suffix_plural()];
        let err = inflect(&number_paradigm(), &morphemes, &LanguageId::new("x"))
            .expect_err("m_tan is missing");
        assert!(err.to_string().contains("m_tan"), "{err}");
    }

    #[test]
    fn inflect_errors_when_an_affix_slot_points_at_a_stem() {
        let paradigm = Paradigm {
            cells: vec![ParadigmCell {
                label: "PL".to_owned(),
                // m_tira is a Stem, not a legal affix.
                affixes: vec![MorphemeId::new("m_tira")],
            }],
            ..number_paradigm()
        };
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let err = inflect(&paradigm, &morphemes, &LanguageId::new("x"))
            .expect_err("a stem cannot be an affix");
        assert!(err.to_string().contains("affix morpheme"), "{err}");
    }

    #[test]
    fn a_regular_paradigm_reports_one_allomorph_per_affix() {
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let cells =
            inflect(&number_paradigm(), &morphemes, &LanguageId::new("x")).expect("inflects");
        let lexicon = Lexicon::from_entries(cells);
        let measure = morphological_irregularity(&lexicon);
        assert_eq!(measure.len(), 1, "one affix in play: the plural");
        assert_eq!(measure[0].morpheme.as_str(), "m_plural");
        assert_eq!(measure[0].count(), 1, "regular: -ka everywhere");
        assert!(!measure[0].is_irregular());
    }

    /// The M8 target, at the measure level: give the two PL cells traces that voice
    /// the suffix `/k/` in one and not the other, and the plural must report two
    /// allomorphs. (The *engine-verified* version of this — where the trace is
    /// produced by `apply_rules` rather than hand-built — lives in the acceptance
    /// test; here we pin the measure in isolation.)
    #[test]
    fn a_conditioned_sound_change_makes_the_measure_report_two_allomorphs() {
        let morphemes = vec![stem_tira(), stem_tan(), suffix_plural()];
        let mut cells =
            inflect(&number_paradigm(), &morphemes, &LanguageId::new("x")).expect("inflects");

        // tira-PL (index 1): suffix /k/ at flat index 4 voices to /ɡ/.
        let tira_pl = &mut cells[1];
        tira_pl.trace = Some(Derivation {
            input: tira_pl.phonemic_form.clone(),
            steps: vec![RuleApplication {
                rule: RuleId::new("r_ivv"),
                index: 0,
                sites: vec![SiteTrace {
                    at: 4,
                    before: PhonemeId::new("ph_k"),
                    after: Some(PhonemeId::new("ph_g")),
                    resolution: None,
                    left: vec![Some(PhonemeId::new("ph_a"))],
                    right: vec![Some(PhonemeId::new("ph_a"))],
                    emptied_syllable: None,
                }],
                blocked: vec![],
            }],
        });
        tira_pl.phonemic_form = tira_pl.trace.as_ref().unwrap().final_form();

        // tan-PL (index 3): the rule did not apply — an empty-steps trace.
        let tan_pl = &mut cells[3];
        tan_pl.trace = Some(Derivation {
            input: tan_pl.phonemic_form.clone(),
            steps: vec![],
        });

        let lexicon = Lexicon::from_entries(cells);
        let measure = morphological_irregularity(&lexicon);
        assert_eq!(measure.len(), 1);
        assert_eq!(
            measure[0].count(),
            2,
            "the suffix now surfaces as -ɡa and -ka: {:?}",
            measure[0].allomorphs
        );
        assert!(measure[0].is_irregular());
        // Below the "extreme" threshold — a two-way alternation is unremarkable.
        assert!(measure[0].count() < HIGH_ALLOMORPH_COUNT);
    }

    #[test]
    fn a_morpheme_round_trips_through_ron() {
        let morpheme = suffix_plural();
        let text = ron::ser::to_string(&morpheme).expect("serialise");
        let back: Morpheme = ron::from_str(&text).expect("deserialise");
        assert_eq!(back, morpheme);
    }

    #[test]
    fn a_morpheme_defaults_its_part_of_speech_to_noun() {
        // part_of_speech omitted — the serde default fills it.
        let text = r#"(id: "m_x", role: suffix, gloss: "PL", form: (syllables: []))"#;
        let morpheme: Morpheme = ron::from_str(text).expect("deserialise");
        assert_eq!(morpheme.part_of_speech, PartOfSpeech::Noun);
    }

    #[test]
    fn a_misspelled_morpheme_field_fails_to_load_rather_than_defaulting() {
        let text = r#"(id: "m_x", roll: suffix, gloss: "PL", form: (syllables: []))"#;
        assert!(ron::from_str::<Morpheme>(text).is_err());
    }
}
