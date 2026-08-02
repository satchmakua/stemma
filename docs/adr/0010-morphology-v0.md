# 10. Morphology v0 is concatenation in `stem_lexicon`, and irregularity is measured, not stored

- **Status:** Accepted
- **Date:** 2026-08-01

## Context

M8 adds `DESIGN.md` §7.3's morphology. Its acceptance is narrow and load-bearing:
*a regular paradigm becomes irregular purely as a consequence of an ordered sound
change, and the trace explains why.* The worked example is **conditioned
allomorphy** — a plural suffix `-ka` attaches regularly to every stem, then
intervocalic voicing fires after vowel-final stems (`tira-ka` → `tira-ɡa`) and not
after consonant-final ones (`tan-ka`), splitting one regular exponent into two
surface allomorphs.

Four forces shape the design:

1. `DESIGN.md` §20.1 names **scope explosion** as the top risk to the project, and
   morphology is where it is most tempting (a full morphosyntactic feature system,
   non-concatenative exponence, grammaticalization, a typological profile, syntax).
2. The hard constraints hold unchanged: everything traceable (§3.3), everything
   deterministic (§9.4), cognate-set ids survive forking (§8.6), report don't
   police (§17), new fields `#[serde(default)]`.
3. The sound-change **engine is the source of truth** and must not learn about
   morphemes — a morpheme boundary is a phonological non-event.
4. M7 left `NotModelled::MorphologicalIrregularity` as the placeholder M8 fills,
   under the `docs/adr/0009` discipline (a band must agree with a report check via
   a shared constant).

## Decision

### No new crate; morphology lives in `stem_lexicon`

`DESIGN.md` §9.2 draws `morpheme.rs` inside `stem_lexicon`, and that is right: a
morpheme's form is a `Root` (the type `stem_phonology` already provides and a
`WordEntry` already stores), and composition is concatenation of syllable lists.
The 8-crate DAG is untouched. A `stem_morphology` crate would be scaffolding for a
subsystem v0 does not have.

### Composition is concatenation, and an inflected cell is an ordinary `WordEntry`

`compose(stem, affixes)` concatenates morpheme forms into one composition `Root`
(prefix\* · stem · suffix\*), emitting a `MorphemeRef` per morpheme with the flat
span it occupies. `inflect(paradigm, morphemes, language)` materialises one
`WordEntry` per (stem × cell).

The decisive consequence: **a cell is a full `WordEntry`, so it flows through
`apply_rules`, `fork`, `trace`, `cognates` and export exactly like any word.** A
morpheme boundary is a segment adjacency, and the engine's environment scan already
crosses syllable boundaries (`apply.rs`: "the /k/ of `ta.ka.la` is intervocalic
across two boundaries"). So **cross-boundary sound change falls out with zero engine
change** — preserving "`apply_rules` is a pure, RNG-free function", the strongest
determinism claim in the project. The rejected alternative — a bespoke
"inflect-that-evolves" that applies rules across boundaries itself — would duplicate
the engine and could drift from it.

### The composition record is spans, not surface segments

`WordEntry.morphemes: Vec<MorphemeRef>` records *which morpheme, which span* in the
composition form — never the surface segments, which live in `phonemic_form`. A
span `[start, end)` indexes `Derivation.input`, which never changes (a later stratum
extends `steps`, not `input`), so a morpheme's surface allomorph is recoverable at
any lineage depth via `Derivation::surface_of_input_span`. Storing surface segments
on the ref would be the second-source-of-truth desync `docs/adr/0007` forbids — the
same reason a rendered `form` string is kept off `WordEntry`. This discharges §3.3
for composed forms: a composed form with empty `morphemes` is a form with no
recorded composition, the same bug class as a transformed form with no trace.

### Each (stem, cell) mints its own cognate set

`inflect` mints a distinct `CognateSetId` per (stem, cell) via
`scoped_cognate_set` — the sole sanctioned mint site, and `morpheme.rs` joins the
per-crate source-scan that enforces it. The invariant (`docs/adr/0007`) is that
`a.cognate_set == b.cognate_set` iff `a` and `b` descend from one and the same
proto entry; `tira-SG` and `tira-PL` are *different* entries (different forms,
different meanings), so they get different sets. `fork` copies each verbatim, so
daughter A's `tira-PL` and daughter B's `tira-PL` are cognate while SG and PL are
not. The rejected alternative — clone the stem's set into every cell — would collapse
SG and PL into one set and break the invariant.

### Irregularity is measured, and joins the profile the M7 way

`morphological_irregularity(lexicon)` reports, per affix, its distinct surface
allomorphs (a `Vec`, first-seen order, never a map or float reaching output). This
fills M7's `NotModelled::MorphologicalIrregularity`, which **leaves** the deferred
list and becomes a scored `PlausibilityProfile` band. Honoring `docs/adr/0009`: the
`HighlyAllomorphic` band and the paired `high_morphological_irregularity` validation
**Note** both read the one shared `HIGH_ALLOMORPH_COUNT`, so band and Note cannot
disagree (a projection test pins it). The Note is never an Error — extreme
allomorphy is unusual, not broken (§17). The M8 demo's two-way split scores
`Allomorphic` and correctly does *not* trip the Note. `high_change_density` stays as
a distinct, coarser lineage signal (rules per century); its claim to *stand in* for
§17's irregularity is retired, but the signal is kept.

### v0 is fenced

**Out of v0, and named so scope cannot creep:** non-concatenative morphology
(infix, reduplication, templatic/introflexive, circumfix); fusional/polysynthetic/
portmanteau exponence; grammaticalization & erosion (a morpheme changing *role*
over time — later-milestone diachronic morphology; v0 morphemes are static and
sound change acts on their *forms*); a typed morphosyntactic feature system (v0
labels are free strings); the §7.3 typological profile
(isolating/agglutinative/fusional as a scored dimension); agreement and syntax (no
syntax engine — §20.1); a resyllabifier (unchanged from M3 — patterns go stale,
nothing reads them); morpheme semantics (M9); Markdown/CLDF paradigm export and UI
(M6/M11 — `stem_export` is untouched); appending inflected cells onto a concept
lexicon (v0 replaces, per the `new-lexicon` rule); any authoritative rejection of a
"weird" morphology (every new signal is a Note or a band, never an Error).

## Consequences

- **The engine, `stem_export`, and `stem_io` are untouched.** A source-scan guard
  test (`stem_soundchange`'s `the_engine_never_references_morphology`) asserts the
  engine names no morphology *type or operation* (`compose`, `inflect`, `Morpheme…`,
  `Morphology`, `Paradigm`) — it clones the `WordEntry.morphemes` field verbatim but
  never reasons about it. (A crate-dependency guard could not express this:
  `stem_soundchange` already depends on `stem_lexicon`, where the morphology types
  live; the scan is the honest enforcement.) Persistence stays generic over serde;
  rendering stays a pure projection.
- **Every pre-M8 file keeps loading and round-tripping byte-identically.**
  `WordEntry.morphemes`, `LanguageGenome.morphology`, and the morphology types are
  all additive `#[serde(default, skip_serializing_if …)]`; a test asserts the
  reference proto serialises with no `morphology` bytes. `reference_phonology.rs`'s
  `["lexicon.empty"]` pin is undisturbed — the new checks stay quiet on a language
  with no morphology.
- **One deviation from `M8-SPEC` §3:** `compose` takes `&[&Morpheme]` and reads each
  affix's own `role`, rather than the spec's redundant `&[(&Morpheme, MorphemeRole)]`
  tuple. The morpheme's stored role is authoritative; the tuple could contradict it.
  Recorded in `PROGRESS.md`.
- **The demo is independently runnable:** `stemma inflect … --paradigm NUMBER` →
  `stemma apply-rules … --rules rules_intervocalic_voicing.ron` → `stemma paradigm`
  shows a regular `-ka` plural become the irregular `-ɡa`/`-ka`, with each cell's
  conditioning rule named; `stemma trace` shows any cell's full derivation.
- **The `WordSource::Derived` variant** that `word.rs` reserved for morphology now
  has its producer: `inflect` stamps it on every cell.
