# PROGRESS — Stemma

A build log of what shipped and the notable decisions behind it. **Keep it honest**
— this is the working memory between build sessions. The forward-looking plan and
acceptance tests live in [ROADMAP.md](ROADMAP.md); this is the backward-looking
"what got done and why" companion.

**Current phase:** Phase 2 — depth. M7 and M8 shipped; next milestone: **M9 —
semantics v0**.

## State of the tree

| Crate | Holds | Status |
|---|---|---|
| `stem_core` | typed IDs (`MorphemeId` since M8), `StemmaError`, `Validate` / `ValidationReport`, `rng`, `suggest` | working |
| `stem_phonology` | features, `Phoneme`, inventory, phonotactics, root generation | working |
| `stem_lexicon` | `WordEntry`, `Lexicon`, the 103-concept list, cognate-set minting, **morphemes / `compose` / `inflect` / allomorph measure** | working |
| `stem_soundchange` | rules, matching, ordered application, resolution, traces | working — **untouched by M8** |
| `stem_genome` | `LanguageGenome`, `fork`, `LineageGraph`, family validation, `render_family`, **`render_paradigm`**, plausibility profile | working |
| `stem_io` | RON/JSON load & save | working — **untouched since M0** |
| `stem_export` | Markdown dictionaries, CLDF CSV, cognate table, family demo | working — **untouched by M8** |
| `stem_cli` | the `stemma` binary | …plus `profile`, `inflect`, `paradigm` |

---

## M8 — Morphology v0 · built 2026-08-01 · ✓ verified

Stemma grows morphemes. A `morphology` block (stems, affixes, paradigms) attaches to
a genome; `stemma inflect --paradigm NUMBER` materialises the paradigm's cells as
ordinary `WordEntry`s (the regular, pre-sound-change forms); `apply-rules` evolves
them; `stemma paradigm` renders the result. **457 tests pass**; clippy clean under
`-D warnings`; fmt applied.

**The acceptance, run end-to-end:** `inflect fixtures/morphology_asterian.ron
--paradigm NUMBER` gives a regular `-ka` plural (`tiraka, menaka, tanka, sulka`).
After `apply-rules … rules_intervocalic_voicing.ron`, `stemma paradigm` shows the
suffix split into **two allomorphs** — `-ɡa` after the vowel-final stems (`tiraɡa`,
`menaɡa`), `-ka` after the consonant-final ones (`tanka`, `sulka`) — with each cell
naming the rule that fired or "did not apply". `stemma trace w_0002` shows the
`r_ivv` step voicing `k > ɡ` at `[4,5)` in `a _ a`; `stemma trace w_0006` shows
`r_ivv … — did not apply`. `stemma profile` scores `Morphological irregularity
allomorphic (PL: 2)`. That is a regular paradigm made irregular *purely* by an
ordered sound change, with the trace explaining why.

### Decisions worth knowing

**No new crate; the engine does not change (`docs/adr/0010`).** A morpheme's `form`
is a `Root`; `compose` concatenates syllable lists, so a morpheme boundary is a
segment adjacency and the engine's cross-boundary environment scan gives conditioned
allomorphy for free. An inflected cell is an ordinary `WordEntry`, so `apply-rules`,
`fork`, `trace`, `cognates`, and export all work on it unchanged — `apply.rs` stays
pure and RNG-free. A source-scan guard test (`the_engine_never_references_morphology`)
catches any morphology type or operation creeping into `stem_soundchange`.

**The composition record is spans, not surface segments.** `WordEntry.morphemes`
stores a `MorphemeRef { morpheme, role, gloss, start, end }` per morpheme — the flat
span in the composition form (= `Derivation.input`, which never changes). The surface
allomorph is recovered by `Derivation::surface_of_input_span`, which replays the
trace carrying each segment's origin index. Storing surface segments would be the
`docs/adr/0007` desync. This discharges §3.3 for composed forms: a composed form with
empty `morphemes` is a form with no recorded composition.

**Each (stem, cell) mints its own cognate set.** `inflect` calls `scoped_cognate_set`
(the sole mint site; `morpheme.rs` joined the source-scan). `tira-SG` and `tira-PL`
are different entries, so different sets — and `fork` copies each verbatim, so two
daughters' `tira-PL` stay cognate while SG ≠ PL (`docs/adr/0007`).

**Irregularity is measured and joins the profile the M7 way (`docs/adr/0009`).**
`morphological_irregularity` counts each affix's distinct surface allomorphs; it
fills M7's `NotModelled::MorphologicalIrregularity`, which leaves the deferred list
and becomes a scored `MorphologicalIrregularity` band. The `HighlyAllomorphic` band
and the `high_morphological_irregularity` validation **Note** read one shared
`HIGH_ALLOMORPH_COUNT` (= 3), so they agree by construction — a projection test pins
it. The demo's two-way split is `Allomorphic`, below the Note. Never an Error (§17).
`high_change_density` stays as a distinct coarse signal; only its *claim* to stand in
for morphological irregularity is retired.

**One deviation from `M8-SPEC` §3:** `compose` takes `&[&Morpheme]` and reads each
affix's own `role`, not the spec's redundant `&[(&Morpheme, MorphemeRole)]` tuple —
the stored role is authoritative and the tuple could contradict it.

**Scope fenced hard (§20.1).** v0 is concatenative (prefix\* · stem · suffix\*) and
nothing else: no non-concatenative or fusional exponence, no grammaticalization, no
typed feature system, no typological profile, no syntax, no resyllabifier, no
semantics, no paradigm export/UI, no append-inflection. All named in `docs/adr/0010`.

---

## M7 — Plausibility profile · built 2026-07-23 · ✓ verified · **Phase 2 begins**

The validator learns typology. `stemma profile` prints DESIGN §17's scored
dimensions — typological rarity, phonotactic complexity, historical depth — as
qualitative *bands*, and the graded report now carries specific, non-authoritarian
plausibility warnings (an 80-consonant inventory, a three-consonant cluster, a
rapid change history). **427 tests pass**; clippy clean under `-D warnings`. This
opens Phase 2 (depth) — and it is the *same* `ValidationReport` with more checks,
not a new subsystem.

Verified by running it: `stemma profile fixtures/proto_asterian.ron` reports
`typical / simple / none`, names the four unbuilt dimensions as "not yet modelled",
and stays quiet (0 warnings); `stemma profile fixtures/implausible_clusters.ron`
reports `complex` and fires `large_consonant_cluster` — while still validating.

### Decisions worth knowing

**The scored block is a derived read-model, not a second validator
(`docs/adr/0009`).** The plausibility *warnings* are ordinary `report.warn`/`note`
calls in the existing `Validate` impls — they reach `stemma validate` with zero new
plumbing, which is the whole proof this is "more checks on the same report". The
*bands* are a pure `fn(&genome) -> PlausibilityProfile` presented alongside the
report the way `summary()` already is: zero `Issue`s, no `Severity`, no serde,
never stored.

**The band and the warning read one shared set of constants, so they cannot
disagree.** `LARGE_CONSONANT_COUNT`/`LARGE_VOWEL_COUNT`/`LOPSIDED_RATIO`/
`VERY_SMALL_TOTAL` live once in `stem_phonology`; both the size warnings and
`PhonemeInventory::rarity()` read them, so the `Rare` band holds *exactly when* a
size warning fires — a test pins the projection. That is the line that keeps the
descriptive view from drifting into a parallel check registry.

**Honesty over completeness.** §17 lists seven dimensions; M7 has real data for
phonology and coarse lineage only. The five that need unbuilt milestones render as
explicit "not yet modelled → Mn" lines, never a number; §17's composite "82%" is
dropped (any percentage would overclaim or average over dimensions that do not
exist); syntax/word-order is dropped entirely, with no "not modelled" line, so it
does not invite a syntax engine (§20.1). The lineage signals say out loud that they
count *authored rules* (an editorial granularity) and read only the sound-change
log — not the morphological irregularity §17's third example is really about, which
waits for M8.

**Report, do not police (§17).** Every new check is a Warning or Note — an
80-consonant monster earns warnings and still `validate()`s. No new Error; the
acceptance suite pins `is_ok()` on the weird languages. The deliberate *absence* of
a small-vowel warning (a two-vowel system is attested) is the same call.

**One float left the codebase.** The legacy `lopsided_inventory` used an `f32`
ratio in its message; it is now integer cross-multiplication (`c > RATIO * v`), so
the last float leaves the validation control path and the constant is shared with
the band.

### Adversarial review — 6 findings, all fixed

A five-dimension panel (17 agents) reviewed the implementation, each finding
independently reproduced. Six distinct defects, all minor, all fixed — and the two
that mattered were both about the projection invariant the design leans on:

1. **The empty inventory broke `Rare ⟺ a size warning fired`.** `rarity()` scored a
   zero-phoneme inventory `Rare` (via `c == 0` / `total < 5`), but `validate`
   early-returns on an empty inventory with only the `empty` Error — so *no* size
   warning fires, and the band disagreed with the checks it is documented to
   summarise. Fixed with an explicit empty guard (empty → `Typical`, mirroring
   `validate`'s early return), and the projection test now sweeps the boundaries
   (45/46 C, empty, vowelless) in both directions.
2. **The `rarity()` doc claimed a vowelless inventory "counts as Rare"** — but the
   code (correctly) scores it `Typical` (a vowelless inventory trips only the
   `no_nucleus` Error, not a size code). The doc was the wrong half and pointed
   *opposite* the invariant; corrected.
3. A doc said "five" not-modelled dimensions; `NOT_MODELLED` holds **four**. Fixed.
4. **A botched doc merge** — the M7 insertion had split the `Phonotactics` struct
   doc mid-sentence, orphaning "The genome field stays". Repaired; the const/enum
   now sit above the struct with intact docs.
5. The depth line rendered "over 1 years" — `years` is now singularized.
6. The `implausible_clusters.ron` header still carried the whole copy-pasted
   Proto-Asterian comment block; removed.

One finding was refuted (the closing "bands sit against attested ranges" line vs
the coarse historical-depth band — a wording nuance the skeptic did not consider a
real defect). Net at **427 tests**.

### Gotchas for the next session (M8 — morphology)

- **M8 fills the first "not yet modelled" row.** `NotModelled::MorphologicalIrregularity`
  (M8) is the honest placeholder; when morphemes exist, M8 measures real
  irregularity and the profile gains a dimension. `high_change_density` is the
  *coarse* stopgap that reads only the sound-change log — M8's measure supersedes
  it, not replaces the report check.
- **Add profile bands the way M7 did: shared constants, a projection test, no
  float, no fabricated score.** The band must agree with a report check or it is a
  second opinion (`docs/adr/0009`).
- **`ipa_not_nfc` is deferred, not dropped** — the `by_ipa` comment now points at a
  later data-hygiene pass (it is interchange hygiene, not a §17 typological
  dimension, and would pull in a Unicode-normalization dependency).
- **`reference_phonology.rs` pins the proto's codes to `["lexicon.empty"]`** — a
  new check that fires on Proto-Asterian breaks it. That guardrail is load-bearing;
  keep new plausibility checks quiet on the reference family.

---

## M6 — The portfolio demo · built 2026-07-22 · ✓ verified · **Phase 1 complete**

`stemma demo` tells the whole story in one command: it grows the Asterian family
from the committed fixtures, builds the comparative table, traces five words in
full, and writes it all as a self-contained Markdown document — "Growing a
Language Family in 90 Seconds." **404 tests pass**; clippy clean under
`-D warnings`. This is the last milestone of Phase 1: the diachronic kernel
(inventory → generation → sound change → forking → the comparative views) now
runs end to end from the command line.

Verified by running the ROADMAP acceptance: `stemma demo --out output/demo.md`
writes a 199-line document (proto glossary, three daughters with their rule
histories, the cognate table with the `star` row `*takala | taal | tagal | tala`,
and five fenced etymologies from `*takala → taal` to `*mikala → miala`); two runs
are byte-identical; and a `stem_export` golden pins the exact bytes.

### Decisions worth knowing

**M6 was composition, and stayed composition.** No engine work: every genome comes
from `evolve`, every form from `written`, every derivation from
`render_derivation`, every table cell from `cognate_table`. The only new *logic*
is a `stem_genome::grow_family` helper (pure `evolve` + `assemble`) and one new
renderer.

**Build was split from render — the panel's improvement over all three proposals.**
Each proposal put the whole demo in one `stem_export` function that called
`evolve` internally. That would have put engine-*build* code in the render crate
(an ADR-0006 stretch) **and** made the renderer's canary engine-dependent — any
`apply.rs` change would move it, defeating the M1 canary-vs-golden isolation.
Instead `grow_family` (build) lives in `stem_genome` and `write_family_demo`
(render) is a *pure* projection over an already-built graph, so its canary is a
true renderer-only tripwire: it hand-builds a graph with a hand-authored
`Derivation`, runs no engine, and no fixture or `apply.rs` change can move its
bytes.

**`stem_export` gained a direct `stem_soundchange` dependency** — a legal downward
edge (it was already transitive via `stem_genome`), needed because the trace
blocks call `render_derivation`. Rendering a derivation is rendering, so ADR-0006
holds; no new ADR.

**The demo is honest.** DESIGN §21's flashiest steps need unbuilt milestones, so
the demo does not fake them: it shows Highland `tagal` (the real form; §21's
`tazal` is an unreachable `g→z` place shift), and the closer *names* meaning drift,
morphology, and a visual explorer as forthcoming without printing a drifted gloss
or an "omen" token. Two tests (a `stem_export` golden assertion and a CLI
acceptance) fail if `tazal`/`omen`/`royal sign`/`night-signal` ever appear.

**Determinism is by construction.** No RNG (the proto lexicon is authored, not
generated), no clock (the colophon is dateless; the source-scan now bans
`SystemTime`/`Instant`/`chrono::`), no map, no sort, `include_str!` inputs so the
binary runs identically from any directory. Two runs are byte-identical, pinned at
both the library and the binary.

**The proto roster is bespoke, not `write_lexicon_markdown`.** That renderer emits
its own H1 and prose that is *false* for the authored fixture ("coined … on the
`lexicon` RNG stream at seed N" — the nine words were authored, not coined). The
demo's compact `Gloss | Form | IPA` roster keeps one H1 and one honest provenance
line. Recorded here so the second rendering surface is tracked, not smuggled.

### Adversarial review — 3 findings, all fixed

A five-dimension adversarial panel (scope/anti-fabrication, determinism,
renderers, architecture, canary-vs-golden) reviewed the implementation, each
finding independently reproduced. Three distinct defects, all fixed:

1. **The family-demo canary pinned no bytes (major).**
   `the_family_demo_canary_matches_its_frozen_bytes` only did `contains()`
   landmark checks despite its name, so a genuine `write_family_demo` regression
   (a changed rule-bullet format, a dropped blank line) would slip past it and be
   caught only by the *re-baselineable* golden — exactly the "a renderer
   regression must not hide" failure the discipline exists to prevent. Now a real
   `assert_eq!` against ~1.6KB of inline expected bytes, engine-independent (the
   canary hand-authors a `Derivation`, so no `apply.rs` change can move it) — a
   true renderer-only tripwire, like the cognate-table canary.
2. **The demo printed a non-runnable command (minor).** The proto section said
   "one command away: `stemma export-md`" — but `export-md` requires a `<PATH>`,
   so a reader copying it hit exit 2. Now `stemma export-md <file>`.
3. **The cognate-table notes branch had no byte coverage (minor).** The canary
   had empty notes; added a test rendering notes as escaped italic bullets.

Also fixed a *refuted-but-real* latent issue: `write_family_demo`'s daughter
heading printed the daughter's absolute depth as "+Ny from proto" — correct only
because the demo's proto is at depth 0. Now the edge delta
(`daughter.depth − proto.depth`), correct for any proto; the demo bytes are
unchanged.

### Gotchas for the next session (Phase 2 begins)

- **Phase 1 is the MVP and it is done.** M7 opens Phase 2 (depth): grow the M0
  validation report into §17's typological *plausibility profile* — scored
  dimensions and specific, non-authoritarian warnings. It is the same
  `ValidationReport` with more checks behind it, **not a separate subsystem**, and
  §17's rule stands: report, do not police. Read `DESIGN.md` §17.
- **The demo's story is single-sourced in `write_asterian_demo`.** The CLI, the
  golden test, and (eventually) the M11 UI all call it, so the document cannot
  drift between front ends. Change the story there, re-baseline
  `golden/family_demo.md`, and only after the inline canaries stay green.
- **`grow_family` is the sanctioned way to build a family in code.** M11 will use
  it too. It returns a report per daughter, parallel to the specs; the demo merges
  them to stderr.
- **`escape`/`fmt_err` are now `pub(crate)` in `markdown.rs`.** The cognate-table
  and demo renderers share them; the dictionary golden is unchanged.

---

## M5 — Cognate tables & word traces · built 2026-07-21 · ✓ verified

The family becomes *legible*. `stemma cognates` prints §10.3's comparative table —
reflexes of each meaning across every daughter, side by side — and `stemma
trace-word` prints §10.2's full derivation addressed by meaning instead of by word
id. **385 tests pass**; clippy clean under `-D warnings`.

Verified by running the ROADMAP acceptance: `stemma cognates … --meanings water
sun star king mother` prints the table with `star → *takala / taal / tagal /
tala`, `king → *rekan / rean / regan / rean`, and `mother → *mikala / mial / migal
/ miala`; `stemma trace-word out/coastal.ron star` is byte-identical to `stemma
trace out/coastal.ron w_0001`.

### Decisions worth knowing

**The whole milestone was one lexicon query, one graph view, two thin verbs, and
one fixture word.** Everything else was already in the tree: the cognate set is
the cross-language join (`docs/adr/0007`), and `render_derivation` already emits
the entire §10.2 ledger. M5 changed no engine code and no file format.

**Meaning resolves by *displayed gloss*, never by concept.** `Lexicon::by_meaning`
matches `display_gloss()` case-insensitively, so `king` finds `*rekan` — concept
MAN, gloss override "king" — which a concept-key match (`king → KING concept →`
no fixture word) would render as an empty row. The fixture is built to expose
exactly that bug: `w_0005` is deliberately concept MAN with a "king" gloss (the
etymology-vs-surface seed M9 needs). One shared resolver serves both `cognates`
and `trace-word`, so the two can never disagree about what a meaning names.

**The table joins by cognate set, resolved *once* against the reference.** Not
re-resolved by meaning per column — that would be a concept join, and under M9's
meaning drift a daughter whose reflex shifted sense would silently drop out of its
own row. Ancestry gets a language into the row; the displayed meaning is only the
row label. A daughter lacking the set is a gap (`—`), not an error (plausibility
reported, not enforced). Each cell renders against its *own* column's inventory,
because Highland's `tagal` carries `ph_g`, absent from the proto inventory —
rendering with the reference's would abort the whole table.

**`MOTHER *mikala` earned its place with a real contrast.** The velar chain again,
but `/i/` is `[-low]`, so Riverine's low-vowel coalescence cannot fire and it
alone keeps all three vowels (`miala`), where STAR's `*takala` coalesced to
`tala` — a lesson (coalescence is conditioned) no earlier word teaches. Chosen
over a zero-risk `*maka` (which would have been isomorphic to `*taka`). The
reflexes were **verified against the engine**, not hand-computed into the goldens.

**`stemma trace-word coastal star` is file-native.** ROADMAP writes a language id;
the CLI is file-native (there is no language registry — the lineage is derived
from files), so "coastal" is `out/coastal.ron`, the same deviation `trace` and
`family` already made. `trace-word` is `trace` with meaning-addressing swapped for
id-addressing: same renderer, zero new rendering.

### Adversarial review — clean

A five-dimension adversarial panel (resolver correctness, the cognate-set join,
determinism/rendering, CLI wiring & fixture correctness, placement & invariants),
each dimension charged to reproduce any defect against the code, returned **zero
findings**. Credible for the smallest milestone of the phase — one lexicon query
and one graph view over established patterns — and backed by front-loaded
verification: the `*mikala` reflexes were confirmed against the engine before the
goldens were written, the acceptance table's exact bytes were baselined from the
real renderer, and `trace-word star == trace w_0001` was checked byte-for-byte.

### Gotchas for the next session

- **The reference fixture is now nine words.** Adding `w_0009` MOTHER moved five
  M4 test goldens (`forms()` `1..=9`, `len()==9`, the reflex table's ninth row,
  coverage `9/9`, the family snapshot's `9 words`) and three doc pins — all
  updated. The M4 PROGRESS entry above now reads `9/9` and `27-cell` for accuracy
  (a reader running the M4 tests today sees those numbers). `proto_asterian.ron`
  is untouched (its own `w_0009` is BLOOD); M3 acceptance stays green because
  `*mikala` mints nothing new.
- **`render_cognate_table` pads by char count, not byte length**, so `ŋ`/`ɣ`
  cells align. Its output is the M6 demo's raw material and will want a
  `stem_export` Markdown/CLDF projection over the same `CognateTable` struct —
  the struct carries the ids and names a projection or a clickable cell needs.
- **`cognates` writes only the table to stdout;** the reference banner and any
  notes go to stderr, so the table stays diffable (the `generate-roots` split).
- **The M5 continuity debt is settled, not carried forward:** MOTHER and KING were
  on the concept list since M2 (a prior session's hook for exactly this); MOTHER
  is now a real word, and `king` resolves by gloss.

---

## M4 — Forking & lineage · built 2026-07-21 · ✓ verified

Languages now *branch*. `stemma fork` splits a parent into a daughter — a
verbatim copy under a new identity, or (with `--rules`) a daughter that has
already undergone its own sound changes — and `stemma family` assembles several
language files into a lineage, printing the family tree, cognate coverage, and a
graded report. **360 tests pass**; clippy clean under `-D warnings`.

Verified by running the ROADMAP acceptance end to end: forking
`asterian_attested` three ways under three hand-written rule histories yields
Coastal, Highland, and Riverine, whose lexicons differ pairwise — `*takala`
reflects as **taal** / **tagal** / **tala**, and the whole 27-cell golden table
matches the engine cell for cell. `stemma family` reports **9/9 cognate sets
present in all three** daughters, every daughter's trace replays to its stored
form, `stemma trace out/coastal.ron w_0001` walks unbroken from *takala to taal,
and two fork runs write byte-identical files.

### Decisions worth knowing

**The lineage graph is derived, never stored.** `DESIGN.md` §8.6 sketches a
`HashMap` of nodes plus a stored `Vec<LineageEdge>`; both were rejected.
`LineageGraph` holds a `Vec<LanguageGenome>` in argv order and derives every edge
from the `parent` field on demand — a stored edge beside `parent` is a second
copy of one fact that nothing keeps synchronised, the exact desync class this
project has refused three times (`form`, stored intermediate forms,
`Syllable::pattern`-as-semantics). And there is **no map anywhere**: a `HashMap`
leaks iteration order toward output (§9.4) and swallows the duplicate ids the
validator must see. Tens of nodes, linear scans, fully deterministic.
(`docs/adr/0008`.)

**`fork` is identity-plus-split; `evolve` still runs the rules.**
`LanguageGenome::fork` clones the genome verbatim and relabels it — no rules, no
RNG, no form change. The CLI's `fork --rules` calls `evolve` through the *same*
load→gate→write helper `apply-rules` uses, so the write gate (refuse on
validation Errors) cannot drift between the two verbs. Rejected outright: an
in-place `advance` operation, because `apply-rules x --out x` is not idempotent —
re-running one line applies a stratum twice, the double-application hazard
`applied_rules`' past-tense contract exists to forbid.

**The cognate obligation was one line, and it is discharged by construction.**
`fork` clones the lexicon whole, so every `cognate_set` is byte-identical and no
code path can mint. A new source scan (`stem_genome_never_mints_a_cognate_set`,
walking `src/*.rs` at runtime) proves it, and it closed a real gap: the old
`stem_lexicon` scan used `include_str!`, which cannot cross a crate boundary, so
it never saw `stem_genome` and already missed `stem_lexicon`'s own `trace.rs`
(now added). The rule is unchanged; its enforcement is honestly per-crate.

**No `LineageEdgeKind`, not even `Descent`.** With edges derived there is no file
format to stabilise, so a one-variant enum is scaffolding by the `HistoricalEvent`
precedent. A dialect split *is* descent; "split" is out-degree, derivable. The
four contact-like kinds are not derivable from `parent` (a contact edge is a
second parent) and arrive with their producers in M7+.

**`WordEntry.ancestor` stays unshipped.** ADR-0007 deferred it "to M4"; M4
discharges it instead. Both fork and evolve copy word ids verbatim, so a
daughter word's ancestor is *always* the same-id parent entry — a stored field
would hold a tautology. It ships when a producer writes a non-identity value (M7
borrowing, M8 derivation); until then a library test plus the cross-file
`family.word_id_orphan` Warning defend the derivability.

**`chronology_years` is absolute from the lineage root.** Already the codebase's
commitment (M3 dated its last rule 480 and passed `--years 480`; the monotonicity
check runs over the whole concatenated log). So the three daughters' rule dates
are absolute, and `--years` conventionally equals the last rule's date. Family
edges report the depth *delta* (`+470y`), never the total; a negative delta is
`family.depth_regression`, an Error.

### The fixtures — three crossing isoglosses

Chosen so the acceptance contrasts are real, not decorative. Coastal innovates a
feeding chain (voicing → lenition → γ-loss) plus apocope; Highland shares voicing
and apocope but assimilates nasals instead of leniting; Riverine shares only the
nasal isogloss and keeps its final vowels and voiceless stops. Result: voicing
{Coastal, Highland}, apocope {Coastal, Highland}, nasal assimilation {Highland,
Riverine} — three isoglosses, each shared by a different pair, wave-model in
miniature. Two convergences fall out (`rean` and `ta` are identical in Coastal
and Riverine through different histories, distinguishable only by trace), and the
cross-file rule-id convention is honest: `r_ivv` is byte-identical in Coastal and
Highland, only its per-branch date differs.

### Adversarial review — 3 findings, all fixed

A five-dimension adversarial panel reviewed the implementation (each finding
independently reproduced by a skeptic before it counted). It surfaced three
distinct defects, all minor, all in the family-coverage rendering, now fixed at
**361 tests**:

1. **Gap language lists came out in DFS order, not node order.**
   `descendant_indices` walked the closure with a stack (`frontier.pop()`, so
   depth-first despite a comment claiming breadth-first), and that order *did*
   reach output — the `gap: X absent from …` line `stemma family` prints. A
   comment even asserted it "does not reach output", which was false. The closure
   is now sorted into stored order before it returns, honouring the documented
   "languages in node order" contract for any tree shape. Caught only because the
   acceptance family is flat (one root, three direct children), where DFS order
   coincides with node order — a depth-≥2 family exposed it.
2. **`apply-rules --years` lacked the `range(0..)` guard `fork` has.** The M4
   refactor routes both verbs through one write gate, but only `fork` rejected
   negatives; `apply-rules --years=-100` on a deep parent would persist a
   descendant *earlier* than its parent (total depth still positive, so the
   genome's `negative_lineage_depth` Error never fires). Both verbs now carry the
   same guard, with a test asserting it.
3. **The coverage line printed "1 sets".** The set count was the one count in
   `render_family` never singularised. Fixed.

One finding was **refuted**: the mint-scan's test-region cutoff (it stops at the
first `#[cfg(test)]`) and its non-recursive `read_dir` are real limitations, but
the skeptic confirmed the current flat `stem_genome/src` is clean, so it is a
latent-robustness note rather than an M4 defect — left as-is.

### Gotchas for the next session

- **`stemma family` output is two parts.** `render_family` (tree + coverage) is
  library-owned and snapshot-pinned for M6; the validation report prints
  *separately* via `print_report`. The snapshot covers only the first part.
- **The acceptance family is valid but not silent.** Each daughter carries
  `lexicon.syllable_shape_mismatch` **Notes** (stale `Syllable::pattern` after
  deletions — M3 established this is Note-severity on a word with a derivation),
  so the family report honestly ends `✓ valid — 0 warning(s), nothing blocking`,
  never `✓ no issues`.
- **Deviation from ROADMAP's literal "Proto-Asterian":** the fork parent is
  `asterian_attested.ron`, the proto stage that *has* words (`proto_asterian.ron`
  has none, and M3's own acceptance forked it). Recorded in the fixture headers.
- **Deviation from DESIGN §21's sketch:** Highland gives `tagal`, not §21's
  `tazal` — ɡ → z is a place shift `set` cannot express (`overlay` never removes
  a cell, apply.rs). Revisit at M10 if node-level writes arrive.
- **M5 continuity debt (recorded, not solved):** ROADMAP M5 names meaning MOTHER
  and gloss "king", but the fixture has no MOTHER concept and "king" is a gloss
  override on MAN. M5 appends a MOTHER word (checking no golden pins the word
  count) or adjusts its meanings.

---

## M3 — Sound-change engine · built 2026-07-20 · ✓ verified

The heart of the program. Languages now *change*: `stemma apply-rules` runs an
ordered rule sequence over a lexicon, every application writes a per-word
`Derivation`, and `stemma trace` prints §10.2's killer feature — the full causal
history of a word, rule by rule, back to the proto-form. **314 tests pass**;
clippy clean under `-D warnings`.

Verified by running it: applying `fixtures/rules_asterian.ron` to
`fixtures/asterian_attested.ron` reproduces the design doc's own worked example
exactly — `takala → tagala → tagal → taɣal` — and the other seven fixture words
land on their hand-computed forms (`tag`, `sawel` unchanged, `akw`, `reɣan`,
`saŋk`, `amp`, `ant`). Three phonemes were minted along the way (/ɡ/ /ɣ/ /ŋ/),
two runs write byte-identical files, and the M1/M2 corpus digests are untouched.

### Decisions worth knowing

**The engine uses no RNG at all.** `apply_rules` is a pure function of its five
arguments. `RngDomain` gained no variant — the strongest determinism claim the
project can make, and it was free.

**A rule can create a phoneme the language does not have.** Voicing /k/ yields a
bundle no inventory phoneme carries; a compiled-in 20-row reference table
(`stem_phonology::reference`) names it. The minted /ɡ/ is **U+0261 SCRIPT G**,
not ASCII `g` — the ASCII letter is its romanisation, which is why `written()`
prints ROADMAP's literal `tagala` while `ipa()` prints `taɡala`. Ids come from
the table row (`ph_g`), so two sister languages independently innovating /ɡ/ get
the *same* phoneme — a correct merger, not a collision. Exact match at every
tier: on this fixture `/k/[+voice]` is Hamming-1 from /k/ itself, so any fuzzy
resolver would silently undo the rule and write a trace that lies.

**Simultaneous application over a frozen snapshot.** Within one rule, every
match is found against the word as it stood before the rule; a rule's output is
never visible to its own matching, and a `Copy`'s donor reads from the snapshot
too. With single-segment targets, application is provably commutative over the
site set — which is why there is no `ltr`/`rtl` flag and no overlap resolver:
they would be unobservable settings, i.e. lies about what the file format means.
The multi-segment tie-breakers are pre-committed in `apply.rs`'s module doc.

**The design doc's own example cannot demonstrate rule ordering.** Verified
arithmetically: `takala` yields `tagal` under either order of voicing and
apocope, because its final vowel never conditioned the /k/. The fixture ships
`*taka`, which genuinely bleeds (`tak`) and counterbleeds (`tag`) — and a test
pins the *reason* it exists so a future session cannot delete it as redundant.

**Assimilation is a feature copy that can transfer absence.** One rule (`copy
place from after(0)`) resolves three ways on the fixture: /n/+/p/ → declared
/m/; /m/+/t/ → declared /n/ — which requires the copy to *remove* the rounding
cell /t/ leaves absent, the reason `FeatureBundle::unset` exists; /n/+/k/ →
minted /ŋ/. The node is the unit of copy (`FeatureNode::Place` carries the
articulators *and* their dependents), making the ill-formed partial copy
unrepresentable.

**Stress landed as a syllable-scoped store, not a feature.** `Prosody::assign`
marks a word once in its life (all-or-nothing, so splitting a rule sequence
across two runs gives the same language), and `Some(Unstressed)` never matches
an unmarked syllable — a language with no declared prosody cannot silently get
"delete the last vowel" while claiming the stronger rule.
`rules.stress_without_prosody` says so out loud.

**Traces are deltas, not snapshots.** `Derivation { input, steps }` stores the
proto-form plus per-site edits; `replay()` reconstructs every intermediate, so
§16.3's property — the trace replays to the stored form — is a statement about
the *file*. A second `apply-rules` run extends the derivation rather than
replacing it, so a derivation always begins at the proto-form.

**The promised escalation landed:** `phonology.features_unspecified` is now an
**Error** — the M1 commitment ("this becomes an Error in M3, when a rule engine
exists") had its trigger fire. Two-sided: `generation_blocking` still filters it
so pre-M1 files keep generating, and `apply_rules` gates on the *unfiltered*
report, so the validator and the engine agree in both directions.

### Adversarial review — 6 findings, 5 fixed, 1 deferred with rationale

An adversarial panel reviewed the implementation (resolution and severity
dimensions completed; the rest were cut short by a session limit and their
findings verified by hand instead). Confirmed and fixed, now at 317 tests:

1. **`ambiguous_target_symbol` was promised and never emitted** (spec §9.5, and
   `inventory.rs` documented it as existing). When two phonemes share a bundle
   (legal — `identical_features` is only a Warning) resolution silently chose
   first-in-authored-order: exactly the Lexurgy-issue-#9 silence the design
   calls out. Now warned once per (rule, chosen phoneme) per run, naming every
   carrier; the per-site record was already in the trace's `ambiguous_with`.
2. **A convergent mint's weight depended on lexicon order.** Two different
   sources feeding one reference row kept whichever weight arrived first — word
   order reaching the evolved genome's bytes, against the module's own promise.
   The mint now keeps the **maximum** over its sources, a function of the set.
3. **`stress_without_prosody` claimed "can never fire" falsely** on lexicons
   with hand-authored stress marks, which the engine legitimately reads. The
   check now takes the lexicon and stays silent when any syllable is marked.
4. **A stale comment in `generate.rs`** still called `features_unspecified` a
   Warning — wrong in the direction that invites un-gating the engine. Fixed.
5. (Duplicate of 1, found independently by the second reviewer.)

Deferred: **`by_ipa` compares bytes, not canonical equivalence** — an author
glyph saved in NFD can evade the mint guard and leave two identically rendered
glyphs in one inventory. Closing it needs Unicode normalization data; recorded
at the comparison site and queued as an `ipa_not_nfc` warning for M7's
plausibility profile, where validation grows anyway.

### Gotchas for the next session

- **M4's fork obligation is unchanged and now demonstrated:** `evolve` copies
  every `cognate_set` verbatim (a test walks it), and `evolve` is *not* a fork —
  it produces one descendant; a fork produces sisters. The primitives are all in
  place.
- **`stemma trace` output comes entirely from `render_derivation`** in
  `stem_soundchange::view` — the CLI contributes parsing only, so the M11 UI
  calls the same function.
- **Resolution is evaluated against the input inventory,** never the growing
  one, so `Inventory` vs `Innovated` in a trace cannot flip when words reorder.
  Mints are appended in reference-table order after the run.
- **`Derivation` lives in `stem_lexicon`, not `stem_soundchange`** — the reverse
  would be a crate cycle (`stem_soundchange` already depends on `stem_lexicon`).
  The spec caught this before implementation.
- **The reference table is append-only and injective** — four construction tests
  hold it. Adding /ʃ/ would be a bug until a stridency feature exists (it would
  be byte-identical to /s/).
- **The cognate-mint source scan now walks all of `crates/*/src/`** rather than
  four hard-coded files, so `stem_soundchange` (and every future crate) is
  covered automatically.

---

## M2 — Lexicon · built 2026-07-19 · ✓ verified

Languages now have words. `stemma new-lexicon` coins one root per concept from a
built-in 103-item list, `export-md` writes a dictionary and `export-csv` a
CLDF-shaped table. **232 tests pass**; clippy clean under `-D warnings`.

Verified by running it: `new-lexicon fixtures/proto_asterian.ron --out
out/proto.ron` produces 103 entries — `aop` "all", `nuko` "ashes", `nak` "bark",
`sa` "big" — each with a stable `WordId` and `CognateSetId`; reloading yields an
identical lexicon; the dictionary and the CSV are byte-identical across runs. The
homophone check found 5 real collisions (`a`, `ni`, `nim`, `wa`) and reported them
as a Note without making the language invalid.

### Decisions worth knowing

**Export is a new crate, not a relaxation of `stem_io`.** `DESIGN.md` §9.2 puts
`markdown.rs` inside `stem_io`, but `stem_io`'s own module docs say it is generic
over serde and must not know the domain — and a Markdown dictionary cannot be
written that way, since a word's rendered form needs the phoneme inventory. The
distinction the split encodes: **persistence is total, reversible and domain-blind;
rendering is lossy, opinionated and domain-specific.** `stem_io/src/` was not
touched by this milestone, not one line. [ADR-0006](docs/adr/0006-export-is-a-separate-crate.md).

**A `concept` field that §8.3 does not list.** §8.3 offers `glosses` (free strings
that drift) and `semantic_nodes` (M9, and `SemanticNodeId` does not exist yet).
Neither can be the cross-language join key §10.3's cognate table needs, so
`ConceptKey` was added. It is deliberately **not** the cognate set: a concept is
shared *meaning*, a cognate set is shared *ancestry*, and Latin *caput* "head" →
French *chef* "chief" have one and not the other. At M2 they are in exact bijection
and a future reader will want to delete one — a test exists to stop that.
[ADR-0007](docs/adr/0007-word-identity-and-cognate-sets.md).

**Swadesh-100 does not contain `king` or `mother`** — and ROADMAP M5's own
acceptance command is `stemma cognates --meanings water sun star king mother`. The
design panel caught this. Three concepts (`MOTHER`, `KING`, `STORM`) are appended
after the hundred, under an explicit rule: *a meaning named by a Phase-1 acceptance
test or a DESIGN worked example must be representable*. Appending rather than
interleaving is what preserves the draw contract's prefix property. Without them,
M5 could not run its own test, and reopening the concept schema at M5 is exactly
the deferred cost this milestone exists to avoid.

**Concepticon anchors were fetched, not remembered.** Each Swadesh concept carries
its Concepticon id. Two values that circulate in secondary sources are wrong and
the fetched ones are used: `hair` is 1036 (not 1040) and `root` is 668 (not 670).
`KING` and `STORM` could not be verified, so their `concepticon_id` is `None` — a
plausible-looking integer under a column bearing an external authority's name would
be a false provenance claim in a program whose premise is provenance.

**`phonemic_form` is a `Root`, never a `String`.** A rendered form stored beside
the segments is a second source of truth that desynchronises the first time M3
mutates a segment, undetectably. `written()` and `ipa()` are views.

**The lexicon draws on its own RNG stream.** `RngDomain::Lexicon`, so the Nth word
of `new-lexicon --seed 42` is deliberately *not* the Nth root of `generate-roots
--seed 42`. Sharing would freeze the lexicon builder's draw budget to the root
generator's forever. `generate-roots` is a scratchpad; `new-lexicon` is an artifact.

### Gotchas for the next session

- **`build_proto_lexicon` must not call `inventory.validate()`.** `RootGenerator::new`
  deliberately filters feature-only codes out of its gate so a half-featured file
  still generates; re-validating would make `new-lexicon` refuse a language
  `generate-roots` accepts — M1's validator/engine defect reopened one axis over.
  `a_language_that_can_generate_roots_can_seed_a_lexicon` guards it.
- **Edit distance is Damerau-Levenshtein, not plain Levenshtein, and that was a
  bug fix.** Under plain Levenshtein a transposition costs 2, so `NOES` tied with
  `NOSE`, `NEW` and `NOT` — and the deterministic tie-break returned `NEW`. Since
  transposition is the most common typing error, counting it as one edit makes the
  right word win. The suggester moved to `stem_core::suggest` so features and
  concepts share one implementation.
- **`--out` rewrites the stored seed.** `--seed 7 --out f.ron` must not write a file
  saying `seed: 42` while holding words drawn from stream 7; the genome's seed
  promises reproducibility *from the file alone*, and this is the first command
  that persists a stochastic result.
- **Two golden tiers again.** The data-free canary in `stem_export` (a hand-built
  4-phoneme genome) cannot be moved by any fixture edit; the fixture goldens and
  the `d16ba861…` lexicon digest legitimately move when fixture content changes.
  Keep them distinguishable.
- **The reference fixture is unchanged and stays lexicon-less.** It is a proto
  *definition*, the thing `new-lexicon` is run against. `skip_serializing_if` keeps
  `convert` from adding an empty `lexicon: []`, so its round trip is still
  byte-identical — a test asserts it.
- **M4's whole cognate obligation is one line:** copy `cognate_set` verbatim, never
  mint. `scoped_cognate_set` is the only minting site in the workspace and a
  source-scanning test enforces it.

---

## M1 — Feature bundles & root generation · built 2026-07-19 · ✓ verified

Phonemes now carry real distinctive features, languages declare their syllable
shapes, and `stemma generate-roots` produces reproducible root words.
**157 tests pass**; clippy is clean under `-D warnings`.

Verified by running it: `generate-roots fixtures/proto_asterian.ron --count 100`
emits roots like `kanmol`, `tatsi`, `masu`, `liwa`, `sitna`. Running it twice with
the same seed is byte-identical; a different seed differs; `--count 20` is an exact
byte-prefix of `--count 100`. `stemma features` prints each segment's resolved
matrix, and a typo in a fixture (`+voicee`) fails to load with
`19:69-19:70: unknown feature '+voicee'; did you mean 'voice'?`.

### How this was designed

M1 was specified before it was written, because the feature model is what M3's
rule engine matches against and getting its shape wrong is expensive to undo. Two
rounds of adversarial design review ran first. **The first round failed usefully:**
four proposals all tried to solve M3 inside M1 — shipping feature registries, node
geometry, ordinal scales, fixpoint resolution and hand-written parsers — and
independent judges scored every one of them 5–6/10, with one verdict reading "M1
scope is over budget against the project's own top-named risk" (§20.1). The second
round was scoped explicitly to ROADMAP M1 and scored 8/7/7.

That is the origin of the single most important decision here: **M1 is much smaller
than it first appears it should be.** No rules, no classes, no scales, no
inheritance, no DSL.

### Decisions worth knowing

**The feature set is a closed enum, not a data-declared registry.** This is
counter-intuitive — §7.2's tone genesis and §7.7's alien channels both argue for an
open namespace — and it was chosen anyway because an open registry cannot tell a
new feature from a misspelled one. `+voicee` silently becomes a real feature that
no segment carries, so every rule keyed on it matches nothing forever, with no
diagnostic. Closed, that same edit is a load error with a suggestion. Safe because
**nothing on disk refers to a feature by number** — bundles serialise as signed
names, so appending a variant cannot change what any saved file means.
[ADR-0004](docs/adr/0004-closed-feature-set.md).

**Storage is ternary; reference is binary.** A cell is `+`, `−`, or absent, where
absent means "the question does not arise" (a plain alveolar has no rounding
value). Conflating absent with minus is the single most damaging error available
here: store /f/ as `[−strident]` and the class `[−strident]` silently swells from
the dental fricatives to every non-sibilant in the language. Rules may reference
only `+` and `−`, because "the segments where this is undefined" is not an attested
natural class.

**`SegmentKind` is a phonotactic slot class, not the feature `[consonantal]`.**
This is what finally resolved the glide problem cleanly. /w/ and /j/ fill consonant
slots *and* are `[-consonantal]`; both are true, and a model that infers one from
the other has to reject an author who wrote the truth. Keeping them separate also
avoided a workspace-wide refactor of `Validate`'s signature.

**ChaCha20 + SHA-256 seed expansion, with `=` version pins.** `StdRng` is
documented as non-portable and, as of rand 0.10, may change output in a *patch*
release; `seed_from_u64` reserves the same right; and rand 0.9.0's changelog says
outright that it broke `Uniform`'s value stability — which `WeightedIndex` samples
internally. Any of the three would silently rewrite every stored language on an
unrelated `cargo update`. `default-features = false` removes the non-portable
generators from the API entirely, which is the real enforcement.
[ADR-0005](docs/adr/0005-rng-and-determinism.md).

**Weights are `u32`, not `f32`** — the one deliberate back-compat break of Phase 1.
The sampler builds a prefix-sum array in iterator order; with floats, identical
weight sets draw differently depending on summation order, and quantisation can
silently zero a small weight, leaving a phoneme in the inventory and absent from
every word. Taken now because the only affected files were fixtures this milestone
rewrote anyway. It would not be worth taking at M5.

### Evidence the model will survive M3

`crates/stem_cli/tests/reference_phonology.rs` asserts, against the real fixture,
that the design doc's own example rules are expressible today:

- `[-sonorant, -continuant, -voice]` selects exactly `{p, t, k}` (§11.1 IntervocalicVoicing)
- `[+syllabic]` selects exactly `{a, e, i, o, u}` (its `V _ V` environment)
- `[+nasal]` selects exactly `{m, n}` (nasal place assimilation)
- `[-sonorant, +dorsal]` selects `{k}` and `[+syllabic, -back]` selects `{e, i}` (§11.1 VelarPalatalization)
- /i/~/j/ and /u/~/w/ differ in exactly one feature, `syllabic`

That last one is the payoff: glide formation is a single feature flip in M3 rather
than an invention of place features out of nowhere.

### What the post-implementation review caught

M1 was reviewed by five adversarial reviewers on separate dimensions, each finding
independently reproduced by a second agent before it counted. **Three reviewers
independently found the same defect**, which is the one worth remembering:

**A vowel-only language passed `validate` with a warning and then failed to
generate.** `phonology.no_consonants` is a Warning by design — CLAUDE.md says
unusual designs are flagged, not rejected — but `RootGenerator::new` built the
consonant distribution unconditionally, so `WeightedIndex::new(vec![])` errored and
leaked a raw `rand` string. The validator and the engine disagreed about whether a
language worked, which is exactly the failure the design means to make impossible.
Fixed by preparing a slot class only if some template actually uses it.

**A one-character field typo silently changed the language.** `frequency_wieght`
was accepted, the real field took its default of 10, and on the fixture's
most-frequent vowel that rewrote **17 of 20 generated roots with no diagnostic at
any severity** — a direct hit on §9.4. `Phoneme` and `LanguageGenome` now carry
`deny_unknown_fields`. This reverses a spec decision: the spec excluded them for
forward compatibility, but the contract that actually matters ("old files keep
loading") is provided by `#[serde(default)]`, not by tolerating unknown fields.

Also fixed: generation no longer blocks on feature faults it never reads (adding
features one phoneme at a time used to break a working file halfway through);
`--count 18446744073709551615` is an argument error rather than a capacity-overflow
panic; phonotactics weight sums are overflow-checked like the inventory's;
`[+round -labial]` is flagged (a /kʷ/ authored that way would escape every
`[+labial]` rule); `Root`/`Syllable` gained `Ord`/`Hash` so M2 can dedupe without a
breaking change; the two CI determinism guards no longer **fail open** (`!` inverts
only the last command's status, so a `cargo tree` error reported success — and the
reviewer verified no frozen canary would have caught the `unbiased` feature that
guard exists for); and the nucleus property test runs 1,000 seeds rather than one.

**The golden corpus digest is unchanged across all of it** — `677f3413…` before and
after — which is the evidence that none of these fixes perturbed the draw order.

### Gotchas for the next session

- **Two bugs were caught by tests written from the spec, not by review.** (1) The
  `identical_features` check sorted its collision list but not *within* each pair,
  so reversing the inventory produced `(b, a)` instead of `(a, b)` — order-dependent
  output, the exact class of defect §9.4 forbids. (2) Nothing else; the rest
  compiled and passed first time.
- **`rand::rngs::ChaCha20Rng` exists only with `features = ["chacha"]`.** Without
  it the path is `rand_chacha::ChaCha20Rng`. Both were verified by compiling.
- **rand 0.10 renamed its traits:** `Rng` is now the core trait (`next_u64`,
  `fill_bytes`) and `RngExt` carries `.random()`. In 0.9 these were `RngCore` and
  `Rng`. Remembered code from 0.8/0.9 will not compile.
- **Three data-free canaries** (seed expansion, raw keystream, weighted-index draw
  sequence) sit alongside the corpus golden digest. No fixture edit can move the
  canaries, so a red canary means the *generator* changed while a moved digest just
  means fixture content changed. Keep that distinction — it is what stops
  re-baselining the digest from becoming a reflex.
- **`features_unspecified` is a Warning, scheduled to become an Error in M3.** It
  is a Warning now only because pre-M1 files must keep loading and M1's generator
  does not read features at all. A featureless phoneme is invisible to every rule
  M3 will write.
- **Deviation on record:** templates are concrete (`CV`, `CVC`, `V`, `VC`) rather
  than the `(C)V(C)` sugar `DESIGN.md` §15 Ticket 4 names. Parenthesis expansion
  needs an optional-slot rate nothing specifies, and its expansion order would
  silently join the RNG draw order. Rejected templates point the author at the
  explicit form. Sugar can land in M2 as a load-time rewrite.

---

## M0 — Skeleton & it runs · built 2026-07-19 · ✓ verified

Stood up the Rust workspace from the design doc: seven crates, a `stemma` CLI with
`validate` / `info` / `convert`, RON+JSON project I/O, and two reference fixtures.
**51 tests pass**; `cargo clippy --workspace --all-targets -- -D warnings` is
clean; `cargo fmt` applied.

**Verified by actually running it**, not just by the test suite: `stemma validate
fixtures/proto_asterian.ron` prints `Proto-Asterian (proto_asterian) — 15 phonemes
(10C/5V), proto, seed 42` and `✓ no issues`, exit 0. The same command against
`fixtures/invalid_no_vowels.ron` reports all three planted faults
(`phonology.bad_weight`, `phonology.duplicate_ipa`, `phonology.no_nucleus`) plus a
note, and exits 1. A RON → JSON → RON round trip through `stemma convert` returns
an identical genome.

### Decisions worth knowing

**Validation returns a graded report, not a bool.** `ValidationReport` collects
`Issue`s at Error / Warning / Note. This was chosen deliberately at M0 because
`DESIGN.md` §17 wants a *plausibility profile* — "80 consonants and 2 vowels is
possible but typologically unusual" — and a boolean validator would have forced the
engine to either reject legitimate speculative designs or say nothing useful about
them. §17 is now an extension of this report rather than a separate subsystem.
Validation also never stops at the first fault; the broken fixture proves it.

**An extra crate, `stem_genome`, that isn't in the design sketch.** §9.2 puts
`language.rs` in `stem_core`, but the genome *owns* a phonology and (later) a
lexicon and rule history, so it must depend on those crates — while they depend on
`stem_core` for IDs. That is a dependency cycle. Splitting the aggregate out keeps
`stem_core` a true foundation with no internal dependencies. Recorded in
[ADR-0002](docs/adr/0002-crate-layering.md).

**IDs are readable strings, not UUIDs.** `WordId("w_0001")`, minted from a counter,
never randomly. Traceability is the product (§3.3) and a trace full of UUIDs is
unreadable; determinism (§9.4) rules out random generation anyway. Recorded in
[ADR-0003](docs/adr/0003-readable-deterministic-ids.md).

**Two crates ship empty on purpose.** `stem_lexicon` and `stem_soundchange` have
no code, only module docs stating what belongs in them and which invariants to
preserve. Fixing the dependency edges now is cheap; rearranging them later, once
types have leaked across boundaries, is not.

### Gotchas for the next session

- **Edition 2024 match ergonomics are stricter.** `.filter(|(_, &count)| …)` over a
  `HashMap` iterator is now a hard error; it needs `.filter(|&(_, &count)| …)`.
  Cost one build cycle here.
- **RON needs `implicit_some` to read hand-authored files.** Without it, an authored
  fixture must write `romanization: Some("y")`, which defeats the point of choosing
  RON for human editing. `stem_io` enables it as a *default extension when reading*
  (so fixtures parse with or without the header) but serialises through plain
  `ron::ser::to_string_pretty` — because ron only emits the `#![enable(...)]` header
  for extensions that are *not* already defaults of the `Options` doing the
  serialising. Routing writes through the configured `Options` silently produces
  files only Stemma can read. A test now pins both halves of this.
- **`HashMap` iteration order leaks into validation output.** Duplicate detection
  sorts before reporting. Watch for this everywhere output is generated —
  determinism is a hard constraint, and this is the easiest way to break it by
  accident.
- **M1 is the load-bearing one.** The feature-bundle model has to serve M3's rule
  matching. Design it whole, against §7.1 and §11, before writing code. A fixed
  struct of `Option<bool>` fields will not survive contact with tone, phonation, or
  the alien channels of §7.7 — model it as a sparse typed bundle.
