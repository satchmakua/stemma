# 3. Entity IDs are readable strings minted from counters

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

`DESIGN.md` §15 (Ticket 2) leaves the choice open: "newtype wrappers around UUID or
strings." Every word, phoneme, rule, cognate set, and historical event needs a
stable identity that survives forking, serialisation, and export.

Two forces push hard on this choice:

- **Traceability is the product** (§3.3, §10.2). Users read traces. They also read
  cognate tables, Markdown exports, and — because a language is a plain RON file —
  the project files themselves.
- **Determinism is a hard constraint** (§9.4). Re-running a pipeline with the same
  seed must reproduce the same language byte for byte.

UUIDv4 fails the second outright: random IDs mean two runs of the same pipeline
produce files that differ everywhere. It also fails the first in practice — a trace
reading `w_9f3e… > w_1c7b… (rule r_44a2…)` tells a user nothing.

## Decision

Every ID is a newtype over `String`, serialised `#[serde(transparent)]` so it
appears as a bare string in RON and JSON. IDs are minted from a sequence counter
(`WordId::sequential(1)` → `w_0001`) or authored by hand in fixtures
(`"proto_asterian"`, `"ph_t"`). **No ID is ever randomly generated.**

The newtypes are distinct types, so a `PhonemeId` cannot be passed where a `WordId`
is expected — the compiler enforces what a bare `String` would not.

## Consequences

- Traces, exports, cognate tables, and project files are readable and diffable. A
  fork diff shows what actually changed rather than a wall of new identifiers.
- Reproducibility comes for free: the same pipeline yields the same IDs.
- **Uniqueness is not automatic.** Sequential IDs collide if two generators share a
  counter space, and hand-authored IDs can be duplicated. This is why duplicate-ID
  detection is an *error* in every `Validate` implementation, and must remain one.
- IDs are heavier than a `u64` or a UUID (a `String` each, cloned on fork). At the
  scale of a lexicon — thousands of words, not millions — this is irrelevant. If
  profiling ever says otherwise, interning is the fix; abandoning legibility is not.
- **The rule to preserve:** if you ever need an ID and there is no counter to hand,
  that is a signal the calling code has lost track of its generation context — not
  a licence to reach for randomness.
