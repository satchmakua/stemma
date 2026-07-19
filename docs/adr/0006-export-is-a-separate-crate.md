# 6. Rendering lives in `stem_export`, not in `stem_io`

- **Status:** Accepted
- **Date:** 2026-07-19

## Context

M2 adds Markdown and CSV export. `DESIGN.md` §9.2 puts `markdown.rs` and `csv.rs`
inside `stem_io`, alongside `ron.rs` and `json.rs` — they all write files, so one
crate looks right.

That layout contradicts something `stem_io` says about itself. Its module docs
read: *"Everything here is generic over serde. This crate deliberately does not
depend on `stem_genome`: persistence should not know the shape of the domain, only
how to move it across the filesystem boundary."* Its manifest backs the claim —
`stem_genome` appears only as a dev-dependency. Every function it exposes is
`fn(&impl Serialize)` or `fn() -> impl DeserializeOwned`.

A Markdown dictionary cannot be written that way. It has to know that a gloss
column comes before a part of speech, that a cognate set is worth showing, and that
a word's *rendered form cannot even be computed without the language's phoneme
inventory* — `WordEntry` stores `PhonemeId`s, so `written()` takes an inventory.
Putting that in `stem_io` would delete the crate's stated reason to exist.

Four options were weighed:

1. **Relax `stem_io`'s principle** and let it know the domain. Rejected: the
   principle is load-bearing, not decorative. It is what lets `stem_io` serve M4's
   lineage graph and M9's semantic graph without being touched.
2. **A `Render` trait in `stem_core`.** Rejected: `stem_core` depends on nothing
   internal (ADR-0002), and a trait whose only implementors need a phoneme
   inventory would drag the domain in through the type parameter anyway.
3. **Rendering methods on the domain types.** Rejected: it scatters presentation
   across four crates and gives `stem_lexicon` an opinion about Markdown.
4. **A new crate above the domain.**

## Decision

Option 4. `stem_export` sits above `stem_genome`, depends on the domain crates,
and is depended on only by `stem_cli`:

```
stem_core → stem_phonology → stem_lexicon → stem_soundchange → stem_genome ⇒ stem_export → stem_cli
                                                                    ↳ stem_io ↗
```

**`stem_io/src/` is not touched by M2. Not one line.**

The distinction the split encodes: **persistence is a total, reversible,
domain-blind mapping; rendering is a lossy, opinionated, domain-specific
projection.** A round trip through `stem_io` must return exactly what went in. A
Markdown dictionary deliberately drops the syllabification, the frequency weights
and the feature bundles, and adds prose that exists nowhere in the model. They both
end in a file, and that is the only thing they share.

Renderers append to a `&mut String` via `std::fmt::Write`, never `std::io::Write`.
Composition demands it — at M6 a family document appends these as sub-renderers
into one buffer, and at M11 the UI renders without touching the filesystem. So does
correctness: `StemmaError::Io` mandates a `path`, which a renderer writing into a
buffer does not have, so an `io::Write` signature would need either a new
`stem_core` error variant (in the crate ADR-0002 protects) or a fabricated
`path: "<output>"` — a lie in a user-facing message.

## Consequences

- `stem_io` keeps its principle and its independence from the domain. A future
  format (LaTeX, HTML, CLDF with metadata) is a module in `stem_export`, not a
  compromise in `stem_io`.
- Exports are unit-testable without the filesystem: a renderer's whole contract is
  `String` in, `String` out, which is why the byte-exact canary tests are possible
  at all.
- One more crate than §9.2 draws. The same deviation-with-a-reason as ADR-0002, and
  the crate names §9.2 uses are otherwise unchanged.
- `stem_export` dev-depends on `stem_io`, because the golden tests load the
  reference fixture. That is not a cycle — the library never does.
- `stem_io`'s manifest description drops "and exports", since it no longer has any.

### The rules to preserve

**Nothing in `stem_export` may sort, use a map, use a float, or read a clock.**
`DESIGN.md` §9.4 requires exported bytes to be a pure function of the genome, and
the cheapest way to guarantee a stable sort is not to sort — row order is lexicon
order, which is concept-list order, which is frozen. A test reads the renderer
sources and fails on `HashMap`, `f32`, `f64`, or `.sort`.

**Export may fail only where `validate` reports an Error.** A document that refuses
to render over something the validator merely warned about is the validator and the
engine disagreeing, which is the defect M1's review caught in the generator.
