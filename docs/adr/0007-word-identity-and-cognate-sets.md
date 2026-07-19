# 7. Word identity: concepts, cognate sets, and what M2 defers

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

`DESIGN.md` §8.3 gives `WordEntry` fourteen fields. Most belong to milestones that
do not exist yet: `semantic_nodes` needs M9's semantic graph, `morphemes` needs
M8's morphology, `trace` needs M3's rule engine, `ancestor` needs M4's fork.

Two questions had to be settled before any of them:

1. **What identifies "the same meaning" across two languages?** §10.3's cognate
   table is a grid of meanings by languages. §8.3 offers `glosses` — free strings,
   presentational, and they drift — and `semantic_nodes`, which do not exist.
   Neither can be the join key.
2. **What identifies "the same word, descended"?** §8.6 makes cognate-set survival
   across forking a hard constraint, and `CLAUDE.md` says breaking it "is not a
   local bug".

## Decision

### `concept` is added; it is not in §8.3, and it is not a cognate set

A `ConceptKey` names a language-neutral meaning from a compiled-in list. It is the
join key for the comparative view.

It is **not** the cognate set, and the two must never be collapsed. A concept is
shared *meaning*; a cognate set is shared *ancestry*. Latin *caput* "head" and
French *chef* "chief" share a cognate set and not a concept. CLDF keeps
`Parameter_ID` and `Cognateset_ID` in separate columns for exactly this reason.

At M2 they are in exact bijection, which is precisely why this is written down: a
future reader will see two fields carrying the same information and want to delete
one. They diverge the moment M4 lands and again the moment synonyms do. A test,
`a_cognate_set_and_a_concept_are_independent_identities`, builds an entry whose
concept and cognate ordinal disagree and asserts it round-trips.

### The cognate-set invariant

> For any two entries `a` and `b` in any two lexicons of one family,
> `a.cognate_set == b.cognate_set` **if and only if** `a` and `b` descend by
> unbroken inheritance from one and the same entry of that family's proto-lexicon.

Three corollaries, all checkable today:

1. **Never a function of the form.** Forms change under every rule; a hashed form
   would reassign cognacy at each one. `*takala` → Coastal `taal` and Highland
   `tazal` must stay one set *precisely because* the forms diverged.
2. **Never a bare per-language counter.** A daughter's counter restarts at 1 and
   would reuse its parent's strings for different words.
3. **Never derived from the concept.** Two synonyms share a concept and must be
   different sets; and after M9's drift, a set named `cog_x_NOSE` whose reflex
   means "beak" is actively false.

The id is minted as `cog_{language}_{ordinal:04}`. The language scope is there
because nothing coordinates minting between two independently authored
proto-languages — without it, Proto-Asterian's `cog_0001` and Proto-Kelvish's
`cog_0001` are the same string, and a table over both would silently assert they
are related. The ordinal is pure digits, so the id decodes by splitting at the
**last** underscore whatever the language id contains; the scope is therefore not
sanitised, since sanitising is lossy and could collide two languages onto one
prefix.

**`build::scoped_cognate_set` is the only minting site in the workspace, and a test
asserts it by scanning the crate sources.** What M4's fork must do is copy
`cognate_set` verbatim and never mint. That is the entire M4 obligation, and it is
one line of the fork.

### What M2 ships and defers

| §8.3 field | M2 | Reason |
|---|---|---|
| `id`, `phonemic_form`, `glosses`, `part_of_speech`, `cognate_set` | **ship** | ROADMAP M2 names each. |
| `source` | **ship** | Two producers exist at M2. |
| `concept` | **added** | The join key §10.3 needs and §8.3 lacks. |
| `form` | **never** | A rendered string beside `phonemic_form` is a second source of truth that desynchronises the first time M3 mutates a segment, undetectably. It is a view: `written()` / `ipa()`. |
| `ancestor` | M4 | No producer until forking exists. |
| `trace` | M3 | Including the container: an empty `EvolutionTrace` on every entry is scaffolding. |
| `semantic_nodes` | M9 | `SemanticNodeId` does not exist and must not be invented here. |
| `morphemes` | M8 | Same. |
| `register`, `frequency`, `usage_notes` | later | No consumer. If `frequency` ever ships it ships as an integer, per `CLAUDE.md`'s rule and `Phoneme::frequency_weight`'s precedent. |

**`phonemic_form` is a `Root`, not a `String`.** §3.3's traceability constraint
requires a form to be walkable back to its segments, and M3's rules match feature
bundles resolved from `PhonemeId`s. The syllabification is carried rather than
recomputed because M3's "final unstressed vowel loss" is defined over the last
syllable, and a flat segment vector destroys that boundary irrecoverably — the
template was a random choice, not a derivable property of the output.

## Consequences

- Every deferral is additive. Nothing on disk refers to a `WordEntry` field by
  position and nothing orders by field, so appending one behind `#[serde(default)]`
  cannot change what an M2 file means.
- `WordEntry` carries `deny_unknown_fields`, for the reason `LanguageGenome` does:
  a misspelled `cognate_st:` must be a load error, not a silently defaulted empty
  string that quietly severs a word from its family.
- The concept list is compiled in rather than read from a sidecar file. A file the
  seed contract does not cover would make `new-lexicon --seed 42` produce different
  languages on two machines — breaking the promise that a result is reproducible
  from the project file alone. Same hazard that made `Phonotactics::default()`
  empty.
- **But an unknown concept key is a Warning, not an Error** — and this is where
  ADR-0004's argument stops transferring. A feature is load-bearing on the engine:
  a rule keyed on an unknown feature matches nothing, silently. A concept is inert;
  nothing in M2, M3 or M4 computes over its meaning. So the list is closed for the
  *typo* argument only, and a conlanger who writes `concept: "OBSIDIAN"` with a
  gloss gets a working dictionary and a note.
