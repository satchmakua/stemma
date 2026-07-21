# 8. Lineage is derived from `parent`, never stored

- **Status:** Accepted
- **Date:** 2026-07-21

## Context

M4 adds forking and lineage. `DESIGN.md` §8.6 sketches the family as
`LanguageLineageGraph { nodes: HashMap<LanguageId, LanguageGenome>, edges:
Vec<LineageEdge> }`, with a six-variant `LineageEdgeKind` (Descent, DialectSplit,
ContactInfluence, Creolization, Standardization, ScriptBorrowing). Ticket 8 asks
for `fork_language(parent, child_name)` that "preserves cognate IDs and records
parent ID". Everything the milestone needs to *persist* — `parent`,
`lineage_depth_years`, `applied_rules`, per-word `Derivation`s whose `input` is
the proto-form, verbatim `cognate_set`s — was already on the genome by M3.

Three questions had to be settled before writing any of it.

## Decision

### The lineage graph is assembled in memory; nothing about it is persisted

`stem_genome::LineageGraph` holds a `Vec<LanguageGenome>` and derives every edge
from the `parent` field on demand. There is no stored edge list.

A stored edge beside the `parent` fields is a second copy of one fact that
nothing keeps synchronised — the defect class this project has rejected three
times: `form` beside `phonemic_form` (`docs/adr/0007`), a stored intermediate
form beside a replayable trace (M3), `Syllable::pattern` read as semantics
(`stem_phonology`). Desync is prevented by making the second copy not exist. The
CLI takes explicit file paths (`stemma family a.ron b.ron …`); argv cannot lie
about which files were loaded, and files remain the project format (§19.2), so no
family manifest is introduced.

### No map, ever, in the graph

A `HashMap` would leak iteration order toward output — the determinism rule
(§9.4) — and silently swallow the duplicate ids `docs/adr/0003` requires the
validator to *see*. Nodes stay a `Vec` in the order the caller gave (argv order
at the CLI: the authored-order rule of M1, applied to files), and every lookup is
a linear scan. A family is tens of nodes.

Rejected alternatives: an **id-sorted canonical order** violates the
authored-order convention and creates a duplicate-id tie-break ambiguity; a
**`BTreeMap` index** is unnecessary at this scale. Walk guards (`ancestry`,
cycle detection, descendant closure) key on **node index, not id**, so a walk
through a duplicate-id or cyclic family terminates and is *reported* rather than
looping or ending early.

### No `LineageEdgeKind` — not even `Descent`

With edges derived rather than persisted, the file-format-stability argument for
shipping all six variants evaporates: adding a kind later is a pure code change,
touching no saved file. A one-variant `Descent` enum is scaffolding by the
`HistoricalEvent` precedent (`stem_genome::language`). A dialect split *is*
descent; "split" is topology — out-degree > 1 — and derivable, so nothing stores
it. The four contact-like kinds are not even derivable from `parent`: a contact
edge is a *second* parent. They arrive with their producers (M7+) as additive
`#[serde(default)]` genome fields, the same way every other deferral in this
project lands.

## Consequences

### `fork` is identity-plus-split; `evolve` still owns rule application

`LanguageGenome::fork(id, name, elapsed_years)` clones the genome verbatim under
a new identity with `parent: Some(self.id)`. It runs no rules, no RNG, and
changes no form. The cognate obligation is discharged **by construction** — the
lexicon is cloned whole, so every `cognate_set` is byte-identical and no code
path can mint. The CLI's `fork` verb calls `evolve` when given `--rules`, through
the *same* load→evolve→gate→write helper `apply-rules` uses, so the write gate
cannot drift between the two. The verbs differ because the user's intent differs
(a sister branched off vs the language advanced a stage); the file records no
verb, so at file level they are one shape.

Rejected: an N-ary atomic fork (clap chunking, an unusable N-TSV stdout mode,
and it coordinates nothing — no RNG, verbatim ids); a library `fork` that takes a
rule set (byte-identical to `evolve`); and an **in-place `advance`** (`apply-rules
x.ron --out x.ron`) — that is not idempotent, so re-running one line applies a
stratum twice, undetectably, which is exactly the double-application hazard
`applied_rules`' past-tense contract exists to make unrepresentable. Every stage
is a new file with a new id.

### Seed is copied verbatim; sisters therefore share a seed

Matching `evolve`. The seed reproduces the inherited lexicon, so a daughter file
stays reproducible from itself alone. A `derive_seed(parent_seed, daughter_id)`
scheme was rejected: `evolve` can already produce seed-sharing sisters (two
`evolve` calls on one parent), so a "branches re-key" invariant cannot hold
topologically, and no RNG runs at a fork, so the identical-innovation hazard has
no producer yet. **Documented consequence:** two sisters running `generate-roots`
draw identical roots. Revisit with a per-language RNG stream (`RngDomain` +
language id) the day a daughter-side stochastic step exists.

### `chronology_years` is absolute time from the lineage root

This is already the codebase's commitment: M3 dates its last rule 480 and passes
`--years 480`, and `chronology_not_monotonic` warns over the *whole concatenated*
`applied_rules` log. A stratum dated relative-to-split would warn on every
multi-stratum genome forever. So a stratum on a daughter of a deep parent must be
dated after the parent's last rule, and by convention `--years N` equals the
stratum's last `chronology_years`. Family edges report `elapsed_years()` as the
depth *delta* (child minus parent), never the child's total; a negative delta is
`family.depth_regression` (Error).

### `WordEntry.ancestor` stays unshipped, and is discharged not deferred

Both `fork` and `evolve` copy word ids verbatim, and no M4 mechanism coins,
borrows, or derives a word — so a daughter word's ancestor is *always* the
same-id entry in the parent. A stored field could only hold a tautology: the
`form`-beside-`phonemic_form` class `docs/adr/0007` bans. The sketched type
(`WordId`) also cannot express the field's real future uses — a loanword's source
is in another language (`(LanguageId, WordId)`), a suppletion has no ancestor.
This amends ADR-0007's deferral table: `ancestor` ships as an additive
`#[serde(default)]` field the day a producer writes a **non-identity** value (M7
borrowing, M8 derivation). The invariant is defended today by a library test and
by the cross-file `family.word_id_orphan` Warning, which catches hand-edited
daughter files whose word ids do not resolve in the parent.

### The cognate-mint scan is per-crate, and CLAUDE.md's phrasing is corrected

CLAUDE.md called `scoped_cognate_set` "the only minting site in the workspace"
and implied one workspace-wide scan enforces it. It does not: the `stem_lexicon`
scan uses `include_str!`, which cannot cross a crate boundary, so it never saw
`stem_genome` (where all M4 code lands) and already missed `stem_lexicon`'s own
`trace.rs`. M4 closes the gap: `trace.rs` joins the `stem_lexicon` list, and
`stem_genome` gains its own `stem_genome_never_mints_a_cognate_set` that
enumerates `src/*.rs` at **runtime** (via `CARGO_MANIFEST_DIR`, no hand-list to
go stale) and scans each file's non-test region. The rule is unchanged —
`scoped_cognate_set` is the only mint site — but its enforcement is now honestly
per-crate, one scan per crate that could mint.

### Family validation reports; it does not police

`family.duplicate_language_id`, `family.parent_cycle`, and
`family.depth_regression` are **Errors** — each makes graph walks, joins, or
timelines meaningless (structural). Everything else is softer:
`family.dangling_parent` (inspecting a subtree is legitimate),
`family.cognate_set_missing` (word death is real history),
`family.cognate_set_unrooted` (legal innovation, but the shape a re-mint takes —
so both sides are named), and `family.word_id_orphan` are Warnings;
`family.no_divergence` (freshly forked, at any parent depth — the family-level
complement to the genome's `no_elapsed_time`) and `family.multiple_roots` are
Notes. Do not promote these to Errors to make M5's table renderer's life easier.

### Naming deviations

`DESIGN.md`/ROADMAP say `LanguageLineageGraph` and `fork_language`; inside
`stem_genome` they are `LineageGraph` (no stutter) and the method
`LanguageGenome::fork` (matching the `evolve` precedent). `render_family`'s text
is snapshot-pinned, because M6's demo scripts against exactly it.
