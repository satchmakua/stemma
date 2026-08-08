# 11. Meaning is modelled exactly as form is, and drift never touches ancestry

- **Status:** Accepted
- **Date:** 2026-08-07

## Context

M9 adds `DESIGN.md` §7.5's semantics. Its acceptance is §10.2's and §21's worked
example: **`*takala` "star" becomes "omen" in Coastal while staying "star" in
Highland** — and the two reflexes stay *one cognate set*, so they keep sharing a row
in the comparative table.

That second clause is the whole milestone. A gloss changing is trivial; a gloss
changing **while ancestry does not** is the thing the comparative view exists to
show, and it is why `concept` (what a word was coined for), `cognate_set` (what it
descends from) and displayed meaning have been three separate identities since M2
(`docs/adr/0007`). M5 even shipped a test — `the_cognate_table_joins_by_cognate_set_not_by_meaning`
— that hand-built an `"omen"` daughter to prove the table would survive this.
**M9's job was to make that test's fiction real without changing its assertion.**

Four forces shape the design:

1. §3.3: a meaning that changes with no recorded history is the same bug class as a
   form that changes with no `RuleApplicationTrace`.
2. §20.1: scope explosion. Semantics invites a sense-disambiguation engine, a
   polysemy graph, a sociolinguistic subsystem, and an LLM. None of that is v0.
3. The sound-change engine must stay the source of truth and must not learn what a
   meaning is — `docs/adr/0010` bought that separation for morphology and M9 must
   not spend it.
4. §17 / `docs/adr/0009`: the semantic plausibility row must be filled the way M8
   filled the morphological one — a band agreeing with a report check through one
   shared constant, never a fabricated score.

## Decision

### Meaning is modelled exactly as form is

The correspondence is field for field, and it is the entire structural claim:

| form (M3) | meaning (M9) |
|---|---|
| `WordEntry.phonemic_form` | `WordEntry.senses: Vec<SenseRef>` |
| `WordEntry.trace: Option<Derivation>` | `WordEntry.sense_history: Option<SenseHistory>` |
| `Derivation.input`, never rewritten | `SenseHistory.input`, never rewritten |
| `steps: Vec<RuleApplication>`, deltas only | `steps: Vec<SenseShift>`, deltas only |
| `replay()` / `final_form()` | `replay()` / `final_senses()` |
| `genome.applied_rules` (a log) | `genome.applied_drifts` (a log) |
| §16.3: replay's final form ≡ the stored form | §16.3: `final_senses()` ≡ `senses` |

Nothing here is a second mechanism for a job the project already has one for. The
project *already* stores a derivable current state beside its replayable record; M9
gets the same shape and the same property test, not a new invention.

`SenseHistory` lives in `stem_lexicon` for `Derivation`'s reason: a §3.3 record
stored **on** a `WordEntry` cannot live in a crate above `stem_lexicon` without a
cycle. **No new crate** — the same refusal ADR-0010 made for `stem_morphology`.

### `SenseRef` echoes its gloss, so `display_gloss` stays context-free

`stem_soundchange::render_derivation` calls `WordEntry::display_gloss()`. A
context-taking `display_gloss(&SemanticSpace)` would make the **sound-change engine
name a semantic type**, spending ADR-0010's central achievement. So a `SenseRef`
carries `{node, gloss}` — the `MorphemeRef` precedent ("echoed… so the ref is
self-contained"), improved on by pairing the echo with a `stale_sense_gloss`
Warning. `apply_drift` is the only write site in the workspace.

`display_gloss` gains **one prepended tier**: sense → authored override → concept
gloss. A daughter inherits `glosses` verbatim through `fork`, so if the override won,
drift would be invisible. It **shadows, never overwrites**: drift does not touch
`glosses`, so `w_0005`'s authored `"king"` survives untouched and returns the moment
its sense is removed. An engine that clobbers an authored field is indistinguishable
afterwards from the author.

### Drift targets one word, and is a set delta

Semantic change is **lexically idiosyncratic** — it has no Neogrammarian regularity,
so there is no semantic analogue of "voiceless stops voice between vowels". A
`DriftEvent` names a `WordId`. Matching by sense instead would drift every word
holding `sn_star` at once, asserting a regularity semantics does not have.

The primitive is `remove` / `add` **set deltas**, not `from`/`to`: `senses` is a
`Vec`, a delta is the honest primitive over it, it covers all ten of §7.5's
mechanisms with one shape (taboo replacement removes and adds nothing; the Coastal
metonymy adds two senses at once), and it makes `SenseShift {removed, added}` the
exact analogue of `SiteTrace {before, after}`. The **effective** delta is recorded,
not the declared one — an event naming a sense the word never held removes nothing,
and the record must say what happened rather than what was asked.

### Two application verbs; `evolve` is untouched

- `with_drift` applies a set **within the stage** — same id, parent, depth, forms,
  rules. This is what `grow_family` uses, so a drifted branch stays **one node and
  one column** rather than sprouting a phantom language in the family tree.
- `drift` is literally `fork(...).with_drift(...)` — it mints a stage, which is what
  the CLI verb needs because a `--out` file sharing its input's `LanguageId` is the
  `duplicate_language_id` Error (`docs/adr/0003`).

`evolve`'s signature is unchanged, so `apply_rules` stays a pure RNG-free function of
five arguments — the strongest determinism claim in the project. `fork` and `evolve`
each gained two lines carrying `semantics` and `applied_drifts`. A drifted word
survives further sound change **for free**, because `apply.rs` builds each output
entry with `entry.clone()` — exactly the free ride M8's `morphemes` got.

### The engine never learns what a meaning is

`stem_soundchange` gains a second source-scan guard,
`the_engine_never_references_semantics`, banning `Semantic*`, `Sense*`, `Drift*`,
`apply_drift` and `sense_chains` across its sources. The `senses` / `sense_history`
*fields* stay legal — the engine carries them through a clone, exactly as it carries
`morphemes`. What is banned is naming a semantic **type or operation**.

### The table shows the drift — a projection, never a second join

`CognateRow.cells` becomes `Vec<Option<CognateCell>>`, where a cell carries the form,
the column's own `display_gloss()`, and a `drifted` flag set when that gloss differs
from the reference column's. The gloss is read off the entry `by_cognate_set`
**already returned** — zero extra lookups. The standing rule that the table never
re-resolves a meaning per column is satisfied, because a projection of the
already-joined cell is not a resolution.

Renderers annotate **only when drifted**, so a family whose meanings have not
diverged renders byte-identically to its pre-M9 output. That is what let the M5
`cognates` byte pin and the `cognate_table` markdown canary survive untouched.

### The band measures distance, and says so

`NotModelled::SemanticPlausibility` **leaves** the deferred list, as M8's
morphological row did. The band is deliberately named **`SemanticDrift`**, not
"semantic plausibility": judging whether `star → omen` is a *plausible pathway* needs
a typology of attested shifts this project does not have and §20.1 fences out, and
emitting a number for it would be the fabrication ADR-0009 forbids. It measures the
longest **recorded** chain — what the engine can actually see — and the renderer says
so.

`HighlyDrifted` and the `long_semantic_drift_chain` Note both read the one
`LONG_SENSE_CHAIN = 3`, so they agree by construction; a projection test pins it.
The Note is never an Error. The demo's own two-step Coastal chain scores `Drifted`
and correctly stays below the bar — M8's rule that the tool's showcase must not trip
its own warning.

**A trap M9 had to fix:** `render_profile`'s test asserted `contains("M9")`. Once the
semantic row was filled, that assertion would have kept passing on
`ScriptHistoryCoherence`'s `"M9+"` string — silently dishonest. The two remaining
deferred dimensions now name design *sections* (`§7.6`, `§18`), since neither sits at
a numbered milestone, and the assertion is inverted to `!contains("M9")`.

### The anti-fabrication fence was rewritten, not retired

M6 banned the demo from printing `omen` because the engine could not produce it.
M9 built that capability, so the ban became a **condition**, and the replacement is
strictly stronger than the original:

- `tazal` and `night-signal` stay banned forever — `ɡ → z` is a place shift
  `Change::Set` cannot express, and `night-signal` is an invented gloss.
- `omen` may appear only when a real `DriftEvent` produced it, the mechanism is
  named, the register is shown, and the closer no longer promises meaning drift as
  future work.
- A **new general guard** replaces the hard-coded string list with the rule it stood
  for: every gloss the demo prints in a gloss position must be a built-in concept's
  gloss or a semantic node declared by a language in the rendered family. A future
  session that invents a pretty gloss fails here even though its string was never on
  any list.

### Deferred, deliberately

§8.5's `HistoricalEvent` union **stays deferred**. Migrating `applied_rules` into it
would renumber every `RuleApplication.index` in every stored derivation — a format
break the project forbids. §10.4's unified timeline is a *derived* merge of two
ordered logs by `chronology_years`: computable, and therefore never stored
(`docs/adr/0008`). `EventId`, declared at M0 and unused ever since, finally gets its
first producer in `DriftEvent.id`.

## Consequences

- **Every pre-M9 file loads and round-trips with zero new bytes.** Four additive
  fields, each `#[serde(default, skip_serializing_if = …)]`. A test asserts the
  reference proto serialises with no `semantics`, `applied_drifts`, `senses` or
  `sense_history`. `reference_phonology.rs`'s `["lexicon.empty"]` pin is exact —
  every new check is gated on non-empty senses/history/space.
- **The mechanical cost was paid, as M8 paid it:** two new fields broke 23
  `WordEntry` struct literals across 12 files. No builder was introduced; the same
  call M8 made.
- **`asterian_attested.ron` gained a `semantics:` block and one `senses:` field, and
  moved no rendered byte** — `sn_star`'s gloss is `"star"`, identical to what concept
  STAR already displayed. It exists so the drift has a real sense to *remove* and so
  `SenseHistory.input` is literally true rather than inferred.
- **`golden/family_demo.md` re-baselined once, deliberately**, after the `src`
  canaries were confirmed green. The diff is exactly five intended changes and
  nothing else.
- **`DriftMechanism` is descriptive, never a switch** — no code branches on it to
  compute a value; it reaches the renderer and the plausibility basis and nothing
  else, the `WordSource` discipline.
- **`register` is a free-text label**, not an enum: register is a fact about one
  culture, not a typological universal, and a closed enum would render "ritual" where
  §10.2 says "priestly". `WordEntry.register` (§8.3) stays deferred — still no
  producer.

## Fenced out of v0 (§20.1)

No LLM-invented or probabilistic drift — v0 **applies** authored drift exactly as M3
applies authored rules, and never *invents* one. No automatic drift simulation; no
syntax, proposition format or interlingua (§7.4); no script or glyph history (§7.6);
no sense-disambiguation engine; no polysemy or hypernym graph over nodes (a chain is
recovered per-word by replay, exactly as intermediate forms are); no register or
sociolinguistic subsystem beyond the label; no taboo-replacement *mechanics* (the
enum names the pathway; replacing a word's form is a later lexical operation); no
semantic borrowing or calquing; no `HistoricalEvent` union and no unified §10.4
timeline; no semantic column in the CLDF CSV (a future `senses.csv` is the
standard-shaped answer); no compiled typology of attested pathways and therefore **no
judgement about whether a given drift is plausible**; no `SemanticNodeId` minting
anywhere — every node is authored, as every rule is; and no new crate.
