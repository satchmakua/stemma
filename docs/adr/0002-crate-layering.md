# 2. The genome aggregate lives outside `stem_core`

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

`DESIGN.md` §9.2 sketches the crate layout, placing `language.rs` and `lineage.rs`
inside `stem_core` alongside `ids.rs` and `errors.rs`.

That layout cannot compile. `LanguageGenome` (§8.1) *owns* a `Phonology`, a
`Lexicon`, a `MorphologyProfile`, and a history of `SoundChangeRule`s, so it must
depend on `stem_phonology`, `stem_lexicon`, and `stem_soundchange`. Those crates in
turn need `PhonemeId`, `WordId`, `StemmaError`, and the `Validate` trait — which
live in `stem_core`. Putting the aggregate in `stem_core` therefore makes the crate
graph cyclic, and cargo rejects cyclic crate dependencies.

Three ways out were considered:

1. **Each domain crate defines its own ID and error types.** Idiomatic Rust, but it
   scatters the ID vocabulary across six crates and makes `EntityRef`-style
   cross-references (§8.5) awkward, since no single crate can name every ID type.
2. **Collapse the MVP into one crate**, which §9.2 explicitly permits. Simplest
   today, but the module boundaries are the part of the design worth protecting,
   and re-splitting after types have leaked across them is expensive.
3. **Split the aggregate into its own crate above the domain crates.**

## Decision

Option 3. `stem_core` is the *foundation* — typed IDs, `StemmaError`, and the
`Validate` trait — and depends on nothing internal. The `LanguageGenome` aggregate,
and later the `LanguageLineageGraph`, live in a new crate **`stem_genome`** that
sits above the domain crates:

```
stem_core → stem_phonology → stem_lexicon → stem_soundchange → stem_genome → stem_io → stem_cli
```

This is a deviation from the design sketch in *file placement only*; the module
boundaries §9.2 asks for are preserved exactly.

`stem_io` sits above `stem_genome` but is generic over serde and does not depend on
it in the library path — persistence should not know the shape of the domain.

## Consequences

- The crate graph is a DAG, which is a hard requirement rather than a preference.
- `stem_core` stays small and stable. Because everything depends on it, a change
  there recompiles the world; keeping it to three concerns makes that rare.
- One crate more than the design sketch lists. The sketch's crate names are
  otherwise unchanged, so §9.2 remains a usable map.
- **The rule to preserve:** `stem_core` must never gain an internal dependency. If
  something in it needs to know about phonology, it belongs in `stem_genome`.
